use crate::{
    auth::AuthUser,
    error::{AppError, AppResult},
    ids::generate_id,
    models::{LibraryRecord, LibraryStatusRecord, LibraryWithRole, Role, StorageRootKind},
    path_resolver::{
        ensure_storage_location_exists, ensure_storage_namespace_exists,
        normalize_existing_storage_namespace, normalize_storage_namespace,
        resolve_storage_location_with_policy, resolve_storage_namespace_with_policy,
        validate_aliases_with_policy, validate_storage_root_with_policy,
    },
    routes::access::ensure_library_manager,
    state::AppState,
};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use sqlx::{FromRow, Postgres, Transaction};
use std::str::FromStr;
use uuid::Uuid;

mod queries;
mod requests;

use queries::{
    list_libraries_for_member, list_libraries_for_server_manager, list_library_statuses_for_member,
    list_library_statuses_for_server_manager,
};
use requests::{
    CreateDefaultStorageRootRequest, CreateLibraryRequest, StorageBindingRequest,
    UpdateLibraryEnabledRequest, UpdateLibraryRequest,
};

#[derive(FromRow)]
struct StorageConnectionLocation {
    id: Uuid,
    kind: String,
    canonical_uri: String,
    windows_unc_path: Option<String>,
    windows_mapped_drive_aliases: serde_json::Value,
    macos_smb_url: Option<String>,
    macos_mount_aliases: serde_json::Value,
    enabled: bool,
}

fn normalize_optional_text(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn validate_default_storage_root(
    request: &CreateDefaultStorageRootRequest,
    fallback_name: &str,
    allow_personal_paths: bool,
) -> AppResult<String> {
    let name = request
        .name
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(fallback_name)
        .trim();
    if name.is_empty() {
        return Err(AppError::BadRequest(
            "workspace name is required".to_string(),
        ));
    }

    validate_storage_root_with_policy(request.kind, &request.canonical_uri, allow_personal_paths)?;
    validate_aliases_with_policy(&request.windows_mapped_drive_aliases, allow_personal_paths)?;
    validate_aliases_with_policy(&request.macos_mount_aliases, allow_personal_paths)?;
    if let Some(value) = &request.windows_unc_path {
        validate_aliases_with_policy(std::slice::from_ref(value), allow_personal_paths)?;
    }
    if let Some(value) = &request.macos_smb_url {
        validate_aliases_with_policy(std::slice::from_ref(value), allow_personal_paths)?;
    }

    Ok(name.to_string())
}

async fn ensure_unique_library_display_name(
    state: &AppState,
    display_name: &str,
    excluded_library_id: Option<&str>,
) -> AppResult<()> {
    let existing_library_id: Option<String> = sqlx::query_scalar(
        r#"
        SELECT id
        FROM libraries
        WHERE deleted_at IS NULL
          AND lower(display_name) = lower($1)
          AND ($2::text IS NULL OR id <> $2)
        LIMIT 1
        "#,
    )
    .bind(display_name)
    .bind(excluded_library_id)
    .fetch_optional(&state.pool)
    .await?;

    if existing_library_id.is_some() {
        return Err(AppError::Conflict(
            "library display name already exists".to_string(),
        ));
    }

    Ok(())
}

pub async fn list_libraries(
    State(state): State<AppState>,
    user: AuthUser,
) -> AppResult<Json<Vec<LibraryWithRole>>> {
    let libraries = if user.role.can_manage_server() {
        list_libraries_for_server_manager(&state, user.id, user.role.as_str()).await?
    } else {
        list_libraries_for_member(&state, user.id).await?
    };

    Ok(Json(libraries))
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryStatusResponse {
    libraries: Vec<LibraryStatusRecord>,
}

pub async fn list_library_statuses(
    State(state): State<AppState>,
    user: AuthUser,
) -> AppResult<Json<LibraryStatusResponse>> {
    let libraries = if user.role.can_manage_server() {
        list_library_statuses_for_server_manager(&state).await?
    } else {
        list_library_statuses_for_member(&state, user.id).await?
    };

    Ok(Json(LibraryStatusResponse { libraries }))
}

pub async fn create_library(
    State(state): State<AppState>,
    user: AuthUser,
    Json(request): Json<CreateLibraryRequest>,
) -> AppResult<Json<LibraryRecord>> {
    if !user.role.can_create_library() {
        return Err(AppError::Forbidden);
    }

    let display_name = request.display_name.trim();
    if display_name.is_empty() {
        return Err(AppError::BadRequest("library name is required".to_string()));
    }
    ensure_unique_library_display_name(&state, display_name, None).await?;
    let description = normalize_optional_text(request.description);
    let icon_url = normalize_optional_text(request.icon_url);
    if request.default_storage_root.is_some() && request.storage_binding.is_some() {
        return Err(AppError::BadRequest(
            "choose either a new storage location or an existing storage connection".to_string(),
        ));
    }
    let default_storage_root_name = request
        .default_storage_root
        .as_ref()
        .map(|root| {
            validate_default_storage_root(
                root,
                display_name,
                state.config.allow_personal_storage_paths,
            )
        })
        .transpose()?;

    let library_id = generate_id("lib_");
    let mut tx = state.pool.begin().await?;

    let library = sqlx::query_as::<_, LibraryRecord>(
        r#"
        INSERT INTO libraries (id, display_name, description, icon_url, created_by_user_id)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id, display_name, description, icon_url, enabled, created_by_user_id, created_at, updated_at
        "#,
    )
    .bind(&library_id)
    .bind(display_name)
    .bind(description)
    .bind(icon_url)
    .bind(user.id)
    .fetch_one(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO library_memberships (library_id, user_id, role)
        VALUES ($1, $2, $3)
        "#,
    )
    .bind(&library_id)
    .bind(user.id)
    .bind(Role::LibraryManager.as_str())
    .execute(&mut *tx)
    .await?;

    if let Some(default_storage_root) = request.default_storage_root {
        let root_name = default_storage_root_name.as_deref().unwrap_or("Workspace");
        let root_location = resolve_storage_location_with_policy(
            default_storage_root.kind,
            &default_storage_root.canonical_uri,
            default_storage_root.windows_unc_path.clone(),
            default_storage_root.macos_smb_url.clone(),
            state.config.allow_personal_storage_paths,
        )?;
        ensure_storage_location_exists(default_storage_root.kind, &root_location)?;
        let connection_id = Uuid::new_v4();
        let connection_name = unique_connection_name(&mut tx, display_name).await?;
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
        .bind(connection_id)
        .bind(connection_name)
        .bind(default_storage_root.kind.as_str())
        .bind(root_location.canonical_uri)
        .bind(root_location.windows_unc_path)
        .bind(serde_json::to_value(
            default_storage_root.windows_mapped_drive_aliases,
        )?)
        .bind(root_location.macos_smb_url)
        .bind(serde_json::to_value(
            default_storage_root.macos_mount_aliases,
        )?)
        .bind(user.id)
        .execute(&mut *tx)
        .await?;
        bind_library_storage(
            &state,
            &mut tx,
            &library_id,
            user.id,
            StorageBindingRequest {
                connection_id,
                namespace: None,
            },
            root_name,
        )
        .await?;
    } else if let Some(binding) = request.storage_binding {
        bind_library_storage(&state, &mut tx, &library_id, user.id, binding, display_name).await?;
    }

    sqlx::query(
        r#"
        INSERT INTO activity_log (id, library_id, actor_user_id, action, target_type, target_id)
        VALUES ($1, $2, $3, 'library.created', 'library', $2)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(&library_id)
    .bind(user.id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(Json(library))
}

pub async fn update_library(
    State(state): State<AppState>,
    user: AuthUser,
    Path(library_id): Path<String>,
    Json(request): Json<UpdateLibraryRequest>,
) -> AppResult<Json<LibraryRecord>> {
    ensure_library_manager(&state, &user, &library_id).await?;

    let display_name = request.display_name.trim();
    if display_name.is_empty() {
        return Err(AppError::BadRequest("library name is required".to_string()));
    }
    ensure_unique_library_display_name(&state, display_name, Some(&library_id)).await?;
    let description = normalize_optional_text(request.description);
    let icon_url = normalize_optional_text(request.icon_url);

    let mut tx = state.pool.begin().await?;
    let library = sqlx::query_as::<_, LibraryRecord>(
        r#"
        UPDATE libraries
        SET display_name = $2, description = $3, icon_url = $4, updated_at = NOW()
        WHERE id = $1 AND deleted_at IS NULL
        RETURNING id, display_name, description, icon_url, enabled, created_by_user_id, created_at, updated_at
        "#,
    )
    .bind(&library_id)
    .bind(display_name)
    .bind(description)
    .bind(icon_url)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| AppError::NotFound("library not found".to_string()))?;

    if let Some(binding) = request.storage_binding {
        let existing_root: Option<(Uuid, Uuid, String)> = sqlx::query_as(
            "SELECT id, storage_connection_id, namespace FROM storage_roots WHERE library_id = $1 ORDER BY created_at ASC LIMIT 1",
        )
        .bind(&library_id)
        .fetch_optional(&mut *tx)
        .await?;
        let namespace = match (binding.namespace.as_deref(), existing_root.as_ref()) {
            (Some(value), _) => normalize_existing_storage_namespace(value)?,
            (None, Some((_, _, current_namespace))) => current_namespace.clone(),
            (None, None) => normalize_storage_namespace(None, &library_id)?,
        };
        let changed = existing_root
            .as_ref()
            .map(|(_, connection_id, current_namespace)| {
                *connection_id != binding.connection_id || current_namespace != &namespace
            })
            .unwrap_or(true);
        if changed {
            let active_assets: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM assets WHERE library_id = $1 AND deleted_at IS NULL",
            )
            .bind(&library_id)
            .fetch_one(&mut *tx)
            .await?;
            if active_assets > 0 {
                return Err(AppError::Conflict(
                    "library storage cannot be changed while assets still reference its workspace"
                        .to_string(),
                ));
            }
            if let Some((root_id, _, _)) = existing_root {
                sqlx::query("DELETE FROM storage_roots WHERE id = $1")
                    .bind(root_id)
                    .execute(&mut *tx)
                    .await?;
            }
            bind_library_storage(
                &state,
                &mut tx,
                &library_id,
                user.id,
                StorageBindingRequest {
                    connection_id: binding.connection_id,
                    namespace: Some(namespace),
                },
                display_name,
            )
            .await?;
        }
    }

    sqlx::query(
        r#"
        INSERT INTO activity_log (id, library_id, actor_user_id, action, target_type, target_id)
        VALUES ($1, $2, $3, 'library.updated', 'library', $2)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(&library_id)
    .bind(user.id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(Json(library))
}

async fn bind_library_storage(
    state: &AppState,
    tx: &mut Transaction<'_, Postgres>,
    library_id: &str,
    user_id: Uuid,
    binding: StorageBindingRequest,
    workspace_name: &str,
) -> AppResult<()> {
    let connection = get_enabled_connection(tx, binding.connection_id).await?;
    let kind = StorageRootKind::from_str(&connection.kind).map_err(AppError::BadRequest)?;
    let namespace = normalize_storage_namespace(binding.namespace.as_deref(), library_id)?;
    let location = resolve_storage_namespace_with_policy(
        kind,
        &connection.canonical_uri,
        &namespace,
        connection.windows_unc_path,
        connection.macos_smb_url,
        state.config.allow_personal_storage_paths,
    )?;
    ensure_storage_namespace_exists(kind, &location)?;
    let root_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO storage_roots (
            id, library_id, storage_connection_id, namespace, name, kind,
            canonical_uri, windows_unc_path, windows_mapped_drive_aliases,
            macos_smb_url, macos_mount_aliases, created_by_user_id
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9::jsonb, $10, $11::jsonb, $12)
        "#,
    )
    .bind(root_id)
    .bind(library_id)
    .bind(connection.id)
    .bind(namespace)
    .bind(workspace_name)
    .bind(kind.as_str())
    .bind(location.canonical_uri)
    .bind(location.windows_unc_path)
    .bind(connection.windows_mapped_drive_aliases)
    .bind(location.macos_smb_url)
    .bind(connection.macos_mount_aliases)
    .bind(user_id)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn get_enabled_connection(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
) -> AppResult<StorageConnectionLocation> {
    sqlx::query_as::<_, StorageConnectionLocation>(
        r#"
        SELECT id, kind, canonical_uri, windows_unc_path,
               windows_mapped_drive_aliases, macos_smb_url, macos_mount_aliases, enabled
        FROM storage_connections WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(&mut **tx)
    .await?
    .filter(|connection| connection.enabled)
    .ok_or_else(|| AppError::BadRequest("storage connection is missing or disabled".to_string()))
}

async fn unique_connection_name(
    tx: &mut Transaction<'_, Postgres>,
    library_name: &str,
) -> AppResult<String> {
    let base = format!("{library_name} Storage");
    for suffix in 0..1000 {
        let candidate = if suffix == 0 {
            base.clone()
        } else {
            format!("{base} {suffix}")
        };
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM storage_connections WHERE lower(name) = lower($1))",
        )
        .bind(&candidate)
        .fetch_one(&mut **tx)
        .await?;
        if !exists {
            return Ok(candidate);
        }
    }
    Err(AppError::Conflict(
        "could not allocate a unique storage connection name".to_string(),
    ))
}

pub async fn update_library_enabled(
    State(state): State<AppState>,
    user: AuthUser,
    Path(library_id): Path<String>,
    Json(request): Json<UpdateLibraryEnabledRequest>,
) -> AppResult<Json<LibraryRecord>> {
    ensure_library_manager(&state, &user, &library_id).await?;

    let mut tx = state.pool.begin().await?;
    let library = sqlx::query_as::<_, LibraryRecord>(
        r#"
        UPDATE libraries
        SET enabled = $2, updated_at = NOW()
        WHERE id = $1 AND deleted_at IS NULL
        RETURNING id, display_name, description, icon_url, enabled, created_by_user_id, created_at, updated_at
        "#,
    )
    .bind(&library_id)
    .bind(request.enabled)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| AppError::NotFound("library not found".to_string()))?;

    sqlx::query(
        r#"
        INSERT INTO activity_log (id, library_id, actor_user_id, action, target_type, target_id, details)
        VALUES ($1, $2, $3, $4, 'library', $2, $5::jsonb)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(&library_id)
    .bind(user.id)
    .bind(if request.enabled {
        "library.enabled"
    } else {
        "library.disabled"
    })
    .bind(serde_json::json!({ "enabled": request.enabled }))
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(Json(library))
}

pub async fn delete_library(
    State(state): State<AppState>,
    user: AuthUser,
    Path(library_id): Path<String>,
) -> AppResult<StatusCode> {
    ensure_library_manager(&state, &user, &library_id).await?;

    let mut tx = state.pool.begin().await?;
    let deleted = sqlx::query(
        r#"
        UPDATE libraries
        SET deleted_at = NOW(), updated_at = NOW()
        WHERE id = $1 AND deleted_at IS NULL
        "#,
    )
    .bind(&library_id)
    .execute(&mut *tx)
    .await?;

    if deleted.rows_affected() == 0 {
        return Err(AppError::NotFound("library not found".to_string()));
    }

    sqlx::query(
        r#"
        INSERT INTO activity_log (id, library_id, actor_user_id, action, target_type, target_id)
        VALUES ($1, $2, $3, 'library.deleted', 'library', $2)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(&library_id)
    .bind(user.id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(StatusCode::NO_CONTENT)
}
