use crate::{
    auth::AuthUser,
    error::{AppError, AppResult},
    models::{LibraryRecord, LibraryWithRole, Role},
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
use requests::{CreateLibraryRequest, UpdateLibraryRequest};

fn normalize_optional_text(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

async fn ensure_unique_library_display_name(
    state: &AppState,
    display_name: &str,
    excluded_library_id: Option<Uuid>,
) -> AppResult<()> {
    let existing_library_id: Option<Uuid> = sqlx::query_scalar(
        r#"
        SELECT id
        FROM libraries
        WHERE deleted_at IS NULL
          AND lower(display_name) = lower($1)
          AND ($2::uuid IS NULL OR id <> $2)
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

    let library_id = Uuid::new_v4();
    let mut tx = state.pool.begin().await?;

    let library = sqlx::query_as::<_, LibraryRecord>(
        r#"
        INSERT INTO libraries (id, display_name, description, icon_url, created_by_user_id)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id, display_name, description, icon_url, created_by_user_id, created_at, updated_at
        "#,
    )
    .bind(library_id)
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
    .bind(library_id)
    .bind(user.id)
    .bind(Role::LibraryManager.as_str())
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO activity_log (id, library_id, actor_user_id, action, target_type, target_id)
        VALUES ($1, $2, $3, 'library.created', 'library', $2)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(library_id)
    .bind(user.id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(Json(library))
}

pub async fn update_library(
    State(state): State<AppState>,
    user: AuthUser,
    Path(library_id): Path<Uuid>,
    Json(request): Json<UpdateLibraryRequest>,
) -> AppResult<Json<LibraryRecord>> {
    ensure_library_manager(&state, &user, library_id).await?;

    let display_name = request.display_name.trim();
    if display_name.is_empty() {
        return Err(AppError::BadRequest("library name is required".to_string()));
    }
    ensure_unique_library_display_name(&state, display_name, Some(library_id)).await?;
    let description = normalize_optional_text(request.description);
    let icon_url = normalize_optional_text(request.icon_url);

    let mut tx = state.pool.begin().await?;
    let library = sqlx::query_as::<_, LibraryRecord>(
        r#"
        UPDATE libraries
        SET display_name = $2, description = $3, icon_url = $4, updated_at = NOW()
        WHERE id = $1 AND deleted_at IS NULL
        RETURNING id, display_name, description, icon_url, created_by_user_id, created_at, updated_at
        "#,
    )
    .bind(library_id)
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
    .bind(library_id)
    .bind(user.id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(Json(library))
}

pub async fn delete_library(
    State(state): State<AppState>,
    user: AuthUser,
    Path(library_id): Path<Uuid>,
) -> AppResult<StatusCode> {
    ensure_library_manager(&state, &user, library_id).await?;

    let mut tx = state.pool.begin().await?;
    let deleted = sqlx::query(
        r#"
        UPDATE libraries
        SET deleted_at = NOW(), updated_at = NOW()
        WHERE id = $1 AND deleted_at IS NULL
        "#,
    )
    .bind(library_id)
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
    .bind(library_id)
    .bind(user.id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(StatusCode::NO_CONTENT)
}
