use crate::{
    auth::AuthUser,
    error::{AppError, AppResult},
    ids::generate_id,
    routes::{
        access::{ensure_library_access, ensure_library_write_access},
        assets::{
            build_asset_file_url, build_asset_responses, join_safe_relative_path,
            query_assets_by_ids, storage_root_write_base_path, AssetResponse,
        },
    },
    state::AppState,
};
use axum::{
    extract::{Path, State},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::{FromRow, Postgres, Transaction};
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Component, Path as StdPath, PathBuf},
};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferAssetRequest {
    operation_id: String,
    source_library_id: String,
    asset_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferFolderRequest {
    operation_id: String,
    source_library_id: String,
    folder_id: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryTransferResponse {
    asset_count: usize,
    folder_count: usize,
    imported_asset_ids: Vec<String>,
    imported_folder_ids: Vec<String>,
}

#[derive(Clone, Debug, FromRow)]
struct SourceAsset {
    id: String,
    name: String,
    asset_kind: String,
    storage_root_id: Option<Uuid>,
    relative_path: Option<String>,
    metadata: Value,
}

#[derive(Clone, Debug, FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
struct SourceFolder {
    id: String,
    parent_id: Option<String>,
    name: String,
    description: String,
    icon: String,
    color: String,
    cover_asset_id: Option<String>,
    sort_order: i64,
}

#[derive(Clone, Debug, FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
struct ExportTagGroup {
    id: String,
    name: String,
    color: String,
    sort_order: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportAssetResponse {
    asset: ExportTransferAsset,
    tag_groups: Vec<ExportTagGroup>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportTransferAsset {
    #[serde(flatten)]
    asset: AssetResponse,
    package_files: Vec<ExportPackageFile>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportPackageFile {
    relative_path: String,
    url: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportFolderResponse {
    folders: Vec<SourceFolder>,
    assets: Vec<ExportTransferAsset>,
    folder_asset_links: Vec<(String, String)>,
    tag_groups: Vec<ExportTagGroup>,
}

#[derive(Clone, Debug, FromRow)]
struct SourceTag {
    group_id: Option<String>,
    name: String,
    color: Option<String>,
    starred: bool,
    sort_order: i64,
}

struct PreparedAssetCopy {
    source: SourceAsset,
    target_id: String,
    relative_path: String,
    metadata: Value,
    copied_paths: Vec<PathBuf>,
}

pub async fn transfer_asset(
    State(state): State<AppState>,
    user: AuthUser,
    Path(target_library_id): Path<String>,
    Json(request): Json<TransferAssetRequest>,
) -> AppResult<Json<LibraryTransferResponse>> {
    ensure_transfer_access(
        &state,
        &user,
        &request.source_library_id,
        &target_library_id,
    )
    .await?;
    let operation_id = normalize_operation_id(&request.operation_id)?;
    let mut tx = state.pool.begin().await?;
    lock_transfer_operation(&mut tx, &operation_id).await?;
    if let Some(response) = find_transfer_operation(
        &mut tx,
        &operation_id,
        user.id,
        &request.source_library_id,
        &target_library_id,
        "asset",
        &request.asset_id,
    )
    .await?
    {
        return Ok(Json(response));
    }

    let source = query_source_asset(&state, &request.source_library_id, &request.asset_id).await?;
    let target_root_id = require_target_storage_root(&state, &target_library_id).await?;
    let prepared = prepare_asset_copy(&state, source, target_root_id).await?;
    let copied_paths = prepared.copied_paths.clone();
    let target_id = prepared.target_id.clone();
    let response = LibraryTransferResponse {
        asset_count: 1,
        folder_count: 0,
        imported_asset_ids: vec![target_id.clone()],
        imported_folder_ids: Vec::new(),
    };
    let result = async {
        insert_prepared_asset(
            &mut tx,
            &user,
            &request.source_library_id,
            &target_library_id,
            target_root_id,
            &prepared,
        )
        .await?;
        insert_transfer_activity(
            &mut tx,
            user.id,
            &request.source_library_id,
            &target_library_id,
            &[target_id.clone()],
            &[],
        )
        .await?;
        lock_target_storage(&mut tx, &target_library_id).await?;
        insert_transfer_operation(
            &mut tx,
            &operation_id,
            user.id,
            &request.source_library_id,
            &target_library_id,
            "asset",
            &request.asset_id,
            &response,
        )
        .await?;
        tx.commit().await?;
        Ok::<(), AppError>(())
    }
    .await;

    if let Err(error) = result {
        cleanup_copied_paths(&copied_paths);
        return Err(error);
    }

    Ok(Json(response))
}

pub async fn transfer_folder(
    State(state): State<AppState>,
    user: AuthUser,
    Path(target_library_id): Path<String>,
    Json(request): Json<TransferFolderRequest>,
) -> AppResult<Json<LibraryTransferResponse>> {
    ensure_transfer_access(
        &state,
        &user,
        &request.source_library_id,
        &target_library_id,
    )
    .await?;
    let operation_id = normalize_operation_id(&request.operation_id)?;
    let mut tx = state.pool.begin().await?;
    lock_transfer_operation(&mut tx, &operation_id).await?;
    if let Some(response) = find_transfer_operation(
        &mut tx,
        &operation_id,
        user.id,
        &request.source_library_id,
        &target_library_id,
        "folder",
        &request.folder_id,
    )
    .await?
    {
        return Ok(Json(response));
    }

    let source_folders =
        query_folder_branch(&state, &request.source_library_id, &request.folder_id).await?;
    if source_folders.is_empty() {
        return Err(AppError::NotFound("folder not found".to_string()));
    }
    let source_folder_ids = source_folders
        .iter()
        .map(|folder| folder.id.clone())
        .collect::<Vec<_>>();
    let source_asset_ids = query_folder_asset_ids(&state, &source_folder_ids).await?;
    let source_assets =
        query_source_assets(&state, &request.source_library_id, &source_asset_ids).await?;
    let source_links = query_folder_asset_links(&state, &source_folder_ids).await?;
    let target_root_id = if source_assets.is_empty() {
        None
    } else {
        Some(require_target_storage_root(&state, &target_library_id).await?)
    };

    let mut prepared_assets = Vec::with_capacity(source_assets.len());
    let mut copied_paths = Vec::new();
    for source in source_assets {
        match prepare_asset_copy(
            &state,
            source,
            target_root_id.expect("storage root checked"),
        )
        .await
        {
            Ok(prepared) => {
                copied_paths.extend(prepared.copied_paths.iter().cloned());
                prepared_assets.push(prepared);
            }
            Err(error) => {
                cleanup_copied_paths(&copied_paths);
                return Err(error);
            }
        }
    }

    let result = async {
        let mut asset_id_map = HashMap::new();
        for prepared in &prepared_assets {
            insert_prepared_asset(
                &mut tx,
                &user,
                &request.source_library_id,
                &target_library_id,
                target_root_id.expect("storage root checked"),
                prepared,
            )
            .await?;
            asset_id_map.insert(prepared.source.id.clone(), prepared.target_id.clone());
        }

        let folder_id_map = insert_folder_branch(
            &mut tx,
            &user,
            &target_library_id,
            &request.folder_id,
            &source_folders,
            &source_links,
            &asset_id_map,
        )
        .await?;
        let imported_asset_ids = asset_id_map.values().cloned().collect::<Vec<_>>();
        let imported_folder_ids = folder_id_map.values().cloned().collect::<Vec<_>>();
        insert_transfer_activity(
            &mut tx,
            user.id,
            &request.source_library_id,
            &target_library_id,
            &imported_asset_ids,
            &imported_folder_ids,
        )
        .await?;
        if !imported_asset_ids.is_empty() {
            lock_target_storage(&mut tx, &target_library_id).await?;
        }
        let response = LibraryTransferResponse {
            asset_count: imported_asset_ids.len(),
            folder_count: imported_folder_ids.len(),
            imported_asset_ids: imported_asset_ids.clone(),
            imported_folder_ids: imported_folder_ids.clone(),
        };
        insert_transfer_operation(
            &mut tx,
            &operation_id,
            user.id,
            &request.source_library_id,
            &target_library_id,
            "folder",
            &request.folder_id,
            &response,
        )
        .await?;
        tx.commit().await?;
        Ok::<(Vec<String>, Vec<String>), AppError>((imported_asset_ids, imported_folder_ids))
    }
    .await;

    let (imported_asset_ids, imported_folder_ids) = match result {
        Ok(result) => result,
        Err(error) => {
            cleanup_copied_paths(&copied_paths);
            return Err(error);
        }
    };

    Ok(Json(LibraryTransferResponse {
        asset_count: imported_asset_ids.len(),
        folder_count: imported_folder_ids.len(),
        imported_asset_ids,
        imported_folder_ids,
    }))
}

pub async fn export_asset(
    State(state): State<AppState>,
    user: AuthUser,
    Path((library_id, asset_id)): Path<(String, String)>,
) -> AppResult<Json<ExportAssetResponse>> {
    ensure_library_access(&state, &user, &library_id).await?;
    let records = query_assets_by_ids(&state, &library_id, &[asset_id]).await?;
    let mut assets = build_asset_responses(&state, &library_id, user.id, records).await?;
    let asset = assets
        .pop()
        .ok_or_else(|| AppError::NotFound("asset not found".to_string()))?;
    if asset.asset.deleted_at.is_some() {
        return Err(AppError::NotFound("asset not found".to_string()));
    }
    let tag_groups =
        query_export_tag_groups(&state, &library_id, std::slice::from_ref(&asset)).await?;
    let asset = build_export_transfer_asset(asset);
    Ok(Json(ExportAssetResponse { asset, tag_groups }))
}

pub async fn export_folder(
    State(state): State<AppState>,
    user: AuthUser,
    Path((library_id, folder_id)): Path<(String, String)>,
) -> AppResult<Json<ExportFolderResponse>> {
    ensure_library_access(&state, &user, &library_id).await?;
    let folders = query_folder_branch(&state, &library_id, &folder_id).await?;
    if folders.is_empty() {
        return Err(AppError::NotFound("folder not found".to_string()));
    }
    let folder_ids = folders
        .iter()
        .map(|folder| folder.id.clone())
        .collect::<Vec<_>>();
    let asset_ids = query_folder_asset_ids(&state, &folder_ids).await?;
    let mut records = query_assets_by_ids(&state, &library_id, &asset_ids).await?;
    records.retain(|asset| asset.deleted_at.is_none());
    let assets = build_asset_responses(&state, &library_id, user.id, records).await?;
    let folder_asset_links = query_folder_asset_links(&state, &folder_ids).await?;
    let tag_groups = query_export_tag_groups(&state, &library_id, &assets).await?;
    let assets = assets
        .into_iter()
        .map(build_export_transfer_asset)
        .collect();
    Ok(Json(ExportFolderResponse {
        folders,
        assets,
        folder_asset_links,
        tag_groups,
    }))
}

fn build_export_transfer_asset(asset: AssetResponse) -> ExportTransferAsset {
    let package_files = match (
        asset.asset.storage_root_id,
        asset.asset.relative_path.as_deref(),
        asset
            .asset
            .metadata
            .get("package")
            .and_then(|value| value.get("files"))
            .and_then(Value::as_array),
    ) {
        (Some(root_id), Some(main_path), Some(files)) => {
            let main_relative_path = asset
                .asset
                .metadata
                .get("package")
                .and_then(|value| value.get("mainFile"))
                .and_then(|value| value.get("relativePath"))
                .and_then(Value::as_str)
                .map(|value| value.replace('\\', "/"));
            let base = main_relative_path
                .as_deref()
                .and_then(|relative_path| {
                    main_path
                        .replace('\\', "/")
                        .strip_suffix(relative_path)
                        .map(|value| value.trim_end_matches('/').to_string())
                })
                .or_else(|| {
                    asset
                        .asset
                        .metadata
                        .get("package")
                        .and_then(|value| value.get("storedDirectory"))
                        .and_then(Value::as_str)
                        .map(|value| value.replace('\\', "/"))
                })
                .or_else(|| {
                    StdPath::new(main_path)
                        .parent()
                        .map(|path| path.to_string_lossy().replace('\\', "/"))
                })
                .unwrap_or_default();
            files
                .iter()
                .filter_map(|entry| entry.get("relativePath"))
                .filter_map(Value::as_str)
                .filter_map(|relative_path| {
                    let storage_path = if base.is_empty() {
                        relative_path.replace('\\', "/")
                    } else {
                        format!("{base}/{}", relative_path.replace('\\', "/"))
                    };
                    build_asset_file_url(&asset.asset.library_id, root_id, &storage_path).map(
                        |url| ExportPackageFile {
                            relative_path: relative_path.replace('\\', "/"),
                            url,
                        },
                    )
                })
                .collect()
        }
        _ => Vec::new(),
    };
    ExportTransferAsset {
        asset,
        package_files,
    }
}

async fn query_export_tag_groups(
    state: &AppState,
    library_id: &str,
    assets: &[AssetResponse],
) -> AppResult<Vec<ExportTagGroup>> {
    let group_ids = assets
        .iter()
        .flat_map(|asset| asset.tags.iter())
        .filter_map(|tag| tag.group_id.clone())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    if group_ids.is_empty() {
        return Ok(Vec::new());
    }
    Ok(sqlx::query_as::<_, ExportTagGroup>(
        r#"
        SELECT id, name, color, sort_order
        FROM tag_groups
        WHERE library_id = $1 AND id = ANY($2)
        ORDER BY sort_order ASC, name ASC
        "#,
    )
    .bind(library_id)
    .bind(group_ids)
    .fetch_all(&state.pool)
    .await?)
}

async fn ensure_transfer_access(
    state: &AppState,
    user: &AuthUser,
    source_library_id: &str,
    target_library_id: &str,
) -> AppResult<()> {
    if source_library_id == target_library_id {
        return Err(AppError::BadRequest(
            "target library must be different from the source library".to_string(),
        ));
    }
    ensure_library_access(state, user, source_library_id).await?;
    ensure_library_write_access(state, user, target_library_id).await?;
    Ok(())
}

async fn query_source_asset(
    state: &AppState,
    library_id: &str,
    asset_id: &str,
) -> AppResult<SourceAsset> {
    sqlx::query_as::<_, SourceAsset>(
        r#"
        SELECT id, name, asset_kind, storage_root_id, relative_path, metadata
        FROM assets
        WHERE library_id = $1 AND id = $2 AND deleted_at IS NULL
        "#,
    )
    .bind(library_id)
    .bind(asset_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::NotFound("asset not found".to_string()))
}

async fn query_source_assets(
    state: &AppState,
    library_id: &str,
    asset_ids: &[String],
) -> AppResult<Vec<SourceAsset>> {
    if asset_ids.is_empty() {
        return Ok(Vec::new());
    }
    let assets = sqlx::query_as::<_, SourceAsset>(
        r#"
        SELECT id, name, asset_kind, storage_root_id, relative_path, metadata
        FROM assets
        WHERE library_id = $1 AND id = ANY($2) AND deleted_at IS NULL
        ORDER BY created_at ASC, id ASC
        "#,
    )
    .bind(library_id)
    .bind(asset_ids)
    .fetch_all(&state.pool)
    .await?;
    if assets.len() != asset_ids.len() {
        return Err(AppError::BadRequest(
            "folder contains an unavailable or trashed asset".to_string(),
        ));
    }
    Ok(assets)
}

async fn query_folder_branch(
    state: &AppState,
    library_id: &str,
    folder_id: &str,
) -> AppResult<Vec<SourceFolder>> {
    Ok(sqlx::query_as::<_, SourceFolder>(
        r#"
        WITH RECURSIVE folder_branch AS (
            SELECT id, library_id, parent_id, name, description, icon, color,
                   cover_asset_id, sort_order, 0 AS depth
            FROM folders
            WHERE library_id = $1 AND id = $2
            UNION ALL
            SELECT child.id, child.library_id, child.parent_id, child.name, child.description,
                   child.icon, child.color, child.cover_asset_id, child.sort_order,
                   parent.depth + 1
            FROM folders child
            JOIN folder_branch parent ON child.parent_id = parent.id
            WHERE child.library_id = $1
        )
        SELECT id, parent_id, name, description, icon, color, cover_asset_id, sort_order
        FROM folder_branch
        ORDER BY depth ASC, sort_order ASC, id ASC
        "#,
    )
    .bind(library_id)
    .bind(folder_id)
    .fetch_all(&state.pool)
    .await?)
}

async fn query_folder_asset_ids(state: &AppState, folder_ids: &[String]) -> AppResult<Vec<String>> {
    if folder_ids.is_empty() {
        return Ok(Vec::new());
    }
    Ok(sqlx::query_scalar::<_, String>(
        r#"
        SELECT DISTINCT asset_id
        FROM asset_folders
        WHERE folder_id = ANY($1)
        ORDER BY asset_id ASC
        "#,
    )
    .bind(folder_ids)
    .fetch_all(&state.pool)
    .await?)
}

async fn query_folder_asset_links(
    state: &AppState,
    folder_ids: &[String],
) -> AppResult<Vec<(String, String)>> {
    if folder_ids.is_empty() {
        return Ok(Vec::new());
    }
    Ok(sqlx::query_as::<_, (String, String)>(
        r#"
        SELECT asset_id, folder_id
        FROM asset_folders
        WHERE folder_id = ANY($1)
        ORDER BY created_at ASC
        "#,
    )
    .bind(folder_ids)
    .fetch_all(&state.pool)
    .await?)
}

async fn require_target_storage_root(state: &AppState, library_id: &str) -> AppResult<Uuid> {
    sqlx::query_scalar::<_, Uuid>(
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
    .await?
    .ok_or_else(|| AppError::BadRequest("target library has no enabled storage".to_string()))
}

async fn prepare_asset_copy(
    state: &AppState,
    source: SourceAsset,
    target_root_id: Uuid,
) -> AppResult<PreparedAssetCopy> {
    let source_root_id = source.storage_root_id.ok_or_else(|| {
        AppError::BadRequest(format!("asset '{}' has no readable storage", source.name))
    })?;
    let source_relative_path = source.relative_path.as_deref().ok_or_else(|| {
        AppError::BadRequest(format!("asset '{}' has no source file path", source.name))
    })?;
    let source_relative_path = normalize_safe_relative_path(source_relative_path)?;
    let source_base = storage_root_write_base_path(state, source_root_id, None).await?;
    let target_base = storage_root_write_base_path(state, target_root_id, None).await?;
    let target_id = generate_id("asset_");
    let extension = StdPath::new(&source_relative_path)
        .extension()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty());
    let target_file_name = extension
        .map(|extension| format!("{target_id}.{extension}"))
        .unwrap_or_else(|| target_id.clone());
    let target_relative_path = format!("assets/{target_id}/{target_file_name}");
    let mut copied_paths = Vec::new();
    copy_file(
        &join_safe_relative_path(&source_base, &source_relative_path),
        &join_safe_relative_path(&target_base, &target_relative_path),
    )?;
    copied_paths.push(join_safe_relative_path(&target_base, &target_relative_path));

    let mut metadata = source.metadata.clone();
    if !metadata.is_object() {
        metadata = json!({});
    }
    if let Some(object) = metadata.as_object_mut() {
        object.insert("sourcePath".to_string(), json!(target_relative_path));
        object.insert("storedPath".to_string(), json!(target_relative_path));
    }

    for key in [
        "thumbnailPath",
        "previewImagePath",
        "previewVideoPath",
        "waveformPath",
    ] {
        let Some(source_derived_path) = metadata
            .get(key)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
        else {
            continue;
        };
        let Some(target_derived_path) = target_derived_path(key, &source_derived_path, &target_id)
        else {
            if let Some(object) = metadata.as_object_mut() {
                object.remove(key);
            }
            continue;
        };
        let source_derived_path = normalize_safe_relative_path(&source_derived_path)?;
        let source_path = join_safe_relative_path(&source_base, &source_derived_path);
        if !source_path.is_file() {
            if let Some(object) = metadata.as_object_mut() {
                object.remove(key);
            }
            continue;
        }
        let target_path = join_safe_relative_path(&target_base, &target_derived_path);
        if let Err(error) = copy_file(&source_path, &target_path) {
            cleanup_copied_paths(&copied_paths);
            return Err(error);
        }
        copied_paths.push(target_path);
        if let Some(object) = metadata.as_object_mut() {
            object.insert(key.to_string(), json!(target_derived_path));
        }
    }

    Ok(PreparedAssetCopy {
        source,
        target_id,
        relative_path: target_relative_path,
        metadata,
        copied_paths,
    })
}

fn target_derived_path(key: &str, source_path: &str, target_id: &str) -> Option<String> {
    let extension = StdPath::new(source_path)
        .extension()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty());
    let kind_suffix = match key {
        "thumbnailPath" => "thumbnail",
        "previewImagePath" => "preview-image",
        "previewVideoPath" => "preview-video",
        "waveformPath" => "waveform",
        _ => return None,
    };
    let file_name = extension
        .map(|extension| format!("{target_id}-{kind_suffix}.{extension}"))
        .unwrap_or_else(|| format!("{target_id}-{kind_suffix}"));
    match key {
        "thumbnailPath" => Some(format!(".madlibrary/thumbs/{file_name}")),
        "previewImagePath" | "previewVideoPath" | "waveformPath" => {
            Some(format!(".madlibrary/previews/{target_id}/{file_name}"))
        }
        _ => None,
    }
}

fn normalize_safe_relative_path(value: &str) -> AppResult<String> {
    let normalized = value
        .trim()
        .replace('\\', "/")
        .trim_start_matches('/')
        .to_string();
    if normalized.is_empty() || normalized.contains('\0') {
        return Err(AppError::BadRequest(
            "asset file path is invalid".to_string(),
        ));
    }
    for component in StdPath::new(&normalized).components() {
        if !matches!(component, Component::Normal(_)) {
            return Err(AppError::BadRequest(
                "asset file path is invalid".to_string(),
            ));
        }
    }
    Ok(normalized)
}

fn normalize_operation_id(value: &str) -> AppResult<String> {
    let value = value.trim();
    if value.is_empty() || value.len() > 128 {
        return Err(AppError::BadRequest(
            "transfer operation id is invalid".to_string(),
        ));
    }
    Ok(value.to_string())
}

async fn lock_transfer_operation(
    tx: &mut Transaction<'_, Postgres>,
    operation_id: &str,
) -> AppResult<()> {
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
        .bind(operation_id)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn find_transfer_operation(
    tx: &mut Transaction<'_, Postgres>,
    operation_id: &str,
    user_id: Uuid,
    source_library_id: &str,
    target_library_id: &str,
    item_kind: &str,
    source_item_id: &str,
) -> AppResult<Option<LibraryTransferResponse>> {
    let record = sqlx::query_as::<_, (Uuid, String, String, String, String, Value)>(
        r#"
        SELECT user_id, source_library_id, target_library_id, item_kind, source_item_id, response
        FROM library_transfer_operations
        WHERE operation_id = $1
        "#,
    )
    .bind(operation_id)
    .fetch_optional(&mut **tx)
    .await?;
    let Some((
        record_user_id,
        record_source_id,
        record_target_id,
        record_kind,
        record_item_id,
        response,
    )) = record
    else {
        return Ok(None);
    };
    if record_user_id != user_id
        || record_source_id != source_library_id
        || record_target_id != target_library_id
        || record_kind != item_kind
        || record_item_id != source_item_id
    {
        return Err(AppError::Conflict(
            "transfer operation id was already used for another item".to_string(),
        ));
    }
    Ok(Some(serde_json::from_value(response)?))
}

#[allow(clippy::too_many_arguments)]
async fn insert_transfer_operation(
    tx: &mut Transaction<'_, Postgres>,
    operation_id: &str,
    user_id: Uuid,
    source_library_id: &str,
    target_library_id: &str,
    item_kind: &str,
    source_item_id: &str,
    response: &LibraryTransferResponse,
) -> AppResult<()> {
    sqlx::query(
        r#"
        INSERT INTO library_transfer_operations (
            operation_id, source_library_id, target_library_id, user_id,
            item_kind, source_item_id, response
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7::jsonb)
        "#,
    )
    .bind(operation_id)
    .bind(source_library_id)
    .bind(target_library_id)
    .bind(user_id)
    .bind(item_kind)
    .bind(source_item_id)
    .bind(serde_json::to_value(response)?)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

fn copy_file(source: &StdPath, target: &StdPath) -> AppResult<()> {
    if !source.is_file() {
        return Err(AppError::BadRequest(format!(
            "source file is missing: {}",
            source.display()
        )));
    }
    let parent = target
        .parent()
        .ok_or_else(|| AppError::BadRequest("target file path is invalid".to_string()))?;
    fs::create_dir_all(parent).map_err(|error| {
        AppError::BadRequest(format!("could not create target directory: {error}"))
    })?;
    fs::copy(source, target)
        .map(|_| ())
        .map_err(|error| AppError::BadRequest(format!("could not copy asset file: {error}")))
}

async fn insert_prepared_asset(
    tx: &mut Transaction<'_, Postgres>,
    user: &AuthUser,
    source_library_id: &str,
    target_library_id: &str,
    target_root_id: Uuid,
    prepared: &PreparedAssetCopy,
) -> AppResult<()> {
    sqlx::query(
        r#"
        INSERT INTO assets (
            id, library_id, name, asset_kind, import_mode, storage_key, storage_root_id,
            relative_path, metadata, created_by_user_id, imported_by_user_id,
            updated_by_user_id, imported_at
        )
        VALUES ($1, $2, $3, $4, 'copy', $5, $6, $5, $7, $8, $8, $8, NOW())
        "#,
    )
    .bind(&prepared.target_id)
    .bind(target_library_id)
    .bind(&prepared.source.name)
    .bind(&prepared.source.asset_kind)
    .bind(&prepared.relative_path)
    .bind(target_root_id)
    .bind(&prepared.metadata)
    .bind(user.id)
    .execute(&mut **tx)
    .await?;

    copy_asset_tags(
        tx,
        user.id,
        source_library_id,
        target_library_id,
        &prepared.source.id,
        &prepared.target_id,
    )
    .await
}

async fn copy_asset_tags(
    tx: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
    source_library_id: &str,
    target_library_id: &str,
    source_asset_id: &str,
    target_asset_id: &str,
) -> AppResult<()> {
    let tags = sqlx::query_as::<_, SourceTag>(
        r#"
        SELECT t.group_id, t.name, t.color, t.starred, t.sort_order
        FROM tags t
        JOIN asset_tags relation ON relation.tag_id = t.id
        WHERE t.library_id = $1 AND relation.asset_id = $2
        ORDER BY t.sort_order ASC, t.name ASC
        "#,
    )
    .bind(source_library_id)
    .bind(source_asset_id)
    .fetch_all(&mut **tx)
    .await?;

    for tag in tags {
        let target_group_id = match tag.group_id.as_deref() {
            Some(source_group_id) => {
                upsert_target_tag_group(
                    tx,
                    user_id,
                    source_library_id,
                    target_library_id,
                    source_group_id,
                )
                .await?
            }
            None => None,
        };
        let existing_tag_id = sqlx::query_scalar::<_, String>(
            r#"
            SELECT id FROM tags
            WHERE library_id = $1 AND name = $2 AND group_id IS NOT DISTINCT FROM $3
            ORDER BY created_at ASC LIMIT 1
            "#,
        )
        .bind(target_library_id)
        .bind(&tag.name)
        .bind(target_group_id.as_deref())
        .fetch_optional(&mut **tx)
        .await?;
        let target_tag_id = match existing_tag_id {
            Some(id) => id,
            None => {
                let id = generate_id("tag_");
                sqlx::query(
                    r#"
                    INSERT INTO tags (
                        id, library_id, group_id, name, color, starred, sort_order,
                        created_by_user_id, updated_by_user_id
                    )
                    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $8)
                    "#,
                )
                .bind(&id)
                .bind(target_library_id)
                .bind(target_group_id)
                .bind(&tag.name)
                .bind(&tag.color)
                .bind(tag.starred)
                .bind(tag.sort_order)
                .bind(user_id)
                .execute(&mut **tx)
                .await?;
                id
            }
        };
        sqlx::query(
            "INSERT INTO asset_tags (asset_id, tag_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
        )
        .bind(target_asset_id)
        .bind(target_tag_id)
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}

async fn upsert_target_tag_group(
    tx: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
    source_library_id: &str,
    target_library_id: &str,
    source_group_id: &str,
) -> AppResult<Option<String>> {
    let source_group = sqlx::query_as::<_, (String, String, i64)>(
        "SELECT name, color, sort_order FROM tag_groups WHERE library_id = $1 AND id = $2",
    )
    .bind(source_library_id)
    .bind(source_group_id)
    .fetch_optional(&mut **tx)
    .await?;
    let Some((name, color, sort_order)) = source_group else {
        return Ok(None);
    };
    if let Some(id) = sqlx::query_scalar::<_, String>(
        "SELECT id FROM tag_groups WHERE library_id = $1 AND name = $2 ORDER BY created_at ASC LIMIT 1",
    )
    .bind(target_library_id)
    .bind(&name)
    .fetch_optional(&mut **tx)
    .await?
    {
        return Ok(Some(id));
    }
    let id = generate_id("tag_group_");
    sqlx::query(
        r#"
        INSERT INTO tag_groups (
            id, library_id, name, color, sort_order, created_by_user_id, updated_by_user_id
        )
        VALUES ($1, $2, $3, $4, $5, $6, $6)
        "#,
    )
    .bind(&id)
    .bind(target_library_id)
    .bind(name)
    .bind(color)
    .bind(sort_order)
    .bind(user_id)
    .execute(&mut **tx)
    .await?;
    Ok(Some(id))
}

async fn insert_folder_branch(
    tx: &mut Transaction<'_, Postgres>,
    user: &AuthUser,
    target_library_id: &str,
    source_root_folder_id: &str,
    source_folders: &[SourceFolder],
    source_links: &[(String, String)],
    asset_id_map: &HashMap<String, String>,
) -> AppResult<HashMap<String, String>> {
    let mut folder_id_map = HashMap::new();
    for folder in source_folders {
        let target_parent_id = if folder.id == source_root_folder_id {
            None
        } else {
            folder
                .parent_id
                .as_ref()
                .and_then(|parent_id| folder_id_map.get(parent_id))
                .cloned()
        };
        if folder.id != source_root_folder_id && target_parent_id.is_none() {
            return Err(AppError::BadRequest(
                "folder hierarchy could not be rebuilt".to_string(),
            ));
        }
        let target_id = generate_id("folder_");
        let target_cover_asset_id = folder
            .cover_asset_id
            .as_ref()
            .and_then(|asset_id| asset_id_map.get(asset_id));
        sqlx::query(
            r#"
            INSERT INTO folders (
                id, library_id, parent_id, name, description, icon, color, cover_asset_id,
                smart_import_id, sort_order, created_by_user_id, updated_by_user_id
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NULL, $9, $10, $10)
            "#,
        )
        .bind(&target_id)
        .bind(target_library_id)
        .bind(target_parent_id)
        .bind(&folder.name)
        .bind(&folder.description)
        .bind(&folder.icon)
        .bind(&folder.color)
        .bind(target_cover_asset_id)
        .bind(folder.sort_order)
        .bind(user.id)
        .execute(&mut **tx)
        .await?;
        folder_id_map.insert(folder.id.clone(), target_id);
    }

    for (source_asset_id, source_folder_id) in source_links {
        let (Some(target_asset_id), Some(target_folder_id)) = (
            asset_id_map.get(source_asset_id),
            folder_id_map.get(source_folder_id),
        ) else {
            continue;
        };
        sqlx::query(
            "INSERT INTO asset_folders (asset_id, folder_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
        )
        .bind(target_asset_id)
        .bind(target_folder_id)
        .execute(&mut **tx)
        .await?;
    }
    Ok(folder_id_map)
}

async fn insert_transfer_activity(
    tx: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
    source_library_id: &str,
    target_library_id: &str,
    asset_ids: &[String],
    folder_ids: &[String],
) -> AppResult<()> {
    sqlx::query(
        r#"
        INSERT INTO activity_log (
            id, library_id, actor_user_id, action, target_type, details
        )
        VALUES ($1, $2, $3, 'library.transfer', 'library', $4::jsonb)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(target_library_id)
    .bind(user_id)
    .bind(json!({
        "sourceLibraryId": source_library_id,
        "assetIds": asset_ids,
        "folderIds": folder_ids,
        "assetCount": asset_ids.len(),
        "folderCount": folder_ids.len(),
    }))
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn lock_target_storage(
    tx: &mut Transaction<'_, Postgres>,
    target_library_id: &str,
) -> AppResult<()> {
    sqlx::query(
        "UPDATE libraries SET storage_locked_at = COALESCE(storage_locked_at, NOW()) WHERE id = $1",
    )
    .bind(target_library_id)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

fn cleanup_copied_paths(paths: &[PathBuf]) {
    let mut parent_dirs = HashSet::new();
    for path in paths.iter().rev() {
        let _ = fs::remove_file(path);
        if let Some(parent) = path.parent() {
            parent_dirs.insert(parent.to_path_buf());
        }
    }
    let mut parent_dirs = parent_dirs.into_iter().collect::<Vec<_>>();
    parent_dirs.sort_by_key(|path| std::cmp::Reverse(path.components().count()));
    for path in parent_dirs {
        let _ = fs::remove_dir(path);
    }
}
