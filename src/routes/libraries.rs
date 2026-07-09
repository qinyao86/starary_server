use crate::{
    auth::AuthUser,
    error::{AppError, AppResult},
    ids::generate_id,
    models::{LibraryRecord, LibraryWithRole, Role},
    path_resolver::{
        ensure_storage_location_exists, ensure_storage_namespace_exists,
        resolve_library_storage_namespace, resolve_storage_location, validate_aliases,
        validate_storage_root,
    },
    routes::access::ensure_library_manager,
    state::AppState,
};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use uuid::Uuid;

mod queries;
mod requests;

use queries::{list_libraries_for_member, list_libraries_for_server_manager};
use requests::{
    CreateDefaultStorageRootRequest, CreateLibraryRequest, UpdateLibraryEnabledRequest,
    UpdateLibraryRequest,
};

fn normalize_optional_text(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn validate_default_storage_root(
    request: &CreateDefaultStorageRootRequest,
    fallback_name: &str,
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

    validate_storage_root(request.kind, &request.canonical_uri)?;
    validate_aliases(&request.windows_mapped_drive_aliases)?;
    validate_aliases(&request.macos_mount_aliases)?;
    if let Some(value) = &request.windows_unc_path {
        validate_aliases(std::slice::from_ref(value))?;
    }
    if let Some(value) = &request.macos_smb_url {
        validate_aliases(std::slice::from_ref(value))?;
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
    let default_storage_root_name = request
        .default_storage_root
        .as_ref()
        .map(|root| validate_default_storage_root(root, display_name))
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
        let root_id = Uuid::new_v4();
        let root_name = default_storage_root_name.as_deref().unwrap_or("Workspace");
        let root_location = resolve_storage_location(
            default_storage_root.kind,
            &default_storage_root.canonical_uri,
            default_storage_root.windows_unc_path.clone(),
            default_storage_root.macos_smb_url.clone(),
        )?;
        ensure_storage_location_exists(default_storage_root.kind, &root_location)?;
        let location = resolve_library_storage_namespace(
            default_storage_root.kind,
            &root_location.canonical_uri,
            &library_id,
            root_location.windows_unc_path,
            root_location.macos_smb_url,
        )?;
        ensure_storage_namespace_exists(default_storage_root.kind, &location)?;

        sqlx::query(
            r#"
            INSERT INTO storage_roots (
                id,
                library_id,
                name,
                kind,
                canonical_uri,
                windows_unc_path,
                windows_mapped_drive_aliases,
                macos_smb_url,
                macos_mount_aliases,
                created_by_user_id
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7::jsonb, $8, $9::jsonb, $10)
            "#,
        )
        .bind(root_id)
        .bind(&library_id)
        .bind(root_name)
        .bind(default_storage_root.kind.as_str())
        .bind(location.canonical_uri)
        .bind(location.windows_unc_path)
        .bind(serde_json::to_value(
            default_storage_root.windows_mapped_drive_aliases,
        )?)
        .bind(location.macos_smb_url)
        .bind(serde_json::to_value(
            default_storage_root.macos_mount_aliases,
        )?)
        .bind(user.id)
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            r#"
            INSERT INTO activity_log (id, library_id, actor_user_id, action, target_type, target_id, details)
            VALUES ($1, $2, $3, 'storage_root.created', 'storage_root', $4, $5::jsonb)
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(&library_id)
        .bind(user.id)
        .bind(root_id.to_string())
        .bind(serde_json::json!({ "name": root_name, "kind": default_storage_root.kind.as_str() }))
        .execute(&mut *tx)
        .await?;
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
