use crate::{
    auth::AuthUser,
    error::{AppError, AppResult},
    models::{StorageConnectionRecord, StorageRootKind},
    path_resolver::{
        ensure_storage_location_exists, resolve_storage_location_with_policy,
        validate_aliases_with_policy,
    },
    state::AppState,
};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

mod migration;

pub use migration::migrate_storage_connection;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateStorageConnectionRequest {
    name: Option<String>,
    kind: StorageRootKind,
    canonical_uri: String,
    windows_unc_path: Option<String>,
    #[serde(default)]
    windows_mapped_drive_aliases: Vec<String>,
    macos_smb_url: Option<String>,
    #[serde(default)]
    macos_mount_aliases: Vec<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateStorageConnectionRequest {
    name: Option<String>,
    kind: StorageRootKind,
    canonical_uri: String,
    windows_unc_path: Option<String>,
    #[serde(default)]
    windows_mapped_drive_aliases: Vec<String>,
    macos_smb_url: Option<String>,
    #[serde(default)]
    macos_mount_aliases: Vec<String>,
    enabled: bool,
}

pub async fn list_storage_connections(
    State(state): State<AppState>,
    user: AuthUser,
) -> AppResult<Json<Vec<StorageConnectionRecord>>> {
    require_server_manager(&user)?;

    Ok(Json(
        sqlx::query_as::<_, StorageConnectionRecord>(STORAGE_CONNECTION_SELECT)
            .fetch_all(&state.pool)
            .await?,
    ))
}

pub async fn create_storage_connection(
    State(state): State<AppState>,
    user: AuthUser,
    Json(request): Json<CreateStorageConnectionRequest>,
) -> AppResult<Json<StorageConnectionRecord>> {
    require_server_manager(&user)?;
    let location = validate_location(
        &state,
        request.kind,
        &request.canonical_uri,
        request.windows_unc_path,
        request.macos_smb_url,
        &request.windows_mapped_drive_aliases,
        &request.macos_mount_aliases,
    )?;
    ensure_storage_location_exists(request.kind, &location)?;
    ensure_unique_location(&state, request.kind, &location.canonical_uri, None).await?;

    let id = Uuid::new_v4();
    let name = match request.name.as_deref() {
        Some(value) => validate_name(&state, value, None).await?,
        None => format!("storage-{id}"),
    };
    let mut tx = state.pool.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtext('starary.default-storage'))")
        .execute(&mut *tx)
        .await?;
    let should_be_default: bool = sqlx::query_scalar(
        "SELECT NOT EXISTS(SELECT 1 FROM storage_connections WHERE is_default AND enabled)",
    )
    .fetch_one(&mut *tx)
    .await?;
    sqlx::query(
        r#"
        INSERT INTO storage_connections (
            id, name, kind, canonical_uri, windows_unc_path,
            windows_mapped_drive_aliases, macos_smb_url, macos_mount_aliases,
            is_default, created_by_user_id
        )
        VALUES ($1, $2, $3, $4, $5, $6::jsonb, $7, $8::jsonb, $9, $10)
        "#,
    )
    .bind(id)
    .bind(name)
    .bind(request.kind.as_str())
    .bind(location.canonical_uri)
    .bind(location.windows_unc_path)
    .bind(serde_json::to_value(request.windows_mapped_drive_aliases)?)
    .bind(location.macos_smb_url)
    .bind(serde_json::to_value(request.macos_mount_aliases)?)
    .bind(should_be_default)
    .bind(user.id)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    Ok(Json(get_connection(&state, id).await?))
}

pub async fn update_storage_connection(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(request): Json<UpdateStorageConnectionRequest>,
) -> AppResult<Json<StorageConnectionRecord>> {
    require_server_manager(&user)?;
    let existing = get_connection(&state, id).await?;
    let name = match request.name.as_deref() {
        Some(value) => validate_name(&state, value, Some(id)).await?,
        None => existing.name.clone(),
    };
    let location = validate_location(
        &state,
        request.kind,
        &request.canonical_uri,
        request.windows_unc_path,
        request.macos_smb_url,
        &request.windows_mapped_drive_aliases,
        &request.macos_mount_aliases,
    )?;
    ensure_storage_location_exists(request.kind, &location)?;
    ensure_unique_location(&state, request.kind, &location.canonical_uri, Some(id)).await?;

    let windows_aliases = serde_json::to_value(&request.windows_mapped_drive_aliases)?;
    let macos_aliases = serde_json::to_value(&request.macos_mount_aliases)?;

    let location_changed = existing.kind != request.kind.as_str()
        || existing.canonical_uri != location.canonical_uri
        || existing.windows_unc_path != location.windows_unc_path
        || existing.windows_mapped_drive_aliases != windows_aliases
        || existing.macos_smb_url != location.macos_smb_url
        || existing.macos_mount_aliases != macos_aliases;
    if location_changed {
        return Err(AppError::Conflict(
            "storage location can only be changed with the migration workflow".to_string(),
        ));
    }

    let mut tx = state.pool.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtext('starary.default-storage'))")
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        r#"
        UPDATE storage_connections
        SET name = $2,
            kind = $3,
            canonical_uri = $4,
            windows_unc_path = $5,
            windows_mapped_drive_aliases = $6::jsonb,
            macos_smb_url = $7,
            macos_mount_aliases = $8::jsonb,
            enabled = $9,
            is_default = CASE WHEN $9 THEN is_default ELSE FALSE END,
            updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(id)
    .bind(name)
    .bind(request.kind.as_str())
    .bind(location.canonical_uri)
    .bind(location.windows_unc_path)
    .bind(windows_aliases)
    .bind(location.macos_smb_url)
    .bind(macos_aliases)
    .bind(request.enabled)
    .execute(&mut *tx)
    .await?;
    if existing.is_default && !request.enabled {
        select_fallback_default(&mut tx, Some(id)).await?;
    }
    tx.commit().await?;

    Ok(Json(get_connection(&state, id).await?))
}

pub async fn set_default_storage_connection(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<StorageConnectionRecord>> {
    require_server_manager(&user)?;
    let mut tx = state.pool.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtext('starary.default-storage'))")
        .execute(&mut *tx)
        .await?;
    let enabled: Option<bool> =
        sqlx::query_scalar("SELECT enabled FROM storage_connections WHERE id = $1 FOR UPDATE")
            .bind(id)
            .fetch_optional(&mut *tx)
            .await?;
    match enabled {
        None => {
            return Err(AppError::NotFound(
                "storage connection not found".to_string(),
            ));
        }
        Some(false) => {
            return Err(AppError::BadRequest(
                "disabled storage connection cannot be the default".to_string(),
            ));
        }
        Some(true) => {}
    }
    sqlx::query("UPDATE storage_connections SET is_default = FALSE WHERE is_default AND id <> $1")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "UPDATE storage_connections SET is_default = TRUE, updated_at = NOW() WHERE id = $1",
    )
    .bind(id)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Json(get_connection(&state, id).await?))
}

pub async fn delete_storage_connection(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> AppResult<StatusCode> {
    require_server_manager(&user)?;
    let existing = get_connection(&state, id).await?;
    if existing.library_count > 0 {
        return Err(AppError::Conflict(
            "storage connection is used by libraries; detach them before deleting it".to_string(),
        ));
    }
    let mut tx = state.pool.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtext('starary.default-storage'))")
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM storage_connections WHERE id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    if existing.is_default {
        select_fallback_default(&mut tx, Some(id)).await?;
    }
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

const STORAGE_CONNECTION_SELECT: &str = r#"
SELECT
    sc.id,
    sc.name,
    sc.kind,
    sc.canonical_uri,
    sc.windows_unc_path,
    sc.windows_mapped_drive_aliases,
    sc.macos_smb_url,
    sc.macos_mount_aliases,
    sc.enabled,
    sc.is_default,
    COALESCE(root_stats.library_count, 0)::BIGINT AS library_count,
    COALESCE(root_stats.library_names, ARRAY[]::TEXT[]) AS library_names,
    COALESCE(asset_stats.asset_count, 0)::BIGINT AS asset_count,
    COALESCE(asset_stats.total_size_bytes, 0)::BIGINT AS total_size_bytes,
    sc.created_by_user_id,
    sc.created_at,
    sc.updated_at
FROM storage_connections sc
LEFT JOIN LATERAL (
    SELECT
        COUNT(sr.id)::BIGINT AS library_count,
        ARRAY_AGG(l.display_name ORDER BY l.display_name) AS library_names
    FROM storage_roots sr
    INNER JOIN libraries l ON l.id = sr.library_id AND l.deleted_at IS NULL
    WHERE sr.storage_connection_id = sc.id
) root_stats ON TRUE
LEFT JOIN LATERAL (
    SELECT
        COUNT(a.id) FILTER (WHERE a.deleted_at IS NULL)::BIGINT AS asset_count,
        COALESCE(SUM(
            CASE
                WHEN a.deleted_at IS NULL AND (a.metadata->>'sizeBytes') ~ '^[0-9]+$' THEN (a.metadata->>'sizeBytes')::BIGINT
                WHEN a.deleted_at IS NULL AND (a.metadata->>'fileSize') ~ '^[0-9]+$' THEN (a.metadata->>'fileSize')::BIGINT
                WHEN a.deleted_at IS NULL AND (a.metadata->>'size') ~ '^[0-9]+$' THEN (a.metadata->>'size')::BIGINT
                ELSE 0
            END
        ), 0)::BIGINT AS total_size_bytes
    FROM assets a
    INNER JOIN storage_roots sr ON sr.id = a.storage_root_id
    WHERE sr.storage_connection_id = sc.id
) asset_stats ON TRUE
ORDER BY sc.is_default DESC, sc.canonical_uri ASC
"#;

async fn select_fallback_default(
    tx: &mut Transaction<'_, Postgres>,
    excluded_id: Option<Uuid>,
) -> AppResult<()> {
    sqlx::query(
        r#"
        UPDATE storage_connections
        SET is_default = TRUE, updated_at = NOW()
        WHERE id = (
            SELECT id
            FROM storage_connections
            WHERE enabled AND ($1::uuid IS NULL OR id <> $1)
            ORDER BY created_at, id
            LIMIT 1
        )
        AND NOT EXISTS (
            SELECT 1
            FROM storage_connections
            WHERE is_default AND enabled AND ($1::uuid IS NULL OR id <> $1)
        )
        "#,
    )
    .bind(excluded_id)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn get_connection(state: &AppState, id: Uuid) -> AppResult<StorageConnectionRecord> {
    let sql = format!("SELECT * FROM ({STORAGE_CONNECTION_SELECT}) connections WHERE id = $1");
    sqlx::query_as::<_, StorageConnectionRecord>(&sql)
        .bind(id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or_else(|| AppError::NotFound("storage connection not found".to_string()))
}

async fn validate_name(
    state: &AppState,
    value: &str,
    excluded_id: Option<Uuid>,
) -> AppResult<String> {
    let name = value.trim();
    if name.is_empty() {
        return Err(AppError::BadRequest(
            "storage connection name is required".to_string(),
        ));
    }
    let existing: Option<Uuid> = sqlx::query_scalar(
        "SELECT id FROM storage_connections WHERE lower(name) = lower($1) AND ($2::uuid IS NULL OR id <> $2)",
    )
    .bind(name)
    .bind(excluded_id)
    .fetch_optional(&state.pool)
    .await?;
    if existing.is_some() {
        return Err(AppError::Conflict(
            "storage connection name already exists".to_string(),
        ));
    }
    Ok(name.to_string())
}

async fn ensure_unique_location(
    state: &AppState,
    kind: StorageRootKind,
    canonical_uri: &str,
    excluded_id: Option<Uuid>,
) -> AppResult<()> {
    let existing: Option<Uuid> = sqlx::query_scalar(
        "SELECT id FROM storage_connections WHERE kind = $1 AND lower(canonical_uri) = lower($2) AND ($3::uuid IS NULL OR id <> $3)",
    )
    .bind(kind.as_str())
    .bind(canonical_uri)
    .bind(excluded_id)
    .fetch_optional(&state.pool)
    .await?;
    if existing.is_some() {
        return Err(AppError::Conflict(
            "storage location already exists".to_string(),
        ));
    }
    Ok(())
}

fn validate_location(
    state: &AppState,
    kind: StorageRootKind,
    canonical_uri: &str,
    windows_unc_path: Option<String>,
    macos_smb_url: Option<String>,
    windows_aliases: &[String],
    macos_aliases: &[String],
) -> AppResult<crate::path_resolver::ResolvedStorageLocation> {
    validate_aliases_with_policy(windows_aliases, state.config.allow_personal_storage_paths)?;
    validate_aliases_with_policy(macos_aliases, state.config.allow_personal_storage_paths)?;
    resolve_storage_location_with_policy(
        kind,
        canonical_uri,
        windows_unc_path,
        macos_smb_url,
        state.config.allow_personal_storage_paths,
    )
}

fn require_server_manager(user: &AuthUser) -> AppResult<()> {
    if user.role.can_manage_server() {
        Ok(())
    } else {
        Err(AppError::Forbidden)
    }
}
