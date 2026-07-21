use crate::{
    auth::{auth_user_from_token, AuthUser},
    error::{AppError, AppResult},
    ids::generate_id,
    models::{AssetRecord, FolderRecord, StorageRootKind, TagRecord},
    routes::access::{ensure_library_access, ensure_library_write_access},
    state::AppState,
};
use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{header, HeaderMap, Response, StatusCode},
    Json,
};
use base64::{engine::general_purpose, Engine};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::Row;
use std::{
    collections::{HashMap, HashSet},
    fs,
    io::SeekFrom,
    path::{Component, Path as StdPath, PathBuf},
};
use tokio::{
    fs::File,
    io::{AsyncReadExt, AsyncSeekExt},
};
use tokio_util::io::ReaderStream;
use uuid::Uuid;

mod duplicates;
mod file_metadata;
mod mutations;
mod query;
mod sequence;
mod text;

pub use duplicates::merge_duplicate_assets;
pub use mutations::{
    delete_assets_permanently, restore_assets, set_asset_folders, set_asset_tags, trash_assets,
    update_asset, update_asset_derived_files, update_assets_rating, update_assets_starred,
    update_assets_viewer,
};
pub use query::{query_asset_ids, query_assets};
pub use sequence::update_image_sequence_frame_numbers;
pub use text::{read_asset_text, update_asset_text};

const MAX_DERIVED_FILE_BYTES: usize = 25 * 1024 * 1024;
const MAX_SOURCE_FILE_BYTES: usize = 256 * 1024 * 1024;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListAssetsQuery {
    #[serde(default = "default_limit")]
    limit: i64,
    #[serde(default)]
    offset: i64,
    #[serde(default)]
    include_deleted: bool,
    #[serde(default)]
    deleted_only: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetListResponse {
    items: Vec<AssetResponse>,
    total: i64,
    limit: i64,
    offset: i64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportAssetsRequest {
    assets: Vec<ImportAssetRequest>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportAssetRequest {
    id: Option<String>,
    name: String,
    asset_kind: String,
    import_mode: Option<String>,
    storage_key: Option<String>,
    storage_root_id: Option<Uuid>,
    relative_path: Option<String>,
    metadata: Option<Value>,
    folder_id: Option<String>,
    #[serde(default)]
    folder_ids: Vec<String>,
    #[serde(default)]
    tags: Vec<ImportAssetTagRequest>,
    source_file: Option<ImportAssetFileRequest>,
    #[serde(default)]
    additional_files: Vec<ImportAssetFileRequest>,
    #[serde(default)]
    derived_files: Vec<ImportAssetDerivedFileRequest>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportAssetTagRequest {
    name: String,
    color: Option<String>,
    starred: Option<bool>,
    sort_order: Option<i64>,
    group: Option<ImportAssetTagGroupRequest>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportAssetTagGroupRequest {
    name: String,
    color: Option<String>,
    sort_order: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportAssetFileRequest {
    relative_path: String,
    content_base64: String,
    content_type: Option<String>,
    size_bytes: Option<u64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportAssetDerivedFileRequest {
    kind: String,
    #[serde(flatten)]
    file: ImportAssetFileRequest,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetFileUrls {
    pub(super) source: Option<String>,
    pub(super) thumbnail: Option<String>,
    pub(super) preview_image: Option<String>,
    pub(super) preview_video: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetFilePathVariants {
    relative_path: String,
    canonical_uri: Option<String>,
    windows_path: Option<String>,
    macos_smb_url: Option<String>,
    macos_mount_paths: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetFilePaths {
    source: Option<AssetFilePathVariants>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetResponse {
    #[serde(flatten)]
    pub(super) asset: AssetRecord,
    pub(super) folders: Vec<FolderRecord>,
    pub(super) tags: Vec<TagRecord>,
    pub(super) starred: bool,
    pub(super) favorite_count: i64,
    pub(super) file_urls: AssetFileUrls,
    pub(super) file_paths: AssetFilePaths,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportAssetsResponse {
    items: Vec<AssetResponse>,
    imported_count: usize,
    total: i64,
}

#[derive(Clone)]
struct AssetStorageRootInfo {
    canonical_uri: String,
    windows_unc_path: Option<String>,
    macos_smb_url: Option<String>,
    macos_mount_aliases: Value,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetFileQuery {
    token: String,
}

pub async fn list_assets(
    State(state): State<AppState>,
    user: AuthUser,
    Path(library_id): Path<String>,
    Query(query): Query<ListAssetsQuery>,
) -> AppResult<Json<AssetListResponse>> {
    ensure_library_access(&state, &user, &library_id).await?;

    let limit = query.limit.clamp(1, 500);
    let offset = query.offset.max(0);
    let total = count_assets(
        &state,
        &library_id,
        query.include_deleted,
        query.deleted_only,
    )
    .await?;
    let records = query_assets_page(
        &state,
        &library_id,
        query.include_deleted,
        query.deleted_only,
        limit,
        offset,
    )
    .await?;
    let items = build_asset_responses(&state, &library_id, user.id, records).await?;

    Ok(Json(AssetListResponse {
        items,
        total,
        limit,
        offset,
    }))
}

pub async fn import_assets(
    State(state): State<AppState>,
    user: AuthUser,
    Path(library_id): Path<String>,
    Json(request): Json<ImportAssetsRequest>,
) -> AppResult<Json<ImportAssetsResponse>> {
    ensure_library_write_access(&state, &user, &library_id).await?;

    if request.assets.is_empty() {
        return Err(AppError::BadRequest(
            "at least one asset is required".to_string(),
        ));
    }
    if request.assets.len() > 500 {
        return Err(AppError::BadRequest(
            "cannot import more than 500 assets at once".to_string(),
        ));
    }

    let default_storage_root_id = find_default_storage_root_id(&state, &library_id).await?;
    let mut tx = state.pool.begin().await?;
    let mut imported_asset_ids = Vec::with_capacity(request.assets.len());
    let mut storage_was_used = false;

    for asset in request.assets {
        let derived_files = asset.derived_files;
        let additional_files = asset.additional_files;
        let source_file = asset.source_file;
        let asset_id = normalize_asset_id(asset.id);
        let name = normalize_required_text(&asset.name, "asset name")?;
        let asset_kind = normalize_required_text(&asset.asset_kind, "asset kind")?;
        let import_mode = normalize_import_mode(asset.import_mode.as_deref())?;
        let mut metadata = asset.metadata.unwrap_or_else(|| json!({}));
        if let Some(operation_id) = metadata_string(&metadata, "transferOperationId") {
            let existing_operation_id = sqlx::query_scalar::<_, String>(
                "SELECT metadata->>'transferOperationId' FROM assets WHERE library_id = $1 AND id = $2",
            )
            .bind(&library_id)
            .bind(&asset_id)
            .fetch_optional(&mut *tx)
            .await?;
            if let Some(existing_operation_id) = existing_operation_id {
                if existing_operation_id == operation_id {
                    imported_asset_ids.push(asset_id);
                    continue;
                }
                return Err(AppError::Conflict("asset already exists".to_string()));
            }
        }

        let mut storage_key = normalize_optional_text(asset.storage_key);
        let storage_root_id = asset.storage_root_id.or(default_storage_root_id);
        let relative_path = normalize_optional_text(asset.relative_path)
            .or_else(|| metadata_string(&metadata, "sourcePath"))
            .or_else(|| metadata_string(&metadata, "storedPath"))
            .or_else(|| Some(name.clone()));

        if import_mode == "copy" {
            if storage_key.is_none() {
                storage_key = relative_path.clone();
            }
            if storage_key.is_none() && source_file.is_none() {
                return Err(AppError::BadRequest(
                    "copy imports require a source file or storage key".to_string(),
                ));
            }
        }
        if import_mode == "reference" && (storage_root_id.is_none() || relative_path.is_none()) {
            return Err(AppError::BadRequest(
                "reference imports require an enabled workspace".to_string(),
            ));
        }
        if let Some(root_id) = storage_root_id {
            storage_was_used = true;
            ensure_storage_root_in_library(&mut tx, &library_id, root_id).await?;
            if let Some(source_file) = source_file.as_ref() {
                write_asset_source_file(&state, root_id, &asset_id, source_file).await?;
            }
            for additional_file in &additional_files {
                write_asset_source_file(&state, root_id, &asset_id, additional_file).await?;
            }
            if !derived_files.is_empty() {
                write_asset_derived_files(&state, root_id, &derived_files).await?;
            }
        } else if source_file.is_some() || !additional_files.is_empty() || !derived_files.is_empty()
        {
            return Err(AppError::BadRequest(
                "asset files require an enabled workspace".to_string(),
            ));
        }
        let mut folder_ids = asset
            .folder_ids
            .into_iter()
            .filter_map(|value| normalize_optional_text(Some(value)))
            .collect::<Vec<_>>();
        if let Some(folder_id) = normalize_optional_text(asset.folder_id) {
            folder_ids.push(folder_id);
        }
        folder_ids.sort();
        folder_ids.dedup();
        for folder_id in &folder_ids {
            ensure_folder_in_library(&mut tx, &library_id, folder_id).await?;
        }

        ensure_metadata_field(&mut metadata, "name", json!(name));
        ensure_metadata_field(&mut metadata, "assetKind", json!(asset_kind));
        ensure_metadata_field(&mut metadata, "importMode", json!(import_mode));
        if let Some(relative_path) = relative_path.as_deref() {
            ensure_metadata_field(&mut metadata, "sourcePath", json!(relative_path));
        }

        sqlx::query(
            r#"
            INSERT INTO assets (
                id,
                library_id,
                name,
                asset_kind,
                import_mode,
                storage_key,
                storage_root_id,
                relative_path,
                metadata,
                created_by_user_id,
                imported_by_user_id,
                imported_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9::jsonb, $10, $10, NOW())
            "#,
        )
        .bind(&asset_id)
        .bind(&library_id)
        .bind(&name)
        .bind(&asset_kind)
        .bind(&import_mode)
        .bind(storage_key)
        .bind(storage_root_id)
        .bind(relative_path)
        .bind(metadata)
        .bind(user.id)
        .execute(&mut *tx)
        .await
        .map_err(map_asset_insert_error)?;

        for folder_id in folder_ids {
            sqlx::query(
                r#"
                INSERT INTO asset_folders (asset_id, folder_id)
                VALUES ($1, $2)
                ON CONFLICT DO NOTHING
                "#,
            )
            .bind(&asset_id)
            .bind(&folder_id)
            .execute(&mut *tx)
            .await?;
        }

        for tag in asset.tags {
            let tag_id = upsert_import_tag(&mut tx, &library_id, user.id, tag).await?;
            sqlx::query(
                "INSERT INTO asset_tags (asset_id, tag_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
            )
            .bind(&asset_id)
            .bind(tag_id)
            .execute(&mut *tx)
            .await?;
        }

        imported_asset_ids.push(asset_id);
    }

    let imported_count = imported_asset_ids.len();
    sqlx::query(
        r#"
        INSERT INTO activity_log (
            id,
            library_id,
            actor_user_id,
            action,
            target_type,
            details
        )
        VALUES ($1, $2, $3, 'assets.imported', 'asset', $4::jsonb)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(&library_id)
    .bind(user.id)
    .bind(json!({
        "assetIds": imported_asset_ids.clone(),
        "count": imported_count,
    }))
    .execute(&mut *tx)
    .await?;

    if storage_was_used {
        sqlx::query(
            "UPDATE libraries SET storage_locked_at = COALESCE(storage_locked_at, NOW()) WHERE id = $1",
        )
        .bind(&library_id)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;

    let records = query_assets_by_ids(&state, &library_id, &imported_asset_ids).await?;
    let items = build_asset_responses(&state, &library_id, user.id, records).await?;
    let total = count_assets(&state, &library_id, false, false).await?;

    Ok(Json(ImportAssetsResponse {
        imported_count: items.len(),
        total,
        items,
    }))
}

async fn upsert_import_tag(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    library_id: &str,
    user_id: Uuid,
    tag: ImportAssetTagRequest,
) -> AppResult<String> {
    let name = normalize_required_text(&tag.name, "tag name")?;
    if let Some(tag_id) = sqlx::query_scalar::<_, String>(
        "SELECT id FROM tags WHERE library_id = $1 AND LOWER(name) = LOWER($2) ORDER BY created_at ASC LIMIT 1",
    )
    .bind(library_id)
    .bind(&name)
    .fetch_optional(&mut **tx)
    .await?
    {
        return Ok(tag_id);
    }

    let group_id = if let Some(group) = tag.group {
        let group_name = normalize_required_text(&group.name, "tag group name")?;
        if let Some(group_id) = sqlx::query_scalar::<_, String>(
            "SELECT id FROM tag_groups WHERE library_id = $1 AND LOWER(name) = LOWER($2) ORDER BY created_at ASC LIMIT 1",
        )
        .bind(library_id)
        .bind(&group_name)
        .fetch_optional(&mut **tx)
        .await?
        {
            Some(group_id)
        } else {
            let group_id = generate_id("tag_group_");
            sqlx::query(
                r#"
                INSERT INTO tag_groups (
                    id, library_id, name, color, sort_order, created_by_user_id, updated_by_user_id
                ) VALUES ($1, $2, $3, $4, $5, $6, $6)
                "#,
            )
            .bind(&group_id)
            .bind(library_id)
            .bind(group_name)
            .bind(group.color.unwrap_or_else(|| "default".to_string()))
            .bind(group.sort_order.unwrap_or(0))
            .bind(user_id)
            .execute(&mut **tx)
            .await?;
            Some(group_id)
        }
    } else {
        None
    };

    let tag_id = generate_id("tag_");
    sqlx::query(
        r#"
        INSERT INTO tags (
            id, library_id, group_id, name, color, starred, sort_order,
            created_by_user_id, updated_by_user_id
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $8)
        "#,
    )
    .bind(&tag_id)
    .bind(library_id)
    .bind(group_id)
    .bind(name)
    .bind(tag.color)
    .bind(tag.starred.unwrap_or(false))
    .bind(tag.sort_order.unwrap_or(0))
    .bind(user_id)
    .execute(&mut **tx)
    .await?;
    Ok(tag_id)
}

fn default_limit() -> i64 {
    100
}

pub async fn read_library_storage_file(
    State(state): State<AppState>,
    Path((library_id, storage_root_id, relative_path)): Path<(String, Uuid, String)>,
    headers: HeaderMap,
    Query(query): Query<AssetFileQuery>,
) -> AppResult<Response<Body>> {
    let user = auth_user_from_token(&state, &query.token).await?;
    ensure_library_access(&state, &user, &library_id).await?;

    let relative_path = normalize_readable_storage_file_relative_path(&relative_path)?;
    let base_path =
        storage_root_write_base_path(&state, storage_root_id, Some(&library_id)).await?;
    let file_path = join_safe_relative_path(&base_path, &relative_path);
    let mut file = File::open(&file_path).await.map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            AppError::NotFound("asset file not found".to_string())
        } else {
            AppError::BadRequest(format!("could not open asset file: {error}"))
        }
    })?;
    let file_size = file
        .metadata()
        .await
        .map_err(|error| {
            AppError::BadRequest(format!("could not read asset file metadata: {error}"))
        })?
        .len();
    let requested_range = headers.get(header::RANGE);
    let byte_range = requested_range.and_then(|value| {
        value
            .to_str()
            .ok()
            .and_then(|value| parse_single_byte_range(value, file_size))
    });

    if requested_range.is_some() && byte_range.is_none() {
        return Response::builder()
            .status(StatusCode::RANGE_NOT_SATISFIABLE)
            .header(header::ACCEPT_RANGES, "bytes")
            .header(header::CONTENT_RANGE, format!("bytes */{file_size}"))
            .body(Body::empty())
            .map_err(|error| AppError::BadRequest(error.to_string()));
    }

    let response = Response::builder()
        .header(header::ACCEPT_RANGES, "bytes")
        .header(header::CACHE_CONTROL, "private, max-age=86400")
        .header(header::CONTENT_TYPE, content_type_for_path(&relative_path));

    if let Some((start, end)) = byte_range {
        let content_length = end - start + 1;
        file.seek(SeekFrom::Start(start))
            .await
            .map_err(|error| AppError::BadRequest(format!("could not seek asset file: {error}")))?;
        response
            .status(StatusCode::PARTIAL_CONTENT)
            .header(header::CONTENT_LENGTH, content_length.to_string())
            .header(
                header::CONTENT_RANGE,
                format!("bytes {start}-{end}/{file_size}"),
            )
            .body(Body::from_stream(ReaderStream::new(
                file.take(content_length),
            )))
            .map_err(|error| AppError::BadRequest(error.to_string()))
    } else {
        response
            .status(StatusCode::OK)
            .header(header::CONTENT_LENGTH, file_size.to_string())
            .body(Body::from_stream(ReaderStream::new(file)))
            .map_err(|error| AppError::BadRequest(error.to_string()))
    }
}

fn parse_single_byte_range(value: &str, file_size: u64) -> Option<(u64, u64)> {
    if file_size == 0 {
        return None;
    }

    let range = value.trim().strip_prefix("bytes=")?;
    if range.contains(',') {
        return None;
    }
    let (start, end) = range.split_once('-')?;

    if start.is_empty() {
        let suffix_length = end.parse::<u64>().ok()?;
        if suffix_length == 0 {
            return None;
        }
        let start = file_size.saturating_sub(suffix_length);
        return Some((start, file_size - 1));
    }

    let start = start.parse::<u64>().ok()?;
    if start >= file_size {
        return None;
    }
    let end = if end.is_empty() {
        file_size - 1
    } else {
        end.parse::<u64>().ok()?.min(file_size - 1)
    };
    (end >= start).then_some((start, end))
}

#[cfg(test)]
mod byte_range_tests {
    use super::parse_single_byte_range;

    #[test]
    fn parses_supported_single_ranges() {
        assert_eq!(parse_single_byte_range("bytes=0-9", 100), Some((0, 9)));
        assert_eq!(parse_single_byte_range("bytes=90-", 100), Some((90, 99)));
        assert_eq!(parse_single_byte_range("bytes=-10", 100), Some((90, 99)));
        assert_eq!(parse_single_byte_range("bytes=90-200", 100), Some((90, 99)));
    }

    #[test]
    fn rejects_invalid_or_multiple_ranges() {
        assert_eq!(parse_single_byte_range("bytes=100-", 100), None);
        assert_eq!(parse_single_byte_range("bytes=20-10", 100), None);
        assert_eq!(parse_single_byte_range("bytes=0-1,4-5", 100), None);
        assert_eq!(parse_single_byte_range("items=0-9", 100), None);
        assert_eq!(parse_single_byte_range("bytes=0-9", 0), None);
    }
}

async fn query_assets_page(
    state: &AppState,
    library_id: &str,
    include_deleted: bool,
    deleted_only: bool,
    limit: i64,
    offset: i64,
) -> AppResult<Vec<AssetRecord>> {
    Ok(sqlx::query_as::<_, AssetRecord>(
        r#"
        SELECT
            id,
            library_id,
            name,
            asset_kind,
            import_mode,
            storage_key,
            storage_root_id,
            relative_path,
            metadata,
            created_by_user_id,
            imported_by_user_id,
            updated_by_user_id,
            deleted_by_user_id,
            restored_by_user_id,
            created_at,
            imported_at,
            updated_at,
            deleted_at,
            restored_at
        FROM assets
        WHERE library_id = $1
          AND (($3 AND deleted_at IS NOT NULL) OR (NOT $3 AND ($2 OR deleted_at IS NULL)))
        ORDER BY created_at DESC, id DESC
        LIMIT $4 OFFSET $5
        "#,
    )
    .bind(library_id)
    .bind(include_deleted)
    .bind(deleted_only)
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.pool)
    .await?)
}

pub(super) async fn build_asset_responses(
    state: &AppState,
    library_id: &str,
    user_id: Uuid,
    records: Vec<AssetRecord>,
) -> AppResult<Vec<AssetResponse>> {
    if records.is_empty() {
        return Ok(Vec::new());
    }

    let asset_ids = records
        .iter()
        .map(|asset| asset.id.clone())
        .collect::<Vec<_>>();
    let mut folders_by_asset = query_asset_folder_relations(state, library_id, &asset_ids).await?;
    let mut tags_by_asset = query_asset_tag_relations(state, library_id, &asset_ids).await?;
    let favorite_state_by_asset =
        query_asset_favorite_states(state, library_id, user_id, &asset_ids).await?;
    let storage_root_ids = records
        .iter()
        .filter_map(|asset| asset.storage_root_id)
        .collect::<HashSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let storage_roots_by_id =
        query_asset_storage_roots(state, library_id, &storage_root_ids).await?;

    Ok(records
        .into_iter()
        .map(|asset| {
            let file_urls = build_asset_file_urls(&asset);
            let file_paths = build_asset_file_paths(
                &asset,
                asset
                    .storage_root_id
                    .and_then(|root_id| storage_roots_by_id.get(&root_id)),
            );
            let folders = folders_by_asset.remove(&asset.id).unwrap_or_default();
            let tags = tags_by_asset.remove(&asset.id).unwrap_or_default();
            let (starred, favorite_count) = favorite_state_by_asset
                .get(&asset.id)
                .copied()
                .unwrap_or((false, 0));
            AssetResponse {
                asset,
                folders,
                tags,
                starred,
                favorite_count,
                file_urls,
                file_paths,
            }
        })
        .collect())
}

async fn query_asset_favorite_states(
    state: &AppState,
    library_id: &str,
    user_id: Uuid,
    asset_ids: &[String],
) -> AppResult<HashMap<String, (bool, i64)>> {
    if asset_ids.is_empty() {
        return Ok(HashMap::new());
    }

    let rows = sqlx::query(
        r#"
        SELECT
            asset_id,
            BOOL_OR(user_id = $2) AS starred,
            COUNT(*)::BIGINT AS favorite_count
        FROM asset_favorites
        WHERE library_id = $1
          AND asset_id = ANY($3)
        GROUP BY asset_id
        "#,
    )
    .bind(library_id)
    .bind(user_id)
    .bind(asset_ids)
    .fetch_all(&state.pool)
    .await?;

    rows.into_iter()
        .map(|row| {
            Ok((
                row.try_get::<String, _>("asset_id")?,
                (
                    row.try_get::<bool, _>("starred")?,
                    row.try_get::<i64, _>("favorite_count")?,
                ),
            ))
        })
        .collect()
}

async fn query_asset_storage_roots(
    state: &AppState,
    library_id: &str,
    root_ids: &[Uuid],
) -> AppResult<HashMap<Uuid, AssetStorageRootInfo>> {
    if root_ids.is_empty() {
        return Ok(HashMap::new());
    }

    let rows = sqlx::query(
        r#"
        SELECT
            id,
            canonical_uri,
            windows_unc_path,
            macos_smb_url,
            macos_mount_aliases
        FROM storage_roots
        WHERE library_id = $1 AND id = ANY($2)
        "#,
    )
    .bind(library_id)
    .bind(root_ids)
    .fetch_all(&state.pool)
    .await?;

    let mut roots = HashMap::new();
    for row in rows {
        let id: Uuid = row.try_get("id")?;
        roots.insert(
            id,
            AssetStorageRootInfo {
                canonical_uri: row.try_get("canonical_uri")?,
                windows_unc_path: row.try_get("windows_unc_path")?,
                macos_smb_url: row.try_get("macos_smb_url")?,
                macos_mount_aliases: row.try_get("macos_mount_aliases")?,
            },
        );
    }

    Ok(roots)
}

async fn query_asset_folder_relations(
    state: &AppState,
    library_id: &str,
    asset_ids: &[String],
) -> AppResult<HashMap<String, Vec<FolderRecord>>> {
    let rows = sqlx::query(
        r#"
        SELECT
            af.asset_id AS relation_asset_id,
            f.id,
            f.parent_id,
            f.name,
            f.description,
            f.icon,
            f.color,
            COUNT(counted_assets.id)::BIGINT AS asset_count,
            f.cover_asset_id,
            CASE WHEN cover_asset.id IS NULL THEN NULL ELSE json_build_object(
                'id', cover_asset.id,
                'name', cover_asset.name,
                'assetKind', cover_asset.asset_kind,
                'storedPath', cover_asset.relative_path,
                'thumbnailPath', cover_asset.metadata->>'thumbnailPath'
            ) END AS cover_asset,
            f.smart_import_id,
            f.sort_order,
            f.created_by_user_id,
            f.updated_by_user_id,
            f.created_at,
            f.updated_at
        FROM asset_folders af
        JOIN folders f ON f.id = af.folder_id AND f.library_id = $2
        LEFT JOIN asset_folders counted_af ON counted_af.folder_id = f.id
        LEFT JOIN assets counted_assets
            ON counted_assets.id = counted_af.asset_id
           AND counted_assets.library_id = f.library_id
           AND counted_assets.deleted_at IS NULL
        LEFT JOIN assets cover_asset ON cover_asset.id = f.cover_asset_id AND cover_asset.deleted_at IS NULL
        WHERE af.asset_id = ANY($1)
        GROUP BY af.asset_id, f.id, cover_asset.id
        ORDER BY f.sort_order ASC, f.name ASC
        "#,
    )
    .bind(asset_ids)
    .bind(library_id)
    .fetch_all(&state.pool)
    .await?;

    let mut folders_by_asset: HashMap<String, Vec<FolderRecord>> = HashMap::new();
    for row in rows {
        let asset_id: String = row.try_get("relation_asset_id")?;
        folders_by_asset
            .entry(asset_id)
            .or_default()
            .push(FolderRecord {
                id: row.try_get("id")?,
                parent_id: row.try_get("parent_id")?,
                name: row.try_get("name")?,
                description: row.try_get("description")?,
                icon: row.try_get("icon")?,
                color: row.try_get("color")?,
                asset_count: row.try_get("asset_count")?,
                cover_asset_id: row.try_get("cover_asset_id")?,
                cover_asset: row.try_get("cover_asset")?,
                smart_import_id: row.try_get("smart_import_id")?,
                sort_order: row.try_get("sort_order")?,
                created_by_user_id: row.try_get("created_by_user_id")?,
                updated_by_user_id: row.try_get("updated_by_user_id")?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            });
    }

    Ok(folders_by_asset)
}

async fn query_asset_tag_relations(
    state: &AppState,
    library_id: &str,
    asset_ids: &[String],
) -> AppResult<HashMap<String, Vec<TagRecord>>> {
    let rows = sqlx::query(
        r#"
        SELECT
            at.asset_id AS relation_asset_id,
            t.id,
            t.group_id,
            t.name,
            t.color,
            t.starred,
            COUNT(counted_assets.id)::BIGINT AS asset_count,
            t.sort_order,
            t.created_by_user_id,
            t.updated_by_user_id,
            t.created_at,
            t.updated_at
        FROM asset_tags at
        JOIN tags t ON t.id = at.tag_id AND t.library_id = $2
        LEFT JOIN asset_tags counted_at ON counted_at.tag_id = t.id
        LEFT JOIN assets counted_assets
            ON counted_assets.id = counted_at.asset_id
           AND counted_assets.library_id = t.library_id
           AND counted_assets.deleted_at IS NULL
        WHERE at.asset_id = ANY($1)
        GROUP BY at.asset_id, t.id
        ORDER BY t.sort_order ASC, t.name ASC
        "#,
    )
    .bind(asset_ids)
    .bind(library_id)
    .fetch_all(&state.pool)
    .await?;

    let mut tags_by_asset: HashMap<String, Vec<TagRecord>> = HashMap::new();
    for row in rows {
        let asset_id: String = row.try_get("relation_asset_id")?;
        tags_by_asset.entry(asset_id).or_default().push(TagRecord {
            id: row.try_get("id")?,
            group_id: row.try_get("group_id")?,
            name: row.try_get("name")?,
            color: row.try_get("color")?,
            starred: row.try_get("starred")?,
            asset_count: row.try_get("asset_count")?,
            sort_order: row.try_get("sort_order")?,
            created_by_user_id: row.try_get("created_by_user_id")?,
            updated_by_user_id: row.try_get("updated_by_user_id")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        });
    }

    Ok(tags_by_asset)
}

fn build_asset_file_urls(asset: &AssetRecord) -> AssetFileUrls {
    let Some(storage_root_id) = asset.storage_root_id else {
        return AssetFileUrls {
            source: None,
            thumbnail: None,
            preview_image: None,
            preview_video: None,
        };
    };

    AssetFileUrls {
        source: asset
            .relative_path
            .as_deref()
            .and_then(|path| build_asset_file_url(&asset.library_id, storage_root_id, path)),
        thumbnail: metadata_string(&asset.metadata, "thumbnailPath")
            .and_then(|path| build_asset_file_url(&asset.library_id, storage_root_id, &path)),
        preview_image: metadata_string(&asset.metadata, "previewImagePath")
            .and_then(|path| build_asset_file_url(&asset.library_id, storage_root_id, &path)),
        preview_video: metadata_string(&asset.metadata, "previewVideoPath")
            .and_then(|path| build_asset_file_url(&asset.library_id, storage_root_id, &path)),
    }
}

fn build_asset_file_paths(
    asset: &AssetRecord,
    storage_root: Option<&AssetStorageRootInfo>,
) -> AssetFilePaths {
    let source = match (asset.relative_path.as_deref(), storage_root) {
        (Some(relative_path), Some(storage_root)) => {
            build_asset_file_path_variants(storage_root, relative_path)
        }
        _ => None,
    };

    AssetFilePaths { source }
}

fn build_asset_file_path_variants(
    storage_root: &AssetStorageRootInfo,
    relative_path: &str,
) -> Option<AssetFilePathVariants> {
    let relative_path = normalize_readable_storage_file_relative_path(relative_path).ok()?;
    let canonical_uri = append_storage_relative_path(&storage_root.canonical_uri, &relative_path);
    let windows_path = storage_root
        .windows_unc_path
        .as_deref()
        .map(|path| append_storage_relative_path(path, &relative_path));
    let macos_smb_url = storage_root
        .macos_smb_url
        .as_deref()
        .map(|path| append_storage_relative_path(path, &relative_path));
    let macos_mount_paths = json_string_array(&storage_root.macos_mount_aliases)
        .into_iter()
        .map(|path| append_storage_relative_path(&path, &relative_path))
        .collect();

    Some(AssetFilePathVariants {
        relative_path,
        canonical_uri: Some(canonical_uri),
        windows_path,
        macos_smb_url,
        macos_mount_paths,
    })
}

pub(super) fn build_asset_file_url(
    library_id: &str,
    storage_root_id: Uuid,
    relative_path: &str,
) -> Option<String> {
    let relative_path = normalize_readable_storage_file_relative_path(relative_path).ok()?;
    Some(format!(
        "/api/v1/libraries/{library_id}/storage-roots/{storage_root_id}/files/{relative_path}"
    ))
}

fn append_storage_relative_path(root: &str, relative_path: &str) -> String {
    let trimmed_root = root.trim_end_matches(|character| character == '/' || character == '\\');
    if trimmed_root.starts_with("smb://")
        || trimmed_root.starts_with("s3://")
        || (trimmed_root.contains('/') && !trimmed_root.contains('\\'))
    {
        format!("{trimmed_root}/{}", relative_path.replace('\\', "/"))
    } else {
        format!("{trimmed_root}\\{}", relative_path.replace('/', "\\"))
    }
}

fn json_string_array(value: &Value) -> Vec<String> {
    value
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str())
                .map(str::trim)
                .filter(|item| !item.is_empty())
                .map(ToOwned::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

async fn count_assets(
    state: &AppState,
    library_id: &str,
    include_deleted: bool,
    deleted_only: bool,
) -> AppResult<i64> {
    Ok(sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM assets
        WHERE library_id = $1
          AND (($3 AND deleted_at IS NOT NULL) OR (NOT $3 AND ($2 OR deleted_at IS NULL)))
        "#,
    )
    .bind(library_id)
    .bind(include_deleted)
    .bind(deleted_only)
    .fetch_one(&state.pool)
    .await?)
}

pub(super) async fn query_assets_by_ids(
    state: &AppState,
    library_id: &str,
    asset_ids: &[String],
) -> AppResult<Vec<AssetRecord>> {
    if asset_ids.is_empty() {
        return Ok(Vec::new());
    }

    Ok(sqlx::query_as::<_, AssetRecord>(
        r#"
        SELECT
            id,
            library_id,
            name,
            asset_kind,
            import_mode,
            storage_key,
            storage_root_id,
            relative_path,
            metadata,
            created_by_user_id,
            imported_by_user_id,
            updated_by_user_id,
            deleted_by_user_id,
            restored_by_user_id,
            created_at,
            imported_at,
            updated_at,
            deleted_at,
            restored_at
        FROM assets
        WHERE library_id = $1 AND id = ANY($2)
        ORDER BY created_at DESC, id DESC
        "#,
    )
    .bind(library_id)
    .bind(asset_ids)
    .fetch_all(&state.pool)
    .await?)
}

async fn find_default_storage_root_id(
    state: &AppState,
    library_id: &str,
) -> AppResult<Option<Uuid>> {
    Ok(sqlx::query_scalar(
        r#"
        SELECT id
        FROM storage_roots
        WHERE library_id = $1 AND enabled = TRUE
        ORDER BY created_at ASC
        LIMIT 1
        "#,
    )
    .bind(library_id)
    .fetch_optional(&state.pool)
    .await?)
}

async fn ensure_storage_root_in_library(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    library_id: &str,
    root_id: Uuid,
) -> AppResult<()> {
    let exists: Option<Uuid> = sqlx::query_scalar(
        r#"
        SELECT id
        FROM storage_roots
        WHERE id = $1 AND library_id = $2 AND enabled = TRUE
        "#,
    )
    .bind(root_id)
    .bind(library_id)
    .fetch_optional(&mut **tx)
    .await?;

    exists
        .map(|_| ())
        .ok_or_else(|| AppError::BadRequest("workspace not found".to_string()))
}

async fn ensure_folder_in_library(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    library_id: &str,
    folder_id: &str,
) -> AppResult<()> {
    let exists: Option<String> = sqlx::query_scalar(
        r#"
        SELECT id
        FROM folders
        WHERE id = $1 AND library_id = $2
        "#,
    )
    .bind(folder_id)
    .bind(library_id)
    .fetch_optional(&mut **tx)
    .await?;

    exists
        .map(|_| ())
        .ok_or_else(|| AppError::BadRequest("folder not found".to_string()))
}

async fn write_asset_source_file(
    state: &AppState,
    root_id: Uuid,
    asset_id: &str,
    file: &ImportAssetFileRequest,
) -> AppResult<()> {
    let base_path = storage_root_write_base_path(state, root_id, None).await?;
    let relative_path = normalize_source_file_relative_path(&file.relative_path, asset_id)?;
    let decoded = decode_import_file("source", file, MAX_SOURCE_FILE_BYTES)?;
    let target_path = join_safe_relative_path(&base_path, &relative_path);
    write_file_atomic(&target_path, &decoded)
}

pub(super) async fn write_asset_derived_files(
    state: &AppState,
    root_id: Uuid,
    files: &[ImportAssetDerivedFileRequest],
) -> AppResult<()> {
    let base_path = storage_root_write_base_path(state, root_id, None).await?;
    for file in files {
        let relative_path = normalize_derived_file_relative_path(&file.file.relative_path)?;
        let decoded = decode_import_file(&file.kind, &file.file, MAX_DERIVED_FILE_BYTES)?;
        let target_path = join_safe_relative_path(&base_path, &relative_path);
        write_file_atomic(&target_path, &decoded)?;
    }

    Ok(())
}

pub(super) async fn storage_root_write_base_path(
    state: &AppState,
    root_id: Uuid,
    library_id: Option<&str>,
) -> AppResult<PathBuf> {
    let row = sqlx::query(
        r#"
        SELECT kind, canonical_uri, windows_unc_path
        FROM storage_roots
        WHERE id = $1 AND enabled = TRUE AND ($2::text IS NULL OR library_id = $2)
        "#,
    )
    .bind(root_id)
    .bind(library_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::BadRequest("workspace not found".to_string()))?;

    let kind_value: String = row.try_get("kind")?;
    let kind = kind_value
        .parse::<StorageRootKind>()
        .map_err(|error| AppError::BadRequest(error.to_string()))?;
    let canonical_uri: String = row.try_get("canonical_uri")?;
    let windows_unc_path: Option<String> = row.try_get("windows_unc_path")?;

    match kind {
        StorageRootKind::ServerFilesystem => Ok(PathBuf::from(canonical_uri)),
        StorageRootKind::Smb => {
            if cfg!(windows) {
                windows_unc_path.map(PathBuf::from).ok_or_else(|| {
                    AppError::BadRequest("shared workspace has no Windows path".to_string())
                })
            } else {
                Err(AppError::BadRequest(
                    "shared workspace file access is only available on a Windows server in this build"
                        .to_string(),
                ))
            }
        }
        StorageRootKind::S3 => Err(AppError::BadRequest(
            "object storage asset file access is not implemented yet".to_string(),
        )),
    }
}

fn decode_import_file(
    kind: &str,
    file: &ImportAssetFileRequest,
    max_size_bytes: usize,
) -> AppResult<Vec<u8>> {
    if let Some(content_type) = file.content_type.as_deref() {
        if content_type.len() > 128 {
            return Err(AppError::BadRequest(format!(
                "import file '{kind}' content type is invalid",
            )));
        }
    }

    let encoded = file
        .content_base64
        .split_once(',')
        .map(|(_, payload)| payload)
        .unwrap_or(file.content_base64.as_str())
        .trim();
    let decoded = general_purpose::STANDARD
        .decode(encoded)
        .map_err(|_| AppError::BadRequest(format!("import file '{kind}' is not valid base64")))?;

    if decoded.len() > max_size_bytes {
        return Err(AppError::BadRequest(format!(
            "import file '{kind}' exceeds the {} MB limit",
            max_size_bytes / 1024 / 1024
        )));
    }

    if let Some(expected_size) = file.size_bytes {
        if expected_size != decoded.len() as u64 {
            return Err(AppError::BadRequest(format!(
                "import file '{kind}' size does not match its payload",
            )));
        }
    }

    Ok(decoded)
}

fn normalize_source_file_relative_path(value: &str, asset_id: &str) -> AppResult<String> {
    let normalized = normalize_safe_relative_path(value)?;
    let expected_prefix = format!("assets/{asset_id}/");
    if !normalized.starts_with(&expected_prefix) {
        return Err(AppError::BadRequest(
            "source files must be stored under their asset directory".to_string(),
        ));
    }

    Ok(normalized)
}

pub(super) fn normalize_derived_file_relative_path(value: &str) -> AppResult<String> {
    let normalized = normalize_safe_relative_path(value)?;
    if !normalized.starts_with(".madlibrary/thumbs/")
        && !normalized.starts_with(".madlibrary/previews/")
    {
        return Err(AppError::BadRequest(
            "derived files must be stored under .madlibrary/thumbs or .madlibrary/previews"
                .to_string(),
        ));
    }

    Ok(normalized)
}

pub(super) fn normalize_readable_storage_file_relative_path(value: &str) -> AppResult<String> {
    let normalized = normalize_safe_relative_path(value)?;
    if normalized.starts_with("assets/")
        || normalized.starts_with(".madlibrary/thumbs/")
        || normalized.starts_with(".madlibrary/previews/")
        || normalized == ".madlibrary/cover.webp"
    {
        return Ok(normalized);
    }

    Err(AppError::BadRequest(
        "storage file path is outside readable asset areas".to_string(),
    ))
}

fn normalize_safe_relative_path(value: &str) -> AppResult<String> {
    let normalized = value
        .trim()
        .replace('\\', "/")
        .trim_start_matches('/')
        .to_string();
    if normalized.is_empty() || normalized.contains('\0') {
        return Err(AppError::BadRequest(
            "import file path is invalid".to_string(),
        ));
    }

    for component in StdPath::new(&normalized).components() {
        if !matches!(component, Component::Normal(_)) {
            return Err(AppError::BadRequest(
                "import file path is invalid".to_string(),
            ));
        }
    }

    Ok(normalized)
}

pub(super) fn join_safe_relative_path(base_path: &StdPath, relative_path: &str) -> PathBuf {
    relative_path
        .split('/')
        .fold(base_path.to_path_buf(), |path, segment| path.join(segment))
}

pub(super) fn write_file_atomic(target_path: &StdPath, bytes: &[u8]) -> AppResult<()> {
    let parent = target_path
        .parent()
        .ok_or_else(|| AppError::BadRequest("derived file path is invalid".to_string()))?;
    fs::create_dir_all(parent).map_err(|error| {
        AppError::BadRequest(format!("could not create derived file directory: {error}"))
    })?;

    let temporary_path = target_path.with_extension(format!(
        "{}.tmp",
        target_path
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("file")
    ));
    fs::write(&temporary_path, bytes)
        .map_err(|error| AppError::BadRequest(format!("could not write derived file: {error}")))?;
    fs::rename(&temporary_path, target_path).map_err(|error| {
        AppError::BadRequest(format!("could not finalize derived file: {error}"))
    })?;
    Ok(())
}

fn content_type_for_path(path: &str) -> &'static str {
    match path
        .rsplit('.')
        .next()
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "avif" => "image/avif",
        "gif" => "image/gif",
        "jpg" | "jpeg" => "image/jpeg",
        "json" => "application/json",
        "mp4" => "video/mp4",
        "png" => "image/png",
        "svg" => "image/svg+xml",
        "webp" => "image/webp",
        _ => "application/octet-stream",
    }
}

fn normalize_asset_id(asset_id: Option<String>) -> String {
    asset_id
        .map(|value| value.trim().to_string())
        .filter(|value| value.starts_with("asset_") && value.len() == "asset_".len() + 12)
        .unwrap_or_else(|| generate_id("asset_"))
}

fn normalize_import_mode(value: Option<&str>) -> AppResult<String> {
    match value.unwrap_or("reference").trim() {
        "copy" => Ok("copy".to_string()),
        "reference" | "" => Ok("reference".to_string()),
        _ => Err(AppError::BadRequest("import mode is invalid".to_string())),
    }
}

fn normalize_required_text(value: &str, label: &str) -> AppResult<String> {
    let normalized = value.trim().to_string();
    if normalized.is_empty() {
        return Err(AppError::BadRequest(format!("{label} is required")));
    }
    Ok(normalized)
}

fn normalize_optional_text(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn metadata_string(metadata: &Value, key: &str) -> Option<String> {
    metadata
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn ensure_metadata_field(metadata: &mut Value, key: &str, value: Value) {
    let Some(object) = metadata.as_object_mut() else {
        let mut object = serde_json::Map::new();
        object.insert(key.to_string(), value);
        *metadata = Value::Object(object);
        return;
    };
    object.entry(key.to_string()).or_insert(value);
}

fn map_asset_insert_error(error: sqlx::Error) -> AppError {
    if let sqlx::Error::Database(database_error) = &error {
        if database_error.is_unique_violation() {
            return AppError::Conflict("asset already exists".to_string());
        }
    }
    AppError::Database(error)
}
