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
use uuid::Uuid;

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
    _user: AuthUser,
) -> AppResult<Json<Vec<StorageConnectionRecord>>> {
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
    sqlx::query(
        r#"
        INSERT INTO storage_connections (
            id, name, kind, canonical_uri, windows_unc_path,
            windows_mapped_drive_aliases, macos_smb_url, macos_mount_aliases,
            created_by_user_id
        )
        VALUES ($1, $2, $3, $4, $5, $6::jsonb, $7, $8::jsonb, $9)
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
    .bind(user.id)
    .execute(&state.pool)
    .await?;

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
    if existing.library_count > 0 && location_changed {
        return Err(AppError::Conflict(
            "storage connection is used by libraries; detach them before changing its location"
                .to_string(),
        ));
    }

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
    .execute(&state.pool)
    .await?;

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
    sqlx::query("DELETE FROM storage_connections WHERE id = $1")
        .bind(id)
        .execute(&state.pool)
        .await?;
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
    COUNT(sr.id)::BIGINT AS library_count,
    sc.created_by_user_id,
    sc.created_at,
    sc.updated_at
FROM storage_connections sc
LEFT JOIN storage_roots sr ON sr.storage_connection_id = sc.id
GROUP BY sc.id
ORDER BY sc.canonical_uri ASC
"#;

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
