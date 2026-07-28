use super::{
    file_metadata::compute_asset_quick_hash,
    join_safe_relative_path,
    mutations::{insert_activity_tx, mutation_response, AssetMutationResponse},
    normalize_readable_storage_file_relative_path, storage_root_write_base_path,
};
use crate::{
    auth::AuthUser,
    error::{AppError, AppResult},
    routes::access::ensure_library_asset_mutation_access,
    state::AppState,
};
use axum::{
    extract::{Path, State},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::Row;
use std::{collections::HashSet, fs, path::Path as StdPath, path::PathBuf};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdateImageSequenceFrameNumbersRequest {
    pub start_frame: i64,
    pub use_padding: bool,
    pub padding: Option<u32>,
    pub keep_start_frame: Option<i64>,
    pub keep_end_frame: Option<i64>,
}

#[derive(Clone)]
struct SequenceFrame {
    frame: i64,
    file_name: String,
}

struct SequenceMove {
    old_path: PathBuf,
    temporary_path: PathBuf,
    target_path: PathBuf,
    next_frame: i64,
    target_file_name: String,
    retained: bool,
}

pub async fn update_image_sequence_frame_numbers(
    State(state): State<AppState>,
    user: AuthUser,
    Path((library_id, asset_id)): Path<(String, String)>,
    Json(request): Json<UpdateImageSequenceFrameNumbersRequest>,
) -> AppResult<Json<AssetMutationResponse>> {
    validate_request(&request)?;

    let row = sqlx::query(
        "SELECT storage_root_id, metadata FROM assets WHERE library_id = $1 AND id = $2 AND deleted_at IS NULL",
    )
    .bind(&library_id)
    .bind(&asset_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::NotFound("asset not found".to_string()))?;
    ensure_library_asset_mutation_access(
        &state,
        &user,
        &library_id,
        std::slice::from_ref(&asset_id),
    )
    .await?;
    let storage_root_id: Option<Uuid> = row.try_get("storage_root_id")?;
    let mut metadata: Value = row.try_get("metadata")?;
    let storage_root_id = storage_root_id.ok_or_else(|| {
        AppError::BadRequest("asset does not have an enabled workspace".to_string())
    })?;
    let sequence = metadata
        .get("sequence")
        .and_then(Value::as_object)
        .ok_or_else(|| AppError::BadRequest("asset is not an image sequence".to_string()))?;
    let prefix = sequence_string(sequence, "prefix").unwrap_or_default();
    let suffix = sequence_string(sequence, "suffix").unwrap_or_default();
    let extension = sequence_string(sequence, "extension")
        .ok_or_else(|| AppError::BadRequest("image sequence extension is missing".to_string()))?;
    let stored_directory = sequence_string(sequence, "storedDirectory").ok_or_else(|| {
        AppError::BadRequest("image sequence storage folder is missing".to_string())
    })?;
    let stored_directory = normalize_readable_storage_file_relative_path(&stored_directory)?;
    let frames = sequence_frames(sequence)?;
    let retained_frames = frames
        .iter()
        .filter(|frame| {
            request
                .keep_start_frame
                .map_or(true, |start| frame.frame >= start)
                && request
                    .keep_end_frame
                    .map_or(true, |end| frame.frame <= end)
        })
        .cloned()
        .collect::<Vec<_>>();
    if retained_frames.is_empty() {
        return Err(AppError::BadRequest(
            "sequence keep range would remove every frame".to_string(),
        ));
    }

    let current_padding = sequence
        .get("padding")
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
        .unwrap_or(1)
        .clamp(1, 12);
    let padding = if request.use_padding {
        request
            .padding
            .and_then(|value| usize::try_from(value).ok())
            .unwrap_or(current_padding)
            .clamp(1, 12)
    } else {
        0
    };
    let selected_frame = sequence
        .get("selectedFrame")
        .and_then(Value::as_i64)
        .unwrap_or(retained_frames[0].frame);
    let selected_index = retained_frames
        .iter()
        .position(|frame| frame.frame == selected_frame)
        .or_else(|| {
            retained_frames
                .iter()
                .enumerate()
                .min_by_key(|(_, frame)| (frame.frame - selected_frame).abs())
                .map(|(index, _)| index)
        })
        .unwrap_or(0);

    let base_path =
        storage_root_write_base_path(&state, storage_root_id, Some(&library_id)).await?;
    let sequence_dir = join_safe_relative_path(&base_path, &stored_directory);
    if !sequence_dir.is_dir() {
        return Err(AppError::NotFound(
            "image sequence storage folder was not found".to_string(),
        ));
    }
    let moves = build_moves(
        &sequence_dir,
        &frames,
        &retained_frames,
        request.start_frame,
        &prefix,
        &suffix,
        &extension,
        padding,
    )?;
    if moves_are_unchanged(&moves) {
        return Ok(Json(
            mutation_response(&state, &library_id, user.id, vec![asset_id]).await?,
        ));
    }

    stage_moves(&moves)?;
    if let Err(error) = finish_moves(&moves) {
        rollback_moves(&moves);
        return Err(error);
    }

    let update_result = persist_sequence_update(
        &state,
        &library_id,
        &asset_id,
        user.id,
        &mut metadata,
        &moves,
        selected_index,
        &stored_directory,
        &prefix,
        &suffix,
        &extension,
        padding,
    )
    .await;
    if let Err(error) = update_result {
        rollback_moves(&moves);
        return Err(error);
    }

    for move_item in moves.iter().filter(|move_item| !move_item.retained) {
        let _ = fs::remove_file(&move_item.temporary_path);
    }
    Ok(Json(
        mutation_response(&state, &library_id, user.id, vec![asset_id]).await?,
    ))
}

#[allow(clippy::too_many_arguments)]
async fn persist_sequence_update(
    state: &AppState,
    library_id: &str,
    asset_id: &str,
    user_id: Uuid,
    metadata: &mut Value,
    moves: &[SequenceMove],
    selected_index: usize,
    stored_directory: &str,
    prefix: &str,
    suffix: &str,
    extension: &str,
    padding: usize,
) -> AppResult<()> {
    let retained_moves = moves
        .iter()
        .filter(|move_item| move_item.retained)
        .collect::<Vec<_>>();
    let selected_move = retained_moves
        .get(selected_index)
        .or_else(|| retained_moves.first())
        .ok_or_else(|| AppError::BadRequest("image sequence has no retained frames".to_string()))?;
    let selected_relative_path = format!("{stored_directory}/{}", selected_move.target_file_name);
    let selected_size = fs::metadata(&selected_move.target_path)
        .map_err(|error| {
            AppError::BadRequest(format!("could not inspect sequence frame: {error}"))
        })?
        .len();
    let total_size = retained_moves.iter().fold(0_u64, |total, move_item| {
        total.saturating_add(
            fs::metadata(&move_item.target_path)
                .map(|metadata| metadata.len())
                .unwrap_or(0),
        )
    });
    let hash = compute_asset_quick_hash(&selected_move.target_path, selected_size)?;
    update_metadata(
        metadata,
        stored_directory,
        prefix,
        suffix,
        extension,
        padding,
        &retained_moves,
        selected_move.next_frame,
        &selected_relative_path,
        total_size,
        &hash,
    )?;

    let mut tx = state.pool.begin().await?;
    let updated_id: Option<String> = sqlx::query_scalar(
        r#"
        UPDATE assets
        SET relative_path = $3,
            storage_key = $3,
            metadata = $4,
            updated_by_user_id = $5,
            updated_at = NOW()
        WHERE library_id = $1 AND id = $2 AND deleted_at IS NULL
        RETURNING id
        "#,
    )
    .bind(library_id)
    .bind(asset_id)
    .bind(&selected_relative_path)
    .bind(&*metadata)
    .bind(user_id)
    .fetch_optional(&mut *tx)
    .await?;
    if updated_id.is_none() {
        return Err(AppError::NotFound("asset not found".to_string()));
    }
    insert_activity_tx(
        &mut tx,
        library_id,
        user_id,
        "assets.sequence_frame_numbers_updated",
        &[asset_id.to_string()],
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

fn validate_request(request: &UpdateImageSequenceFrameNumbersRequest) -> AppResult<()> {
    if request.start_frame < 0 {
        return Err(AppError::BadRequest(
            "sequence start frame cannot be negative".to_string(),
        ));
    }
    if matches!(
        (request.keep_start_frame, request.keep_end_frame),
        (Some(start), Some(end)) if start > end
    ) {
        return Err(AppError::BadRequest(
            "sequence keep range is invalid".to_string(),
        ));
    }
    if request.use_padding && request.padding == Some(0) {
        return Err(AppError::BadRequest(
            "sequence padding must be greater than zero".to_string(),
        ));
    }
    Ok(())
}

fn sequence_string(sequence: &serde_json::Map<String, Value>, field: &str) -> Option<String> {
    sequence
        .get(field)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn sequence_frames(sequence: &serde_json::Map<String, Value>) -> AppResult<Vec<SequenceFrame>> {
    let mut frames = sequence
        .get("frames")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            AppError::BadRequest("image sequence frame metadata is missing".to_string())
        })?
        .iter()
        .filter_map(|frame| {
            Some(SequenceFrame {
                frame: frame.get("frame")?.as_i64()?,
                file_name: frame.get("fileName")?.as_str()?.trim().to_string(),
            })
        })
        .filter(|frame| !frame.file_name.is_empty())
        .collect::<Vec<_>>();
    frames.sort_by(|left, right| {
        left.frame
            .cmp(&right.frame)
            .then_with(|| left.file_name.cmp(&right.file_name))
    });
    let unique_names = frames
        .iter()
        .map(|frame| frame.file_name.to_ascii_lowercase())
        .collect::<HashSet<_>>();
    if frames.is_empty() || unique_names.len() != frames.len() {
        return Err(AppError::BadRequest(
            "image sequence frame metadata is invalid".to_string(),
        ));
    }
    Ok(frames)
}

#[allow(clippy::too_many_arguments)]
fn build_moves(
    sequence_dir: &StdPath,
    frames: &[SequenceFrame],
    retained_frames: &[SequenceFrame],
    start_frame: i64,
    prefix: &str,
    suffix: &str,
    extension: &str,
    padding: usize,
) -> AppResult<Vec<SequenceMove>> {
    let token = Uuid::new_v4();
    let retained_names = retained_frames
        .iter()
        .map(|frame| frame.file_name.to_ascii_lowercase())
        .collect::<HashSet<_>>();
    let original_paths = frames
        .iter()
        .map(|frame| {
            sequence_dir
                .join(&frame.file_name)
                .to_string_lossy()
                .to_ascii_lowercase()
        })
        .collect::<HashSet<_>>();
    let mut retained_index = 0_usize;
    let mut target_names = HashSet::new();
    let mut moves = Vec::with_capacity(frames.len());

    for frame in frames {
        validate_frame_name(&frame.file_name)?;
        let old_path = sequence_dir.join(&frame.file_name);
        if !old_path.is_file() {
            return Err(AppError::NotFound(format!(
                "image sequence frame was not found: {}",
                frame.file_name
            )));
        }
        let retained = retained_names.contains(&frame.file_name.to_ascii_lowercase());
        let (next_frame, target_file_name) = if retained {
            let next_frame = start_frame
                .checked_add(i64::try_from(retained_index).map_err(|_| {
                    AppError::BadRequest("image sequence has too many frames".to_string())
                })?)
                .ok_or_else(|| {
                    AppError::BadRequest("image sequence frame range is too large".to_string())
                })?;
            retained_index += 1;
            let frame_text = if padding > 0 {
                format!("{next_frame:0padding$}")
            } else {
                next_frame.to_string()
            };
            (
                next_frame,
                format!("{prefix}{frame_text}{suffix}.{extension}"),
            )
        } else {
            (frame.frame, frame.file_name.clone())
        };
        validate_frame_name(&target_file_name)?;
        if retained && !target_names.insert(target_file_name.to_ascii_lowercase()) {
            return Err(AppError::Conflict(
                "sequence frame numbering would create duplicate file names".to_string(),
            ));
        }
        let target_path = sequence_dir.join(&target_file_name);
        if retained
            && target_path.exists()
            && !original_paths.contains(&target_path.to_string_lossy().to_ascii_lowercase())
        {
            return Err(AppError::Conflict(format!(
                "a sequence frame already exists: {target_file_name}"
            )));
        }
        let temporary_path = sequence_dir.join(format!(
            ".{}.{}.starary-sequence-tmp",
            frame.file_name, token
        ));
        if temporary_path.exists() {
            return Err(AppError::Conflict(
                "a temporary sequence file already exists".to_string(),
            ));
        }
        moves.push(SequenceMove {
            old_path,
            temporary_path,
            target_path,
            next_frame,
            target_file_name,
            retained,
        });
    }
    Ok(moves)
}

fn validate_frame_name(file_name: &str) -> AppResult<()> {
    if file_name.is_empty()
        || file_name == "."
        || file_name == ".."
        || file_name.contains('/')
        || file_name.contains('\\')
        || file_name.contains('\0')
    {
        return Err(AppError::BadRequest(
            "image sequence frame name is invalid".to_string(),
        ));
    }
    Ok(())
}

fn moves_are_unchanged(moves: &[SequenceMove]) -> bool {
    moves
        .iter()
        .all(|move_item| move_item.retained && move_item.old_path == move_item.target_path)
}

fn stage_moves(moves: &[SequenceMove]) -> AppResult<()> {
    let mut staged_count = 0_usize;
    for move_item in moves {
        if let Err(error) = fs::rename(&move_item.old_path, &move_item.temporary_path) {
            for staged in moves[..staged_count].iter().rev() {
                let _ = fs::rename(&staged.temporary_path, &staged.old_path);
            }
            return Err(AppError::BadRequest(format!(
                "could not stage sequence frame rename: {error}"
            )));
        }
        staged_count += 1;
    }
    Ok(())
}

fn finish_moves(moves: &[SequenceMove]) -> AppResult<()> {
    for move_item in moves.iter().filter(|move_item| move_item.retained) {
        fs::rename(&move_item.temporary_path, &move_item.target_path).map_err(|error| {
            AppError::BadRequest(format!("could not finish sequence frame rename: {error}"))
        })?;
    }
    Ok(())
}

fn rollback_moves(moves: &[SequenceMove]) {
    for move_item in moves.iter().filter(|move_item| move_item.retained).rev() {
        if move_item.target_path.exists() {
            let _ = fs::rename(&move_item.target_path, &move_item.temporary_path);
        }
    }
    for move_item in moves.iter().rev() {
        if move_item.temporary_path.exists() {
            let _ = fs::rename(&move_item.temporary_path, &move_item.old_path);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn update_metadata(
    metadata: &mut Value,
    stored_directory: &str,
    prefix: &str,
    suffix: &str,
    extension: &str,
    padding: usize,
    retained_moves: &[&SequenceMove],
    selected_frame: i64,
    selected_relative_path: &str,
    total_size: u64,
    hash: &str,
) -> AppResult<()> {
    let metadata_object = metadata
        .as_object_mut()
        .ok_or_else(|| AppError::BadRequest("asset metadata is invalid".to_string()))?;
    let fps = metadata_object
        .get("sequence")
        .and_then(Value::as_object)
        .and_then(|sequence| sequence.get("fps"))
        .and_then(Value::as_f64)
        .filter(|fps| *fps > 0.0);
    let sequence = metadata_object
        .get_mut("sequence")
        .and_then(Value::as_object_mut)
        .ok_or_else(|| AppError::BadRequest("image sequence metadata is invalid".to_string()))?;
    let frame_numbers = retained_moves
        .iter()
        .map(|move_item| move_item.next_frame)
        .collect::<Vec<_>>();
    sequence.insert("storedDirectory".to_string(), json!(stored_directory));
    sequence.insert("prefix".to_string(), json!(prefix));
    sequence.insert("suffix".to_string(), json!(suffix));
    sequence.insert("extension".to_string(), json!(extension));
    sequence.insert("padding".to_string(), json!(padding));
    sequence.insert("selectedFrame".to_string(), json!(selected_frame));
    sequence.insert(
        "selectedFramePath".to_string(),
        json!(selected_relative_path),
    );
    sequence.insert("frameCount".to_string(), json!(retained_moves.len()));
    sequence.insert("startFrame".to_string(), json!(frame_numbers.first()));
    sequence.insert("endFrame".to_string(), json!(frame_numbers.last()));
    sequence.insert("missingFrames".to_string(), json!([]));
    sequence.insert(
        "frames".to_string(),
        json!(retained_moves
            .iter()
            .map(|move_item| json!({
                "frame": move_item.next_frame,
                "fileName": move_item.target_file_name,
            }))
            .collect::<Vec<_>>()),
    );
    metadata_object.insert("sourcePath".to_string(), json!(selected_relative_path));
    metadata_object.insert("storedPath".to_string(), json!(selected_relative_path));
    metadata_object.insert("sizeBytes".to_string(), json!(total_size));
    metadata_object.insert("hash".to_string(), json!(hash));
    metadata_object.remove("previewVideoPath");
    metadata_object.insert(
        "previewVideoError".to_string(),
        json!("Preview needs to be rebuilt after sequence frame renumbering."),
    );
    if let Some(fps) = fps {
        let duration = retained_moves.len() as f64 / fps;
        metadata_object.insert("duration".to_string(), json!(duration));
        metadata_object.insert("durationSeconds".to_string(), json!(duration));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn create(test_name: &str) -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be after the Unix epoch")
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "starary-sequence-{test_name}-{}-{nonce}",
                std::process::id()
            ));
            fs::create_dir_all(&path).expect("test directory should be created");
            Self(path)
        }

        fn path(&self) -> &StdPath {
            &self.0
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn test_frames() -> Vec<SequenceFrame> {
        vec![
            SequenceFrame {
                frame: 1,
                file_name: "shot_001.png".to_string(),
            },
            SequenceFrame {
                frame: 2,
                file_name: "shot_002.png".to_string(),
            },
        ]
    }

    #[test]
    fn rollback_restores_renamed_and_trimmed_frames() {
        let directory = TestDirectory::create("rollback");
        let frames = test_frames();
        fs::write(directory.path().join("shot_001.png"), b"first")
            .expect("first frame should be written");
        fs::write(directory.path().join("shot_002.png"), b"second")
            .expect("second frame should be written");

        let moves = build_moves(
            directory.path(),
            &frames,
            &frames[..1],
            10,
            "shot_",
            "",
            "png",
            3,
        )
        .expect("move plan should be valid");
        stage_moves(&moves).expect("frames should be staged");
        finish_moves(&moves).expect("retained frames should be renamed");

        assert!(directory.path().join("shot_010.png").is_file());
        assert!(!directory.path().join("shot_001.png").exists());
        assert!(!directory.path().join("shot_002.png").exists());

        rollback_moves(&moves);

        assert_eq!(
            fs::read(directory.path().join("shot_001.png"))
                .expect("first frame should be restored"),
            b"first"
        );
        assert_eq!(
            fs::read(directory.path().join("shot_002.png"))
                .expect("trimmed frame should be restored"),
            b"second"
        );
        assert!(!directory.path().join("shot_010.png").exists());
        assert!(moves
            .iter()
            .all(|move_item| !move_item.temporary_path.exists()));
    }

    #[test]
    fn staging_failure_restores_already_staged_frames() {
        let directory = TestDirectory::create("staging-failure");
        let frames = test_frames();
        fs::write(directory.path().join("shot_001.png"), b"first")
            .expect("first frame should be written");
        fs::write(directory.path().join("shot_002.png"), b"second")
            .expect("second frame should be written");

        let moves = build_moves(
            directory.path(),
            &frames,
            &frames,
            10,
            "shot_",
            "",
            "png",
            3,
        )
        .expect("move plan should be valid");
        fs::remove_file(directory.path().join("shot_002.png"))
            .expect("second frame should be removed to force a staging error");

        assert!(stage_moves(&moves).is_err());
        assert_eq!(
            fs::read(directory.path().join("shot_001.png"))
                .expect("first frame should be restored"),
            b"first"
        );
        assert!(moves
            .iter()
            .all(|move_item| !move_item.temporary_path.exists()));
    }
}
