use super::{
    build_asset_responses, join_safe_relative_path, normalize_readable_storage_file_relative_path,
    query_assets_by_ids, storage_root_write_base_path, AssetResponse,
};
use crate::{
    auth::AuthUser,
    error::{AppError, AppResult},
    routes::access::{ensure_library_access, ensure_library_asset_mutation_access},
    state::AppState,
};
use axum::{
    extract::{Path, State},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::{Postgres, Row, Transaction};
use std::{
    collections::HashSet,
    fs,
    path::{Path as StdPath, PathBuf},
};
use uuid::Uuid;

const MAX_MUTATION_ASSETS: usize = 500;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdateAssetRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub url: Option<String>,
    pub rating: Option<Option<i64>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdateAssetsRatingRequest {
    pub asset_ids: Vec<String>,
    pub rating: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdateAssetsStarredRequest {
    pub asset_ids: Vec<String>,
    pub starred: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdateAssetsViewerRequest {
    pub asset_ids: Vec<String>,
    pub viewer: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ConvertAssetsImportModeRequest {
    pub asset_ids: Vec<String>,
    pub target_mode: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SetAssetFoldersRequest {
    pub asset_ids: Vec<String>,
    #[serde(default = "default_relation_mode")]
    pub mode: String,
    pub source_folder_id: Option<String>,
    pub folder_ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SetAssetTagsRequest {
    pub asset_ids: Vec<String>,
    #[serde(default = "default_relation_mode")]
    pub mode: String,
    pub tag_ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AssetIdsRequest {
    pub asset_ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CheckAssetMutationAccessRequest {
    #[serde(default)]
    pub asset_ids: Vec<String>,
    #[serde(default)]
    pub folder_ids: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckAssetMutationAccessResponse {
    pub allowed: bool,
    pub total_count: usize,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdateAssetDerivedFilesRequest {
    #[serde(default)]
    pub derived_files: Vec<super::ImportAssetDerivedFileRequest>,
    pub metadata_patch: Value,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetMutationResponse {
    pub items: Vec<AssetResponse>,
    pub asset_ids: Vec<String>,
    pub affected_count: usize,
    pub total: i64,
}

#[derive(Debug)]
struct RenamePlan {
    old_path: PathBuf,
    new_path: PathBuf,
    relative_path: String,
}

#[derive(Debug)]
struct ImportModeConversionPlan {
    asset_id: String,
    previous_relative_path: Option<String>,
    previous_package_root_relative_path: Option<String>,
    next_relative_path: String,
    next_storage_key: Option<String>,
    storage_action: ImportModeConversionStorageAction,
    metadata: Value,
}

#[derive(Debug)]
enum ImportModeConversionStorageAction {
    None,
    CopyFile {
        source_path: PathBuf,
        target_path: PathBuf,
    },
    CopyPackage {
        source_root: PathBuf,
        target_root: PathBuf,
    },
}

#[derive(Debug)]
enum ImportModeConversionRollback {
    File(PathBuf),
    PackageCopy {
        source_root: PathBuf,
        target_root: PathBuf,
    },
}

pub async fn check_asset_mutation_access(
    State(state): State<AppState>,
    user: AuthUser,
    Path(library_id): Path<String>,
    Json(request): Json<CheckAssetMutationAccessRequest>,
) -> AppResult<Json<CheckAssetMutationAccessResponse>> {
    ensure_library_access(&state, &user, &library_id).await?;
    let mut asset_ids = normalize_ids_unbounded(request.asset_ids);
    let folder_ids = normalize_ids_unbounded(request.folder_ids);
    if !folder_ids.is_empty() {
        let folder_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM folders WHERE library_id = $1 AND id = ANY($2)",
        )
        .bind(&library_id)
        .bind(&folder_ids)
        .fetch_one(&state.pool)
        .await?;
        if folder_count != folder_ids.len() as i64 {
            return Err(AppError::BadRequest(
                "one or more folders were not found".to_string(),
            ));
        }

        let folder_asset_ids: Vec<String> = sqlx::query_scalar(
            r#"
            WITH RECURSIVE folder_branch AS (
                SELECT id
                FROM folders
                WHERE library_id = $1 AND id = ANY($2)
                UNION
                SELECT child.id
                FROM folders child
                INNER JOIN folder_branch parent ON child.parent_id = parent.id
                WHERE child.library_id = $1
            )
            SELECT DISTINCT relation.asset_id
            FROM asset_folders relation
            INNER JOIN folder_branch folder ON folder.id = relation.folder_id
            INNER JOIN assets asset ON asset.id = relation.asset_id
            WHERE asset.library_id = $1 AND asset.deleted_at IS NULL
            "#,
        )
        .bind(&library_id)
        .bind(&folder_ids)
        .fetch_all(&state.pool)
        .await?;
        asset_ids.extend(folder_asset_ids);
        asset_ids = normalize_ids_unbounded(asset_ids);
    }

    ensure_library_asset_mutation_access(&state, &user, &library_id, &asset_ids).await?;
    Ok(Json(CheckAssetMutationAccessResponse {
        allowed: true,
        total_count: asset_ids.len(),
    }))
}

pub async fn update_asset(
    State(state): State<AppState>,
    user: AuthUser,
    Path((library_id, asset_id)): Path<(String, String)>,
    Json(request): Json<UpdateAssetRequest>,
) -> AppResult<Json<AssetMutationResponse>> {
    if let Some(Some(rating)) = request.rating {
        validate_rating(rating)?;
    }

    let current = sqlx::query(
        r#"
        SELECT name, asset_kind, import_mode, storage_root_id, relative_path, storage_key, metadata
        FROM assets
        WHERE library_id = $1 AND id = $2 AND deleted_at IS NULL
        "#,
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

    let current_name: String = current.try_get("name")?;
    let asset_kind: String = current.try_get("asset_kind")?;
    let import_mode: String = current.try_get("import_mode")?;
    let storage_root_id: Option<Uuid> = current.try_get("storage_root_id")?;
    let current_relative_path: Option<String> = current.try_get("relative_path")?;
    let current_storage_key: Option<String> = current.try_get("storage_key")?;
    let mut metadata: Value = current.try_get("metadata")?;

    let next_name = request
        .name
        .as_deref()
        .map(validate_asset_name)
        .transpose()?
        .unwrap_or_else(|| current_name.clone());
    let mut next_relative_path = current_relative_path.clone();
    let mut next_storage_key = current_storage_key.clone();
    let rename_plan = if next_name != current_name && asset_kind != "link" {
        if import_mode == "reference" {
            return Err(AppError::BadRequest(
                "referenced assets cannot be renamed".to_string(),
            ));
        }
        let root_id = storage_root_id
            .ok_or_else(|| AppError::BadRequest("asset has no workspace".to_string()))?;
        let current_path = current_relative_path
            .as_deref()
            .ok_or_else(|| AppError::BadRequest("asset has no source path".to_string()))?;
        let normalized = normalize_readable_storage_file_relative_path(current_path)?;
        let parent = normalized
            .rsplit_once('/')
            .map(|(parent, _)| parent)
            .unwrap_or("");
        let relative_path = if parent.is_empty() {
            next_name.clone()
        } else {
            format!("{parent}/{next_name}")
        };
        let base_path = storage_root_write_base_path(&state, root_id, Some(&library_id)).await?;
        let old_path = join_safe_relative_path(&base_path, &normalized);
        let new_path = join_safe_relative_path(&base_path, &relative_path);
        if new_path.exists() {
            return Err(AppError::Conflict(
                "an asset file with this name already exists".to_string(),
            ));
        }
        next_relative_path = Some(relative_path.clone());
        next_storage_key = Some(relative_path.clone());
        Some(RenamePlan {
            old_path,
            new_path,
            relative_path,
        })
    } else {
        None
    };

    let metadata_object = metadata
        .as_object_mut()
        .ok_or_else(|| AppError::BadRequest("asset metadata must be an object".to_string()))?;
    metadata_object.insert("name".to_string(), json!(next_name));
    if let Some(description) = request.description {
        metadata_object.insert("description".to_string(), json!(description.trim()));
    }
    if let Some(url) = request.url {
        metadata_object.insert("url".to_string(), json!(url.trim()));
    }
    if let Some(rating) = request.rating {
        metadata_object.insert("rating".to_string(), json!(rating));
    }
    if let Some(plan) = rename_plan.as_ref() {
        metadata_object.insert("sourcePath".to_string(), json!(plan.relative_path));
        metadata_object.insert("storedPath".to_string(), json!(plan.relative_path));
        if let Some(extension) = next_name
            .rsplit_once('.')
            .map(|(_, extension)| extension.to_ascii_lowercase())
        {
            metadata_object.insert("extension".to_string(), json!(extension));
        }
    }

    if let Some(plan) = rename_plan.as_ref() {
        fs::rename(&plan.old_path, &plan.new_path).map_err(|error| {
            AppError::BadRequest(format!("could not rename asset file: {error}"))
        })?;
    }

    let update_result = sqlx::query(
        r#"
        UPDATE assets
        SET name = $3,
            relative_path = $4,
            storage_key = $5,
            metadata = $6,
            updated_by_user_id = $7,
            updated_at = NOW()
        WHERE library_id = $1 AND id = $2 AND deleted_at IS NULL
        "#,
    )
    .bind(&library_id)
    .bind(&asset_id)
    .bind(&next_name)
    .bind(&next_relative_path)
    .bind(&next_storage_key)
    .bind(&metadata)
    .bind(user.id)
    .execute(&state.pool)
    .await;

    if let Err(error) = update_result {
        if let Some(plan) = rename_plan.as_ref() {
            let _ = fs::rename(&plan.new_path, &plan.old_path);
        }
        return Err(error.into());
    }

    insert_activity(
        &state,
        &library_id,
        user.id,
        "asset.updated",
        &[asset_id.clone()],
    )
    .await?;
    Ok(Json(
        mutation_response(&state, &library_id, user.id, vec![asset_id]).await?,
    ))
}

pub async fn update_asset_derived_files(
    State(state): State<AppState>,
    user: AuthUser,
    Path((library_id, asset_id)): Path<(String, String)>,
    Json(request): Json<UpdateAssetDerivedFilesRequest>,
) -> AppResult<Json<AssetMutationResponse>> {
    let metadata_patch = validate_derived_metadata_patch(request.metadata_patch, &asset_id)?;
    let storage_root_id: Option<Uuid> = sqlx::query_scalar(
        "SELECT storage_root_id FROM assets WHERE library_id = $1 AND id = $2 AND deleted_at IS NULL",
    )
    .bind(&library_id)
    .bind(&asset_id)
    .fetch_optional(&state.pool)
    .await?
    .flatten();
    let storage_root_id = storage_root_id.ok_or_else(|| {
        AppError::BadRequest("asset does not have an enabled workspace".to_string())
    })?;
    ensure_library_asset_mutation_access(
        &state,
        &user,
        &library_id,
        std::slice::from_ref(&asset_id),
    )
    .await?;
    if !request.derived_files.is_empty() {
        super::write_asset_derived_files(&state, storage_root_id, &request.derived_files).await?;
    }

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
    .bind(&library_id)
    .bind(&asset_id)
    .bind(metadata_patch)
    .bind(user.id)
    .fetch_optional(&mut *tx)
    .await?;
    if updated_id.is_none() {
        return Err(AppError::NotFound("asset not found".to_string()));
    }
    insert_activity_tx(
        &mut tx,
        &library_id,
        user.id,
        "assets.derived_files_updated",
        std::slice::from_ref(&asset_id),
    )
    .await?;
    tx.commit().await?;
    Ok(Json(
        mutation_response(&state, &library_id, user.id, vec![asset_id]).await?,
    ))
}

pub async fn update_assets_rating(
    State(state): State<AppState>,
    user: AuthUser,
    Path(library_id): Path<String>,
    Json(request): Json<UpdateAssetsRatingRequest>,
) -> AppResult<Json<AssetMutationResponse>> {
    if let Some(rating) = request.rating {
        validate_rating(rating)?;
    }
    mutate_metadata_field(
        &state,
        &user,
        &library_id,
        request.asset_ids,
        "rating",
        json!(request.rating),
        "assets.rating_updated",
    )
    .await
}

pub async fn update_assets_starred(
    State(state): State<AppState>,
    user: AuthUser,
    Path(library_id): Path<String>,
    Json(request): Json<UpdateAssetsStarredRequest>,
) -> AppResult<Json<AssetMutationResponse>> {
    ensure_library_access(&state, &user, &library_id).await?;
    let asset_ids = normalize_ids(request.asset_ids, "assets")?;
    let mut tx = state.pool.begin().await?;
    ensure_assets_in_library(&mut tx, &library_id, &asset_ids, false).await?;

    if request.starred {
        sqlx::query(
            r#"
            INSERT INTO asset_favorites (library_id, asset_id, user_id)
            SELECT $1, asset_id, $3
            FROM UNNEST($2::text[]) AS ids(asset_id)
            ON CONFLICT DO NOTHING
            "#,
        )
        .bind(&library_id)
        .bind(&asset_ids)
        .bind(user.id)
        .execute(&mut *tx)
        .await?;
    } else {
        sqlx::query(
            "DELETE FROM asset_favorites WHERE library_id = $1 AND asset_id = ANY($2) AND user_id = $3",
        )
        .bind(&library_id)
        .bind(&asset_ids)
        .bind(user.id)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(Json(
        mutation_response(&state, &library_id, user.id, asset_ids).await?,
    ))
}

pub async fn update_assets_viewer(
    State(state): State<AppState>,
    user: AuthUser,
    Path(library_id): Path<String>,
    Json(request): Json<UpdateAssetsViewerRequest>,
) -> AppResult<Json<AssetMutationResponse>> {
    let viewer = request
        .viewer
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty() && value != "auto");
    mutate_metadata_field(
        &state,
        &user,
        &library_id,
        request.asset_ids,
        "viewer",
        json!(viewer),
        "assets.viewer_updated",
    )
    .await
}

pub async fn convert_assets_import_mode(
    State(state): State<AppState>,
    user: AuthUser,
    Path(library_id): Path<String>,
    Json(request): Json<ConvertAssetsImportModeRequest>,
) -> AppResult<Json<AssetMutationResponse>> {
    let target_mode = normalize_conversion_target_mode(&request.target_mode)?;
    let asset_ids = normalize_ids(request.asset_ids, "assets")?;
    ensure_library_asset_mutation_access(&state, &user, &library_id, &asset_ids).await?;

    let rows = sqlx::query(
        r#"
        SELECT id, name, asset_kind, import_mode, storage_root_id, relative_path, storage_key, metadata
        FROM assets
        WHERE library_id = $1 AND id = ANY($2) AND deleted_at IS NULL
        "#,
    )
    .bind(&library_id)
    .bind(&asset_ids)
    .fetch_all(&state.pool)
    .await?;
    if rows.len() != asset_ids.len() {
        return Err(AppError::BadRequest(
            "one or more assets were not found".to_string(),
        ));
    }

    let mut plans = Vec::new();
    for row in rows {
        let asset_id: String = row.try_get("id")?;
        let import_mode: String = row.try_get("import_mode")?;
        if import_mode == target_mode {
            continue;
        }
        if import_mode != "copy" && import_mode != "reference" {
            return Err(AppError::BadRequest(format!(
                "{asset_id} has an unsupported import mode"
            )));
        }

        let asset_kind: String = row.try_get("asset_kind")?;
        let metadata: Value = row.try_get("metadata")?;
        validate_convertible_asset_kind(&asset_id, &asset_kind, &metadata)?;

        let storage_root_id: Uuid = row
            .try_get::<Option<Uuid>, _>("storage_root_id")?
            .ok_or_else(|| AppError::BadRequest(format!("{asset_id} has no workspace")))?;
        let base_path =
            storage_root_write_base_path(&state, storage_root_id, Some(&library_id)).await?;
        let current_relative_path = row.try_get::<Option<String>, _>("relative_path")?;
        let current_relative_path = current_relative_path
            .as_deref()
            .map(normalize_readable_storage_file_relative_path)
            .transpose()?;
        let mut next_metadata = metadata;
        let previous_package_root_relative_path = if asset_kind == "package" {
            let main_relative_path =
                metadata_package_main_relative_path(&next_metadata, &asset_id)?;
            current_relative_path
                .as_deref()
                .and_then(|value| {
                    package_root_relative_path_from_main_path(value, &main_relative_path)
                })
                .or_else(|| metadata_package_string(&next_metadata, "storedDirectory"))
        } else {
            None
        };
        let (next_relative_path, next_storage_key, storage_action) = if asset_kind == "package" {
            let main_relative_path =
                metadata_package_main_relative_path(&next_metadata, &asset_id)?;
            let source_relative_path = current_relative_path
                .clone()
                .or_else(|| {
                    row.try_get::<Option<String>, _>("storage_key")
                        .ok()
                        .flatten()
                })
                .ok_or_else(|| AppError::BadRequest(format!("{asset_id} has no source path")))?;
            let normalized_source_relative_path =
                normalize_readable_storage_file_relative_path(&source_relative_path)?;
            let source_root_relative_path = package_root_relative_path(
                &next_metadata,
                &normalized_source_relative_path,
                &main_relative_path,
            )?;
            let source_root = join_safe_relative_path(&base_path, &source_root_relative_path);
            let source_main = source_root.join(&main_relative_path);
            if !source_main.is_file() {
                return Err(AppError::BadRequest(format!(
                    "{asset_id} package main file was not found"
                )));
            }
            if target_mode == "copy" {
                let target_root_relative_path =
                    normalize_readable_storage_file_relative_path(&format!("assets/{asset_id}"))?;
                let target_relative_path = normalize_readable_storage_file_relative_path(
                    &format!("{target_root_relative_path}/{main_relative_path}"),
                )?;
                let target_root = join_safe_relative_path(&base_path, &target_root_relative_path);
                update_team_package_import_mode_metadata(
                    &mut next_metadata,
                    &asset_id,
                    "copy",
                    &source_root_relative_path,
                )?;
                (
                    target_relative_path.clone(),
                    Some(target_relative_path),
                    ImportModeConversionStorageAction::CopyPackage {
                        source_root,
                        target_root,
                    },
                )
            } else {
                update_team_package_import_mode_metadata(
                    &mut next_metadata,
                    &asset_id,
                    "reference",
                    &source_root_relative_path,
                )?;
                (
                    normalized_source_relative_path,
                    None,
                    ImportModeConversionStorageAction::None,
                )
            }
        } else if target_mode == "copy" {
            let source_relative_path = current_relative_path
                .as_deref()
                .ok_or_else(|| AppError::BadRequest(format!("{asset_id} has no source path")))?;
            let source_path = join_safe_relative_path(&base_path, source_relative_path);
            if !source_path.is_file() {
                return Err(AppError::BadRequest(format!(
                    "{asset_id} source file was not found"
                )));
            }
            let name: String = row.try_get("name")?;
            let target_relative_path = normalize_readable_storage_file_relative_path(&format!(
                "assets/{asset_id}/{}",
                validate_asset_name(&name)?
            ))?;
            let target_path = join_safe_relative_path(&base_path, &target_relative_path);
            if source_path != target_path {
                if let Some(parent) = target_path.parent() {
                    fs::create_dir_all(parent).map_err(|error| {
                        AppError::BadRequest(format!("could not prepare asset folder: {error}"))
                    })?;
                }
                if target_path.exists() {
                    return Err(AppError::Conflict(format!(
                        "{asset_id} target file already exists"
                    )));
                }
            }
            (
                target_relative_path.clone(),
                Some(target_relative_path),
                ImportModeConversionStorageAction::CopyFile {
                    source_path,
                    target_path,
                },
            )
        } else {
            let relative_path = current_relative_path
                .clone()
                .or_else(|| {
                    row.try_get::<Option<String>, _>("storage_key")
                        .ok()
                        .flatten()
                })
                .ok_or_else(|| AppError::BadRequest(format!("{asset_id} has no file path")))?;
            let normalized = normalize_readable_storage_file_relative_path(&relative_path)?;
            let source_path = join_safe_relative_path(&base_path, &normalized);
            if !source_path.is_file() {
                return Err(AppError::BadRequest(format!(
                    "{asset_id} source file was not found"
                )));
            }
            (normalized, None, ImportModeConversionStorageAction::None)
        };

        update_import_mode_metadata(&mut next_metadata, &target_mode, &next_relative_path)?;
        plans.push(ImportModeConversionPlan {
            asset_id,
            previous_relative_path: current_relative_path,
            previous_package_root_relative_path,
            next_relative_path,
            next_storage_key,
            storage_action,
            metadata: next_metadata,
        });
    }

    if plans.is_empty() {
        return Ok(Json(
            mutation_response(&state, &library_id, user.id, Vec::new()).await?,
        ));
    }

    let mut rollbacks = Vec::new();
    for plan in &plans {
        match &plan.storage_action {
            ImportModeConversionStorageAction::None => {}
            ImportModeConversionStorageAction::CopyFile {
                source_path,
                target_path,
            } => {
                if source_path != target_path {
                    fs::copy(source_path, target_path).map_err(|error| {
                        AppError::BadRequest(format!("could not copy asset file: {error}"))
                    })?;
                    rollbacks.push(ImportModeConversionRollback::File(target_path.clone()));
                }
            }
            ImportModeConversionStorageAction::CopyPackage {
                source_root,
                target_root,
            } => {
                copy_dir_recursive(source_root, target_root)?;
                rollbacks.push(ImportModeConversionRollback::PackageCopy {
                    source_root: source_root.clone(),
                    target_root: target_root.clone(),
                });
            }
        }
    }

    let mut tx = state.pool.begin().await?;
    let mut converted_ids = Vec::with_capacity(plans.len());
    for plan in &plans {
        sqlx::query(
            r#"
            UPDATE assets
            SET import_mode = $3,
                relative_path = $4,
                storage_key = $5,
                metadata = $6,
                updated_by_user_id = $7,
                updated_at = NOW()
            WHERE library_id = $1 AND id = $2 AND deleted_at IS NULL
            "#,
        )
        .bind(&library_id)
        .bind(&plan.asset_id)
        .bind(&target_mode)
        .bind(&plan.next_relative_path)
        .bind(&plan.next_storage_key)
        .bind(&plan.metadata)
        .bind(user.id)
        .execute(&mut *tx)
        .await
        .map_err(|error| {
            cleanup_import_mode_conversion_rollbacks(&rollbacks);
            error
        })?;
        converted_ids.push(plan.asset_id.clone());
    }
    insert_activity_tx(
        &mut tx,
        &library_id,
        user.id,
        "assets.import_mode_converted",
        &converted_ids,
    )
    .await?;
    tx.commit().await.map_err(|error| {
        cleanup_import_mode_conversion_rollbacks(&rollbacks);
        error
    })?;

    if target_mode == "reference" {
        for plan in &plans {
            if let Some(previous_relative_path) = plan.previous_relative_path.as_deref() {
                if previous_relative_path.starts_with(&format!("assets/{}/", plan.asset_id))
                    && previous_relative_path != plan.next_relative_path
                {
                    if let Some(root_id) = sqlx::query_scalar::<_, Option<Uuid>>(
                        "SELECT storage_root_id FROM assets WHERE library_id = $1 AND id = $2",
                    )
                    .bind(&library_id)
                    .bind(&plan.asset_id)
                    .fetch_optional(&state.pool)
                    .await?
                    .flatten()
                    {
                        let base_path =
                            storage_root_write_base_path(&state, root_id, Some(&library_id))
                                .await?;
                        if let Some(package_root_relative_path) =
                            plan.previous_package_root_relative_path.as_deref()
                        {
                            remove_package_storage_except_reference(&join_safe_relative_path(
                                &base_path,
                                package_root_relative_path,
                            ));
                        } else {
                            let _ = fs::remove_file(join_safe_relative_path(
                                &base_path,
                                previous_relative_path,
                            ));
                        }
                    }
                }
            }
        }
    }

    Ok(Json(
        mutation_response(&state, &library_id, user.id, converted_ids).await?,
    ))
}

pub async fn set_asset_folders(
    State(state): State<AppState>,
    user: AuthUser,
    Path(library_id): Path<String>,
    Json(request): Json<SetAssetFoldersRequest>,
) -> AppResult<Json<AssetMutationResponse>> {
    let asset_ids = normalize_ids(request.asset_ids, "assets")?;
    ensure_library_asset_mutation_access(&state, &user, &library_id, &asset_ids).await?;
    let folder_ids = normalize_ids_unbounded(request.folder_ids);
    let mut tx = state.pool.begin().await?;
    ensure_assets_in_library(&mut tx, &library_id, &asset_ids, false).await?;
    ensure_relation_targets(&mut tx, "folders", &library_id, &folder_ids).await?;

    match request.mode.trim() {
        "replace" => {
            sqlx::query("DELETE FROM asset_folders WHERE asset_id = ANY($1)")
                .bind(&asset_ids)
                .execute(&mut *tx)
                .await?;
            insert_asset_folder_relations(&mut tx, &asset_ids, &folder_ids).await?;
        }
        "add" => insert_asset_folder_relations(&mut tx, &asset_ids, &folder_ids).await?,
        "remove" => {
            sqlx::query(
                "DELETE FROM asset_folders WHERE asset_id = ANY($1) AND folder_id = ANY($2)",
            )
            .bind(&asset_ids)
            .bind(&folder_ids)
            .execute(&mut *tx)
            .await?;
        }
        "clear" => {
            sqlx::query("DELETE FROM asset_folders WHERE asset_id = ANY($1)")
                .bind(&asset_ids)
                .execute(&mut *tx)
                .await?;
        }
        "move" => {
            let source_folder_id = request
                .source_folder_id
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .ok_or_else(|| {
                    AppError::BadRequest("sourceFolderId is required for move".to_string())
                })?;
            if folder_ids.len() != 1 {
                return Err(AppError::BadRequest(
                    "exactly one target folder is required for move".to_string(),
                ));
            }
            ensure_relation_targets(
                &mut tx,
                "folders",
                &library_id,
                &[source_folder_id.to_string()],
            )
            .await?;
            sqlx::query("DELETE FROM asset_folders WHERE asset_id = ANY($1) AND folder_id = $2")
                .bind(&asset_ids)
                .bind(source_folder_id)
                .execute(&mut *tx)
                .await?;
            insert_asset_folder_relations(&mut tx, &asset_ids, &folder_ids).await?;
        }
        _ => {
            return Err(AppError::BadRequest(
                "folder relation mode must be replace, add, remove, clear, or move".to_string(),
            ));
        }
    }
    touch_assets(&mut tx, &library_id, &asset_ids, user.id).await?;
    insert_activity_tx(
        &mut tx,
        &library_id,
        user.id,
        "assets.folders_updated",
        &asset_ids,
    )
    .await?;
    tx.commit().await?;
    Ok(Json(
        mutation_response(&state, &library_id, user.id, asset_ids).await?,
    ))
}

pub async fn set_asset_tags(
    State(state): State<AppState>,
    user: AuthUser,
    Path(library_id): Path<String>,
    Json(request): Json<SetAssetTagsRequest>,
) -> AppResult<Json<AssetMutationResponse>> {
    let asset_ids = normalize_ids(request.asset_ids, "assets")?;
    ensure_library_asset_mutation_access(&state, &user, &library_id, &asset_ids).await?;
    let tag_ids = normalize_ids_unbounded(request.tag_ids);
    let mut tx = state.pool.begin().await?;
    ensure_assets_in_library(&mut tx, &library_id, &asset_ids, false).await?;
    ensure_relation_targets(&mut tx, "tags", &library_id, &tag_ids).await?;

    match request.mode.trim() {
        "replace" => {
            sqlx::query("DELETE FROM asset_tags WHERE asset_id = ANY($1)")
                .bind(&asset_ids)
                .execute(&mut *tx)
                .await?;
            insert_asset_tag_relations(&mut tx, &asset_ids, &tag_ids).await?;
        }
        "add" => insert_asset_tag_relations(&mut tx, &asset_ids, &tag_ids).await?,
        "remove" => {
            sqlx::query("DELETE FROM asset_tags WHERE asset_id = ANY($1) AND tag_id = ANY($2)")
                .bind(&asset_ids)
                .bind(&tag_ids)
                .execute(&mut *tx)
                .await?;
        }
        "clear" => {
            sqlx::query("DELETE FROM asset_tags WHERE asset_id = ANY($1)")
                .bind(&asset_ids)
                .execute(&mut *tx)
                .await?;
        }
        _ => {
            return Err(AppError::BadRequest(
                "tag relation mode must be replace, add, remove, or clear".to_string(),
            ));
        }
    }
    touch_assets(&mut tx, &library_id, &asset_ids, user.id).await?;
    insert_activity_tx(
        &mut tx,
        &library_id,
        user.id,
        "assets.tags_updated",
        &asset_ids,
    )
    .await?;
    tx.commit().await?;
    Ok(Json(
        mutation_response(&state, &library_id, user.id, asset_ids).await?,
    ))
}

pub async fn trash_assets(
    State(state): State<AppState>,
    user: AuthUser,
    Path(library_id): Path<String>,
    Json(request): Json<AssetIdsRequest>,
) -> AppResult<Json<AssetMutationResponse>> {
    change_deleted_state(&state, &user, &library_id, request.asset_ids, true).await
}

pub async fn restore_assets(
    State(state): State<AppState>,
    user: AuthUser,
    Path(library_id): Path<String>,
    Json(request): Json<AssetIdsRequest>,
) -> AppResult<Json<AssetMutationResponse>> {
    change_deleted_state(&state, &user, &library_id, request.asset_ids, false).await
}

pub async fn delete_assets_permanently(
    State(state): State<AppState>,
    user: AuthUser,
    Path(library_id): Path<String>,
    Json(request): Json<AssetIdsRequest>,
) -> AppResult<Json<AssetMutationResponse>> {
    let asset_ids = normalize_ids(request.asset_ids, "assets")?;
    let file_rows = sqlx::query(
        "SELECT storage_root_id, relative_path, metadata FROM assets WHERE library_id = $1 AND id = ANY($2) AND deleted_at IS NOT NULL",
    )
    .bind(&library_id)
    .bind(&asset_ids)
    .fetch_all(&state.pool)
    .await?;
    if file_rows.len() != asset_ids.len() {
        return Err(AppError::BadRequest(
            "only assets in the trash can be permanently deleted".to_string(),
        ));
    }
    ensure_library_asset_mutation_access(&state, &user, &library_id, &asset_ids).await?;

    let mut files = Vec::new();
    for row in file_rows {
        let Some(root_id) = row.try_get::<Option<Uuid>, _>("storage_root_id")? else {
            continue;
        };
        let base_path = storage_root_write_base_path(&state, root_id, Some(&library_id)).await?;
        let relative_path: Option<String> = row.try_get("relative_path")?;
        let metadata: Value = row.try_get("metadata")?;
        for candidate in [
            relative_path,
            metadata
                .get("thumbnailPath")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned),
            metadata
                .get("previewImagePath")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned),
        ]
        .into_iter()
        .flatten()
        {
            if let Ok(relative_path) = normalize_readable_storage_file_relative_path(&candidate) {
                files.push(join_safe_relative_path(&base_path, &relative_path));
            }
        }
    }

    let mut tx = state.pool.begin().await?;
    let deleted_ids: Vec<String> = sqlx::query_scalar(
        "DELETE FROM assets WHERE library_id = $1 AND id = ANY($2) AND deleted_at IS NOT NULL RETURNING id",
    )
    .bind(&library_id)
    .bind(&asset_ids)
    .fetch_all(&mut *tx)
    .await?;
    insert_activity_tx(
        &mut tx,
        &library_id,
        user.id,
        "assets.deleted",
        &deleted_ids,
    )
    .await?;
    tx.commit().await?;

    for file in files {
        let _ = fs::remove_file(file);
    }
    Ok(Json(
        mutation_response(&state, &library_id, user.id, Vec::new())
            .await?
            .with_ids(deleted_ids),
    ))
}

async fn mutate_metadata_field(
    state: &AppState,
    user: &AuthUser,
    library_id: &str,
    asset_ids: Vec<String>,
    field: &str,
    value: Value,
    activity: &str,
) -> AppResult<Json<AssetMutationResponse>> {
    let asset_ids = normalize_ids(asset_ids, "assets")?;
    ensure_library_asset_mutation_access(state, user, library_id, &asset_ids).await?;
    let mut tx = state.pool.begin().await?;
    ensure_assets_in_library(&mut tx, library_id, &asset_ids, false).await?;
    sqlx::query(
        "UPDATE assets SET metadata = jsonb_set(metadata, ARRAY[$3::text], $4::jsonb, TRUE), updated_by_user_id = $5, updated_at = NOW() WHERE library_id = $1 AND id = ANY($2) AND deleted_at IS NULL",
    )
    .bind(library_id)
    .bind(&asset_ids)
    .bind(field)
    .bind(value)
    .bind(user.id)
    .execute(&mut *tx)
    .await?;
    insert_activity_tx(&mut tx, library_id, user.id, activity, &asset_ids).await?;
    tx.commit().await?;
    Ok(Json(
        mutation_response(state, library_id, user.id, asset_ids).await?,
    ))
}

async fn change_deleted_state(
    state: &AppState,
    user: &AuthUser,
    library_id: &str,
    asset_ids: Vec<String>,
    deleted: bool,
) -> AppResult<Json<AssetMutationResponse>> {
    let asset_ids = normalize_ids(asset_ids, "assets")?;
    ensure_library_asset_mutation_access(state, user, library_id, &asset_ids).await?;
    let mut tx = state.pool.begin().await?;
    ensure_assets_in_library(&mut tx, library_id, &asset_ids, !deleted).await?;
    let action = if deleted {
        "assets.trashed"
    } else {
        "assets.restored"
    };
    let returned_ids: Vec<String> = if deleted {
        sqlx::query_scalar(
            "UPDATE assets SET deleted_at = NOW(), deleted_by_user_id = $3, restored_at = NULL, restored_by_user_id = NULL, updated_by_user_id = $3, updated_at = NOW() WHERE library_id = $1 AND id = ANY($2) AND deleted_at IS NULL RETURNING id",
        )
        .bind(library_id)
        .bind(&asset_ids)
        .bind(user.id)
        .fetch_all(&mut *tx)
        .await?
    } else {
        sqlx::query_scalar(
            "UPDATE assets SET deleted_at = NULL, deleted_by_user_id = NULL, restored_at = NOW(), restored_by_user_id = $3, updated_by_user_id = $3, updated_at = NOW() WHERE library_id = $1 AND id = ANY($2) AND deleted_at IS NOT NULL RETURNING id",
        )
        .bind(library_id)
        .bind(&asset_ids)
        .bind(user.id)
        .fetch_all(&mut *tx)
        .await?
    };
    insert_activity_tx(&mut tx, library_id, user.id, action, &returned_ids).await?;
    tx.commit().await?;
    Ok(Json(
        mutation_response(state, library_id, user.id, returned_ids).await?,
    ))
}

pub(super) async fn mutation_response(
    state: &AppState,
    library_id: &str,
    user_id: Uuid,
    asset_ids: Vec<String>,
) -> AppResult<AssetMutationResponse> {
    let records = query_assets_by_ids(state, library_id, &asset_ids).await?;
    let items = build_asset_responses(state, library_id, user_id, records).await?;
    let total = sqlx::query_scalar(
        "SELECT COUNT(*) FROM assets WHERE library_id = $1 AND deleted_at IS NULL",
    )
    .bind(library_id)
    .fetch_one(&state.pool)
    .await?;
    Ok(AssetMutationResponse {
        affected_count: asset_ids.len(),
        asset_ids,
        items,
        total,
    })
}

impl AssetMutationResponse {
    fn with_ids(mut self, asset_ids: Vec<String>) -> Self {
        self.affected_count = asset_ids.len();
        self.asset_ids = asset_ids;
        self
    }
}

async fn ensure_assets_in_library(
    tx: &mut Transaction<'_, Postgres>,
    library_id: &str,
    asset_ids: &[String],
    require_deleted: bool,
) -> AppResult<()> {
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM assets WHERE library_id = $1 AND id = ANY($2) AND (($3 AND deleted_at IS NOT NULL) OR (NOT $3 AND deleted_at IS NULL))",
    )
    .bind(library_id)
    .bind(asset_ids)
    .bind(require_deleted)
    .fetch_one(&mut **tx)
    .await?;
    if count != asset_ids.len() as i64 {
        return Err(AppError::BadRequest(
            "one or more assets were not found".to_string(),
        ));
    }
    Ok(())
}

async fn ensure_relation_targets(
    tx: &mut Transaction<'_, Postgres>,
    table: &str,
    library_id: &str,
    ids: &[String],
) -> AppResult<()> {
    if ids.is_empty() {
        return Ok(());
    }
    let query = match table {
        "folders" => "SELECT COUNT(*) FROM folders WHERE library_id = $1 AND id = ANY($2)",
        "tags" => "SELECT COUNT(*) FROM tags WHERE library_id = $1 AND id = ANY($2)",
        _ => {
            return Err(AppError::Internal(anyhow::anyhow!(
                "invalid relation table"
            )))
        }
    };
    let count: i64 = sqlx::query_scalar(query)
        .bind(library_id)
        .bind(ids)
        .fetch_one(&mut **tx)
        .await?;
    if count != ids.len() as i64 {
        return Err(AppError::BadRequest(format!(
            "one or more {table} were not found"
        )));
    }
    Ok(())
}

async fn touch_assets(
    tx: &mut Transaction<'_, Postgres>,
    library_id: &str,
    asset_ids: &[String],
    user_id: Uuid,
) -> AppResult<()> {
    sqlx::query(
        "UPDATE assets SET updated_by_user_id = $3, updated_at = NOW() WHERE library_id = $1 AND id = ANY($2)",
    )
    .bind(library_id)
    .bind(asset_ids)
    .bind(user_id)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn insert_activity(
    state: &AppState,
    library_id: &str,
    user_id: Uuid,
    action: &str,
    asset_ids: &[String],
) -> AppResult<()> {
    let mut tx = state.pool.begin().await?;
    insert_activity_tx(&mut tx, library_id, user_id, action, asset_ids).await?;
    tx.commit().await?;
    Ok(())
}

pub(super) async fn insert_activity_tx(
    tx: &mut Transaction<'_, Postgres>,
    library_id: &str,
    user_id: Uuid,
    action: &str,
    asset_ids: &[String],
) -> AppResult<()> {
    sqlx::query(
        "INSERT INTO activity_log (id, library_id, actor_user_id, action, target_type, details) VALUES ($1, $2, $3, $4, 'asset', $5::jsonb)",
    )
    .bind(Uuid::new_v4())
    .bind(library_id)
    .bind(user_id)
    .bind(action)
    .bind(json!({ "assetIds": asset_ids, "count": asset_ids.len() }))
    .execute(&mut **tx)
    .await?;
    Ok(())
}

fn normalize_ids(values: Vec<String>, label: &str) -> AppResult<Vec<String>> {
    let values = normalize_ids_unbounded(values);
    if values.is_empty() {
        return Err(AppError::BadRequest(format!(
            "at least one {label} id is required"
        )));
    }
    if values.len() > MAX_MUTATION_ASSETS {
        return Err(AppError::BadRequest(format!(
            "cannot update more than {MAX_MUTATION_ASSETS} assets at once"
        )));
    }
    Ok(values)
}

fn normalize_ids_unbounded(values: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    values
        .into_iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty() && seen.insert(value.clone()))
        .collect()
}

fn normalize_conversion_target_mode(value: &str) -> AppResult<String> {
    match value.trim() {
        "copy" => Ok("copy".to_string()),
        "reference" => Ok("reference".to_string()),
        _ => Err(AppError::BadRequest("import mode is invalid".to_string())),
    }
}

fn validate_convertible_asset_kind(
    asset_id: &str,
    asset_kind: &str,
    metadata: &Value,
) -> AppResult<()> {
    if asset_kind == "link" {
        return Err(AppError::BadRequest(format!(
            "{asset_id} links cannot be converted"
        )));
    }
    if metadata
        .get("subtype")
        .and_then(Value::as_str)
        .is_some_and(|value| value == "sequence")
    {
        return Err(AppError::BadRequest(format!(
            "{asset_id} image sequences cannot be converted yet"
        )));
    }
    Ok(())
}

fn metadata_package_string(metadata: &Value, key: &str) -> Option<String> {
    metadata
        .get("package")
        .and_then(Value::as_object)
        .and_then(|package| package.get(key))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.replace('\\', "/"))
}

fn metadata_package_main_relative_path(metadata: &Value, asset_id: &str) -> AppResult<String> {
    let value = metadata
        .get("package")
        .and_then(|package| package.get("mainFile"))
        .and_then(Value::as_object)
        .and_then(|main_file| main_file.get("relativePath"))
        .and_then(Value::as_str)
        .ok_or_else(|| AppError::BadRequest(format!("{asset_id} package main file is missing")))?;
    normalize_readable_storage_file_relative_path(value)
}

fn package_root_relative_path_from_main_path(
    main_path: &str,
    main_relative_path: &str,
) -> Option<String> {
    let main_path = main_path.replace('\\', "/");
    let main_relative_path = main_relative_path.replace('\\', "/");
    main_path
        .strip_suffix(&main_relative_path)
        .map(|value| value.trim_end_matches('/').to_string())
}

fn package_root_relative_path(
    metadata: &Value,
    main_path: &str,
    main_relative_path: &str,
) -> AppResult<String> {
    let candidates = [
        metadata_package_string(metadata, "referenceDirectory"),
        metadata_package_string(metadata, "rootSourcePath"),
        metadata_package_string(metadata, "storedDirectory"),
        package_root_relative_path_from_main_path(main_path, main_relative_path),
    ];
    for candidate in candidates.into_iter().flatten() {
        if let Ok(relative_path) = normalize_readable_storage_file_relative_path(&candidate) {
            return Ok(relative_path);
        }
    }
    Err(AppError::BadRequest(
        "package root path is missing".to_string(),
    ))
}

fn update_team_package_import_mode_metadata(
    metadata: &mut Value,
    asset_id: &str,
    import_mode: &str,
    root_relative_path: &str,
) -> AppResult<()> {
    let package = metadata
        .get_mut("package")
        .and_then(Value::as_object_mut)
        .ok_or_else(|| AppError::BadRequest("package metadata is missing".to_string()))?;
    if import_mode == "copy" {
        package.insert(
            "storedDirectory".to_string(),
            json!(format!("assets/{asset_id}")),
        );
        package.insert("referenceDirectory".to_string(), Value::Null);
    } else {
        package.insert("storedDirectory".to_string(), Value::Null);
        package.insert("referenceDirectory".to_string(), json!(root_relative_path));
    }
    package.insert("rootSourcePath".to_string(), json!(root_relative_path));

    let next_root = if import_mode == "copy" {
        format!("assets/{asset_id}")
    } else {
        root_relative_path.to_string()
    };
    for key in ["textureMaps", "modelPackage"] {
        if let Some(object) = metadata.get_mut(key).and_then(Value::as_object_mut) {
            object.insert("rootRelativePath".to_string(), json!(next_root.clone()));
        }
    }
    Ok(())
}

fn copy_dir_recursive(source: &StdPath, target: &StdPath) -> AppResult<()> {
    if !source.is_dir() {
        return Err(AppError::BadRequest(
            "package source folder was not found".to_string(),
        ));
    }
    fs::create_dir_all(target).map_err(|error| {
        AppError::BadRequest(format!("could not prepare package folder: {error}"))
    })?;
    for entry in fs::read_dir(source)
        .map_err(|error| AppError::BadRequest(format!("could not read package folder: {error}")))?
    {
        let entry = entry.map_err(|error| {
            AppError::BadRequest(format!("could not read package folder: {error}"))
        })?;
        let file_type = entry.file_type().map_err(|error| {
            AppError::BadRequest(format!("could not read package entry: {error}"))
        })?;
        let source_path = entry.path();
        let target_path = target.join(entry.file_name());
        if file_type.is_dir() {
            copy_dir_recursive(&source_path, &target_path)?;
        } else if file_type.is_file() {
            if let Some(parent) = target_path.parent() {
                fs::create_dir_all(parent).map_err(|error| {
                    AppError::BadRequest(format!("could not prepare package folder: {error}"))
                })?;
            }
            fs::copy(&source_path, &target_path).map_err(|error| {
                AppError::BadRequest(format!("could not copy package file: {error}"))
            })?;
        }
    }
    Ok(())
}

fn collect_files_and_dirs(root: &StdPath) -> AppResult<(Vec<PathBuf>, Vec<PathBuf>)> {
    let mut files = Vec::new();
    let mut dirs = Vec::new();
    fn visit(path: &StdPath, files: &mut Vec<PathBuf>, dirs: &mut Vec<PathBuf>) -> AppResult<()> {
        for entry in fs::read_dir(path).map_err(|error| {
            AppError::BadRequest(format!("could not read package folder: {error}"))
        })? {
            let entry = entry.map_err(|error| {
                AppError::BadRequest(format!("could not read package folder: {error}"))
            })?;
            let entry_path = entry.path();
            let file_type = entry.file_type().map_err(|error| {
                AppError::BadRequest(format!("could not read package entry: {error}"))
            })?;
            if file_type.is_dir() {
                visit(&entry_path, files, dirs)?;
                dirs.push(entry_path);
            } else if file_type.is_file() {
                files.push(entry_path);
            }
        }
        Ok(())
    }
    if root.is_dir() {
        visit(root, &mut files, &mut dirs)?;
    }
    Ok((files, dirs))
}

fn cleanup_package_copy(source_root: &StdPath, target_root: &StdPath) {
    let Ok((source_files, source_dirs)) = collect_files_and_dirs(source_root) else {
        return;
    };
    for source_file in source_files {
        if let Ok(relative_path) = source_file.strip_prefix(source_root) {
            let _ = fs::remove_file(target_root.join(relative_path));
        }
    }
    let mut source_dirs = source_dirs;
    source_dirs.sort_by_key(|path| std::cmp::Reverse(path.components().count()));
    for source_dir in source_dirs {
        if let Ok(relative_path) = source_dir.strip_prefix(source_root) {
            let _ = fs::remove_dir(target_root.join(relative_path));
        }
    }
    let _ = fs::remove_dir(target_root);
}

fn cleanup_import_mode_conversion_rollbacks(rollbacks: &[ImportModeConversionRollback]) {
    for rollback in rollbacks {
        match rollback {
            ImportModeConversionRollback::File(path) => {
                let _ = fs::remove_file(path);
            }
            ImportModeConversionRollback::PackageCopy {
                source_root,
                target_root,
            } => cleanup_package_copy(source_root, target_root),
        }
    }
}

fn remove_package_storage_except_reference(package_root: &StdPath) {
    let Ok(entries) = fs::read_dir(package_root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let is_reference_manifest = path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.eq_ignore_ascii_case("reference.json"));
        if is_reference_manifest {
            continue;
        }
        if path.is_dir() {
            let _ = fs::remove_dir_all(path);
        } else {
            let _ = fs::remove_file(path);
        }
    }
}

fn update_import_mode_metadata(
    metadata: &mut Value,
    import_mode: &str,
    relative_path: &str,
) -> AppResult<()> {
    let object = metadata
        .as_object_mut()
        .ok_or_else(|| AppError::BadRequest("asset metadata must be an object".to_string()))?;
    object.insert("importMode".to_string(), json!(import_mode));
    object.insert("sourcePath".to_string(), json!(relative_path));
    object.insert("storedPath".to_string(), json!(relative_path));
    Ok(())
}

fn validate_asset_name(value: &str) -> AppResult<String> {
    let value = value.trim();
    if value.is_empty()
        || value == "."
        || value == ".."
        || value.contains('/')
        || value.contains('\\')
        || value.chars().any(char::is_control)
    {
        return Err(AppError::BadRequest("asset name is invalid".to_string()));
    }
    Ok(value.to_string())
}

fn validate_rating(rating: i64) -> AppResult<()> {
    if !(0..=5).contains(&rating) {
        return Err(AppError::BadRequest(
            "asset rating must be between 0 and 5".to_string(),
        ));
    }
    Ok(())
}

fn validate_derived_metadata_patch(value: Value, asset_id: &str) -> AppResult<Value> {
    const ALLOWED_FIELDS: &[&str] = &[
        "assetSubType",
        "colorPalette",
        "duration",
        "durationSeconds",
        "previewImagePath",
        "previewVideoError",
        "previewVideoPath",
        "processor",
        "sequence",
        "thumbnailCustom",
        "thumbnailHeight",
        "thumbnailPath",
        "thumbnailProcessor",
        "thumbnailSource",
        "thumbnailWidth",
    ];
    let object = value
        .as_object()
        .ok_or_else(|| AppError::BadRequest("metadataPatch must be an object".to_string()))?;
    if object
        .keys()
        .any(|key| !ALLOWED_FIELDS.contains(&key.as_str()))
    {
        return Err(AppError::BadRequest(
            "metadataPatch contains unsupported derived metadata fields".to_string(),
        ));
    }
    for field in ["thumbnailPath", "previewImagePath", "previewVideoPath"] {
        if let Some(path) = object.get(field).and_then(Value::as_str) {
            super::normalize_derived_file_relative_path(path)?;
        }
    }
    if let Some(sequence) = object.get("sequence") {
        validate_sequence_metadata_patch(sequence, asset_id)?;
    }
    Ok(value)
}

fn validate_sequence_metadata_patch(value: &Value, asset_id: &str) -> AppResult<()> {
    let sequence = value
        .as_object()
        .ok_or_else(|| AppError::BadRequest("sequence metadata must be an object".to_string()))?;
    let stored_directory = sequence
        .get("storedDirectory")
        .and_then(Value::as_str)
        .ok_or_else(|| AppError::BadRequest("sequence storage folder is required".to_string()))?;
    let stored_directory = super::normalize_readable_storage_file_relative_path(stored_directory)?;
    if stored_directory != format!("assets/{asset_id}") {
        return Err(AppError::BadRequest(
            "sequence storage folder is invalid".to_string(),
        ));
    }
    if let Some(selected_path) = sequence.get("selectedFramePath").and_then(Value::as_str) {
        let selected_path = super::normalize_readable_storage_file_relative_path(selected_path)?;
        if !selected_path.starts_with(&format!("{stored_directory}/")) {
            return Err(AppError::BadRequest(
                "sequence selected frame is outside its storage folder".to_string(),
            ));
        }
    }
    let frames = sequence
        .get("frames")
        .and_then(Value::as_array)
        .ok_or_else(|| AppError::BadRequest("sequence frames are required".to_string()))?;
    if frames.is_empty() || frames.len() > 100_000 {
        return Err(AppError::BadRequest(
            "sequence frame metadata is invalid".to_string(),
        ));
    }
    for frame in frames {
        let file_name = frame.get("fileName").and_then(Value::as_str).unwrap_or("");
        if file_name.is_empty()
            || file_name == "."
            || file_name == ".."
            || file_name.contains('/')
            || file_name.contains('\\')
            || file_name.contains('\0')
        {
            return Err(AppError::BadRequest(
                "sequence frame metadata is invalid".to_string(),
            ));
        }
    }
    Ok(())
}

async fn insert_asset_folder_relations(
    tx: &mut Transaction<'_, Postgres>,
    asset_ids: &[String],
    folder_ids: &[String],
) -> AppResult<()> {
    for asset_id in asset_ids {
        for folder_id in folder_ids {
            sqlx::query(
                "INSERT INTO asset_folders (asset_id, folder_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
            )
            .bind(asset_id)
            .bind(folder_id)
            .execute(&mut **tx)
            .await?;
        }
    }
    Ok(())
}

async fn insert_asset_tag_relations(
    tx: &mut Transaction<'_, Postgres>,
    asset_ids: &[String],
    tag_ids: &[String],
) -> AppResult<()> {
    for asset_id in asset_ids {
        for tag_id in tag_ids {
            sqlx::query(
                "INSERT INTO asset_tags (asset_id, tag_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
            )
            .bind(asset_id)
            .bind(tag_id)
            .execute(&mut **tx)
            .await?;
        }
    }
    Ok(())
}

fn default_relation_mode() -> String {
    "replace".to_string()
}
