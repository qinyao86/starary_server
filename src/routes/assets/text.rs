use super::{
    file_metadata::compute_asset_quick_hash,
    join_safe_relative_path,
    mutations::{insert_activity_tx, mutation_response, AssetMutationResponse},
    normalize_readable_storage_file_relative_path, storage_root_write_base_path,
};
use crate::{
    auth::AuthUser,
    error::{AppError, AppResult},
    routes::access::{ensure_library_access, ensure_library_write_access},
    state::AppState,
};
use axum::{
    extract::{Path, State},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::Row;
use std::{fs, path::Path as StdPath};
use uuid::Uuid;

const TEXT_VIEWER_MAX_BYTES: u64 = 2 * 1024 * 1024;
const TEXT_PREVIEW_MAX_CHARS: usize = 280;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdateAssetTextRequest {
    pub text: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetTextResponse {
    pub text: String,
    pub size_bytes: u64,
}

pub async fn read_asset_text(
    State(state): State<AppState>,
    user: AuthUser,
    Path((library_id, asset_id)): Path<(String, String)>,
) -> AppResult<Json<AssetTextResponse>> {
    ensure_library_access(&state, &user, &library_id).await?;
    let path = resolve_text_asset_path(&state, &library_id, &asset_id).await?;
    let bytes = read_text_asset_bytes(&path)?;
    let size_bytes = bytes.len() as u64;
    let text = String::from_utf8(bytes)
        .map_err(|_| AppError::BadRequest("only UTF-8 text files can be edited".to_string()))?;
    Ok(Json(AssetTextResponse { text, size_bytes }))
}

pub async fn update_asset_text(
    State(state): State<AppState>,
    user: AuthUser,
    Path((library_id, asset_id)): Path<(String, String)>,
    Json(request): Json<UpdateAssetTextRequest>,
) -> AppResult<Json<AssetMutationResponse>> {
    ensure_library_write_access(&state, &user, &library_id).await?;
    if request.text.len() as u64 > TEXT_VIEWER_MAX_BYTES {
        return Err(AppError::BadRequest(
            "text file is too large to edit in the app".to_string(),
        ));
    }

    let path = resolve_text_asset_path(&state, &library_id, &asset_id).await?;
    let previous_bytes = read_text_asset_bytes(&path)?;
    write_text_file_atomic(&path, request.text.as_bytes())?;

    if let Err(error) = update_asset_text_record(
        &state,
        &library_id,
        &asset_id,
        user.id,
        &request.text,
        &path,
    )
    .await
    {
        let _ = write_text_file_atomic(&path, &previous_bytes);
        return Err(error);
    }

    Ok(Json(
        mutation_response(&state, &library_id, user.id, vec![asset_id]).await?,
    ))
}

async fn resolve_text_asset_path(
    state: &AppState,
    library_id: &str,
    asset_id: &str,
) -> AppResult<std::path::PathBuf> {
    let row = sqlx::query(
        "SELECT storage_root_id, relative_path FROM assets WHERE library_id = $1 AND id = $2 AND deleted_at IS NULL",
    )
    .bind(library_id)
    .bind(asset_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::NotFound("asset not found".to_string()))?;
    let storage_root_id: Option<Uuid> = row.try_get("storage_root_id")?;
    let relative_path: Option<String> = row.try_get("relative_path")?;
    let storage_root_id = storage_root_id.ok_or_else(|| {
        AppError::BadRequest("asset does not have an enabled workspace".to_string())
    })?;
    let relative_path = normalize_readable_storage_file_relative_path(
        relative_path
            .as_deref()
            .ok_or_else(|| AppError::BadRequest("asset has no source path".to_string()))?,
    )?;
    validate_text_asset_extension(&relative_path)?;
    let base_path = storage_root_write_base_path(state, storage_root_id, Some(library_id)).await?;
    Ok(join_safe_relative_path(&base_path, &relative_path))
}

fn validate_text_asset_extension(path: &str) -> AppResult<()> {
    let extension = path
        .rsplit_once('.')
        .map(|(_, extension)| extension.to_ascii_lowercase())
        .unwrap_or_default();
    if matches!(extension.as_str(), "txt" | "md" | "markdown") {
        return Ok(());
    }
    Err(AppError::BadRequest(
        "this file type is not supported by the text editor".to_string(),
    ))
}

fn read_text_asset_bytes(path: &StdPath) -> AppResult<Vec<u8>> {
    let metadata = fs::metadata(path).map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            AppError::NotFound("asset file not found".to_string())
        } else {
            AppError::BadRequest(format!("could not read text asset: {error}"))
        }
    })?;
    if metadata.len() > TEXT_VIEWER_MAX_BYTES {
        return Err(AppError::BadRequest(
            "text file is too large to edit in the app".to_string(),
        ));
    }
    fs::read(path)
        .map_err(|error| AppError::BadRequest(format!("could not read text asset: {error}")))
}

fn write_text_file_atomic(path: &StdPath, bytes: &[u8]) -> AppResult<()> {
    let token = Uuid::new_v4();
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or("txt");
    let temporary_path = path.with_extension(format!("{extension}.{token}.madlibrary-tmp"));
    let backup_path = path.with_extension(format!("{extension}.{token}.madlibrary-backup"));
    fs::write(&temporary_path, bytes)
        .map_err(|error| AppError::BadRequest(format!("could not write text asset: {error}")))?;
    fs::rename(path, &backup_path).map_err(|error| {
        let _ = fs::remove_file(&temporary_path);
        AppError::BadRequest(format!("could not prepare text asset replacement: {error}"))
    })?;
    if let Err(error) = fs::rename(&temporary_path, path) {
        let _ = fs::rename(&backup_path, path);
        let _ = fs::remove_file(&temporary_path);
        return Err(AppError::BadRequest(format!(
            "could not replace text asset: {error}"
        )));
    }
    let _ = fs::remove_file(backup_path);
    Ok(())
}

async fn update_asset_text_record(
    state: &AppState,
    library_id: &str,
    asset_id: &str,
    user_id: Uuid,
    text: &str,
    path: &StdPath,
) -> AppResult<()> {
    let metadata = fs::metadata(path)
        .map_err(|error| AppError::BadRequest(format!("could not inspect text asset: {error}")))?;
    let size_bytes = metadata.len();
    let hash = compute_asset_quick_hash(path, size_bytes)?;
    let file_modified_ms = metadata
        .modified()
        .ok()
        .and_then(|modified_at| modified_at.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|duration| duration.as_millis() as u64);
    let metadata_patch = json!({
        "fileModifiedMs": file_modified_ms,
        "hash": hash,
        "sizeBytes": size_bytes,
        "textEncoding": "utf-8",
        "textPreview": build_text_preview(text),
    });
    let mut tx = state.pool.begin().await?;
    let updated_id: Option<String> = sqlx::query_scalar(
        r#"
        UPDATE assets
        SET metadata = metadata || $3::jsonb,
            updated_by_user_id = $4,
            updated_at = NOW()
        WHERE library_id = $1 AND id = $2 AND deleted_at IS NULL
        RETURNING id
        "#,
    )
    .bind(library_id)
    .bind(asset_id)
    .bind(metadata_patch)
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
        "assets.text_updated",
        &[asset_id.to_string()],
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

fn build_text_preview(text: &str) -> String {
    text.replace("\r\n", "\n")
        .replace('\r', "\n")
        .chars()
        .take(TEXT_PREVIEW_MAX_CHARS)
        .collect()
}
