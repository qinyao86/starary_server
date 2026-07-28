mod files;

use super::{ensure_unique_location, get_connection, require_server_manager, validate_location};
use crate::{
    auth::AuthUser,
    error::{AppError, AppResult},
    models::{StorageConnectionRecord, StorageRootKind},
    path_resolver::{
        ensure_storage_location_exists, resolve_storage_namespace_with_policy, storage_identity,
        storage_locations_overlap, ResolvedStorageLocation,
    },
    state::AppState,
};
use axum::{
    extract::{Path, State},
    Json,
};
use files::{FileMigrationPlan, MigrationManifest};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::{path::PathBuf, str::FromStr};
use uuid::Uuid;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MigrateStorageConnectionRequest {
    kind: StorageRootKind,
    canonical_uri: String,
    windows_unc_path: Option<String>,
    #[serde(default)]
    windows_mapped_drive_aliases: Vec<String>,
    macos_smb_url: Option<String>,
    #[serde(default)]
    macos_mount_aliases: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageMigrationResponse {
    connection: StorageConnectionRecord,
    migrated_library_count: i64,
    migrated_asset_count: i64,
    estimated_size_bytes: i64,
    previous_location: String,
    current_location: String,
}

#[derive(Clone, FromRow)]
struct MigrationRoot {
    id: Uuid,
    library_id: String,
    namespace: String,
    kind: String,
    canonical_uri: String,
    windows_unc_path: Option<String>,
}

#[derive(Clone)]
struct RootPlan {
    root: MigrationRoot,
    target: ResolvedStorageLocation,
    files: FileMigrationPlan,
}

pub async fn migrate_storage_connection(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(request): Json<MigrateStorageConnectionRequest>,
) -> AppResult<Json<StorageMigrationResponse>> {
    require_server_manager(&user)?;
    let existing = get_connection(&state, id).await?;
    let source_kind = StorageRootKind::from_str(&existing.kind).map_err(AppError::BadRequest)?;
    if source_kind == StorageRootKind::S3 || request.kind == StorageRootKind::S3 {
        return Err(AppError::BadRequest(
            "object storage migration is not supported yet".to_string(),
        ));
    }

    let target = validate_location(
        &state,
        request.kind,
        &request.canonical_uri,
        request.windows_unc_path,
        request.macos_smb_url,
        &request.windows_mapped_drive_aliases,
        &request.macos_mount_aliases,
    )?;
    ensure_storage_location_exists(request.kind, &target)?;
    ensure_unique_location(&state, request.kind, &target.canonical_uri, Some(id)).await?;
    if source_kind == request.kind
        && storage_identity(&existing.canonical_uri) == storage_identity(&target.canonical_uri)
    {
        return Err(AppError::BadRequest(
            "migration destination must differ from the current storage location".to_string(),
        ));
    }

    let lock_name = format!("starary.storage-migration.{id}");
    let mut migration_lock = state.pool.acquire().await?;
    let acquired: bool = sqlx::query_scalar("SELECT pg_try_advisory_lock(hashtext($1))")
        .bind(&lock_name)
        .fetch_one(&mut *migration_lock)
        .await?;
    if !acquired {
        return Err(AppError::Conflict(
            "this storage location is already being migrated".to_string(),
        ));
    }

    let migration_result: AppResult<StorageMigrationResponse> = async {
    let roots = sqlx::query_as::<_, MigrationRoot>(
        r#"
        SELECT id, library_id, namespace, kind, canonical_uri, windows_unc_path
        FROM storage_roots
        WHERE storage_connection_id = $1
        ORDER BY library_id
        "#,
    )
    .bind(id)
    .fetch_all(&state.pool)
    .await?;
    let plans = build_plans(&state, request.kind, &target, roots)?;
    ensure_targets_are_exclusive(&state, id, request.kind, &plans).await?;

    let file_plans = plans.iter().map(|plan| plan.files.clone()).collect::<Vec<_>>();
    let manifest = run_initial_copy(file_plans.clone()).await?;

    let library_states = load_library_states(&state, &plans).await?;
    disable_libraries(&state, &library_states).await?;
    if let Err(error) = run_final_sync(file_plans, manifest).await {
        restore_libraries(&state, &library_states).await?;
        return Err(error);
    }

    let previous_location = existing.canonical_uri.clone();
    let database_result = switch_database_location(
        &state,
        &user,
        id,
        request.kind,
        &target,
        &request.windows_mapped_drive_aliases,
        &request.macos_mount_aliases,
        &plans,
        &library_states,
        &existing,
    )
    .await;
    if let Err(error) = database_result {
        restore_libraries(&state, &library_states).await?;
        return Err(AppError::Internal(anyhow::anyhow!(
            "files were copied but the database migration was not committed; the old location remains active: {error}"
        )));
    }

    Ok(StorageMigrationResponse {
        connection: get_connection(&state, id).await?,
        migrated_library_count: existing.library_count,
        migrated_asset_count: existing.asset_count,
        estimated_size_bytes: existing.total_size_bytes,
        previous_location,
        current_location: target.canonical_uri,
    })
    }
    .await;

    let unlocked: bool = sqlx::query_scalar("SELECT pg_advisory_unlock(hashtext($1))")
        .bind(&lock_name)
        .fetch_one(&mut *migration_lock)
        .await?;
    if !unlocked && migration_result.is_ok() {
        return Err(AppError::Internal(anyhow::anyhow!(
            "storage migration completed but its advisory lock was not released"
        )));
    }
    migration_result.map(Json)
}

fn build_plans(
    state: &AppState,
    target_kind: StorageRootKind,
    target_connection: &ResolvedStorageLocation,
    roots: Vec<MigrationRoot>,
) -> AppResult<Vec<RootPlan>> {
    roots
        .into_iter()
        .map(|root| {
            let source_kind =
                StorageRootKind::from_str(&root.kind).map_err(AppError::BadRequest)?;
            let source = accessible_path(
                source_kind,
                &root.canonical_uri,
                root.windows_unc_path.as_deref(),
            )?;
            let target = resolve_storage_namespace_with_policy(
                target_kind,
                &target_connection.canonical_uri,
                &root.namespace,
                target_connection.windows_unc_path.clone(),
                target_connection.macos_smb_url.clone(),
                state.config.allow_personal_storage_paths,
            )?;
            let destination = accessible_path(
                target_kind,
                &target.canonical_uri,
                target.windows_unc_path.as_deref(),
            )?;
            Ok(RootPlan {
                root,
                target,
                files: FileMigrationPlan {
                    source,
                    destination,
                },
            })
        })
        .collect()
}

fn accessible_path(
    kind: StorageRootKind,
    canonical_uri: &str,
    windows_unc_path: Option<&str>,
) -> AppResult<PathBuf> {
    match kind {
        StorageRootKind::ServerFilesystem => Ok(PathBuf::from(canonical_uri)),
        StorageRootKind::Smb if cfg!(windows) => windows_unc_path
            .map(PathBuf::from)
            .ok_or_else(|| AppError::BadRequest("shared storage has no Windows path".to_string())),
        StorageRootKind::Smb => Err(AppError::BadRequest(
            "shared storage migration requires a Windows server".to_string(),
        )),
        StorageRootKind::S3 => Err(AppError::BadRequest(
            "object storage migration is not supported yet".to_string(),
        )),
    }
}

async fn ensure_targets_are_exclusive(
    state: &AppState,
    connection_id: Uuid,
    target_kind: StorageRootKind,
    plans: &[RootPlan],
) -> AppResult<()> {
    for (index, plan) in plans.iter().enumerate() {
        for source in plans {
            if storage_locations_overlap(&plan.target.canonical_uri, &source.root.canonical_uri) {
                return Err(AppError::Conflict(
                    "migration destination overlaps the current storage location".to_string(),
                ));
            }
        }
        for other in plans.iter().skip(index + 1) {
            if storage_locations_overlap(&plan.target.canonical_uri, &other.target.canonical_uri) {
                return Err(AppError::Conflict(
                    "migration would create overlapping library folders".to_string(),
                ));
            }
        }
    }

    let existing: Vec<(String, String)> = sqlx::query_as(
        "SELECT kind, canonical_uri FROM storage_roots WHERE storage_connection_id <> $1",
    )
    .bind(connection_id)
    .fetch_all(&state.pool)
    .await?;
    for plan in plans {
        if existing.iter().any(|(kind, uri)| {
            kind == target_kind.as_str()
                && storage_locations_overlap(uri, &plan.target.canonical_uri)
        }) {
            return Err(AppError::StorageLocationConflict(
                plan.target.canonical_uri.clone(),
            ));
        }
    }
    Ok(())
}

async fn run_initial_copy(plans: Vec<FileMigrationPlan>) -> AppResult<MigrationManifest> {
    Ok(
        tokio::task::spawn_blocking(move || files::prepare_and_copy(&plans))
            .await
            .map_err(|error| {
                AppError::Internal(anyhow::anyhow!("storage migration worker failed: {error}"))
            })??,
    )
}

async fn run_final_sync(
    plans: Vec<FileMigrationPlan>,
    manifest: MigrationManifest,
) -> AppResult<()> {
    tokio::task::spawn_blocking(move || files::synchronize(&plans, true, Some(&manifest)))
        .await
        .map_err(|error| {
            AppError::Internal(anyhow::anyhow!("storage migration worker failed: {error}"))
        })??;
    Ok(())
}

async fn load_library_states(
    state: &AppState,
    plans: &[RootPlan],
) -> AppResult<Vec<(String, bool)>> {
    let ids = plans
        .iter()
        .map(|plan| plan.root.library_id.clone())
        .collect::<Vec<_>>();
    Ok(
        sqlx::query_as("SELECT id, enabled FROM libraries WHERE id = ANY($1)")
            .bind(ids)
            .fetch_all(&state.pool)
            .await?,
    )
}

async fn disable_libraries(state: &AppState, libraries: &[(String, bool)]) -> AppResult<()> {
    let ids = libraries
        .iter()
        .map(|(id, _)| id.clone())
        .collect::<Vec<_>>();
    if !ids.is_empty() {
        sqlx::query("UPDATE libraries SET enabled = FALSE WHERE id = ANY($1)")
            .bind(ids)
            .execute(&state.pool)
            .await?;
    }
    Ok(())
}

async fn restore_libraries(state: &AppState, libraries: &[(String, bool)]) -> AppResult<()> {
    for (id, enabled) in libraries {
        sqlx::query("UPDATE libraries SET enabled = $2 WHERE id = $1")
            .bind(id)
            .bind(enabled)
            .execute(&state.pool)
            .await?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn switch_database_location(
    state: &AppState,
    user: &AuthUser,
    connection_id: Uuid,
    target_kind: StorageRootKind,
    target: &ResolvedStorageLocation,
    windows_aliases: &[String],
    macos_aliases: &[String],
    plans: &[RootPlan],
    library_states: &[(String, bool)],
    existing: &StorageConnectionRecord,
) -> AppResult<()> {
    let windows_aliases_json = serde_json::to_value(windows_aliases)?;
    let macos_aliases_json = serde_json::to_value(macos_aliases)?;
    let mut tx = state.pool.begin().await?;
    sqlx::query(
        r#"
        UPDATE storage_connections
        SET kind = $2, canonical_uri = $3, windows_unc_path = $4,
            windows_mapped_drive_aliases = $5::jsonb, macos_smb_url = $6,
            macos_mount_aliases = $7::jsonb, updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(connection_id)
    .bind(target_kind.as_str())
    .bind(&target.canonical_uri)
    .bind(&target.windows_unc_path)
    .bind(&windows_aliases_json)
    .bind(&target.macos_smb_url)
    .bind(&macos_aliases_json)
    .execute(&mut *tx)
    .await?;

    for plan in plans {
        sqlx::query(
            r#"
            UPDATE storage_roots
            SET kind = $2, canonical_uri = $3, storage_identity = $4,
                windows_unc_path = $5, windows_mapped_drive_aliases = $6::jsonb,
                macos_smb_url = $7, macos_mount_aliases = $8::jsonb, updated_at = NOW()
            WHERE id = $1 AND storage_connection_id = $9
            "#,
        )
        .bind(plan.root.id)
        .bind(target_kind.as_str())
        .bind(&plan.target.canonical_uri)
        .bind(storage_identity(&plan.target.canonical_uri))
        .bind(&plan.target.windows_unc_path)
        .bind(&windows_aliases_json)
        .bind(&plan.target.macos_smb_url)
        .bind(&macos_aliases_json)
        .bind(connection_id)
        .execute(&mut *tx)
        .await?;
    }
    for (id, enabled) in library_states {
        sqlx::query("UPDATE libraries SET enabled = $2 WHERE id = $1")
            .bind(id)
            .bind(enabled)
            .execute(&mut *tx)
            .await?;
    }
    sqlx::query(
        r#"
        INSERT INTO activity_log (id, actor_user_id, action, target_type, target_id, details)
        VALUES ($1, $2, 'storage_connection.migrated', 'storage_connection', $3, $4::jsonb)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(user.id)
    .bind(connection_id.to_string())
    .bind(serde_json::json!({
        "previousLocation": existing.canonical_uri,
        "currentLocation": target.canonical_uri,
        "libraryCount": existing.library_count,
        "assetCount": existing.asset_count,
        "estimatedSizeBytes": existing.total_size_bytes,
        "oldFilesRetained": true,
    }))
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(())
}
