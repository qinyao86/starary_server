use crate::{
    auth::{hash_password, AuthUser},
    error::{AppError, AppResult},
    models::{Role, UserRecord},
    state::AppState,
};
use axum::{
    extract::{Path, State},
    Json,
};
use uuid::Uuid;

pub(crate) mod guards;
mod requests;

use guards::{ensure_another_active_owner, ensure_unique_display_name};
use requests::{CreateUserRequest, UpdateUserRequest};

pub async fn list_users(
    State(state): State<AppState>,
    user: AuthUser,
) -> AppResult<Json<Vec<UserRecord>>> {
    if !user.role.can_manage_server() {
        return Err(AppError::Forbidden);
    }

    let users = sqlx::query_as::<_, UserRecord>(
        r#"
        SELECT
            u.id,
            u.email,
            u.display_name,
            u.global_role,
            u.is_active,
            u.last_login_at,
            u.last_seen_at,
            u.last_seen_library_id,
            l.display_name AS last_seen_library_name,
            u.created_at,
            u.updated_at
        FROM users u
        LEFT JOIN libraries l ON l.id = u.last_seen_library_id AND l.deleted_at IS NULL
        ORDER BY
            u.last_seen_at DESC NULLS LAST,
            u.display_name ASC,
            u.email ASC
        "#,
    )
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(users))
}

pub async fn create_user(
    State(state): State<AppState>,
    actor: AuthUser,
    Json(request): Json<CreateUserRequest>,
) -> AppResult<Json<UserRecord>> {
    if !actor.role.can_manage_server() {
        return Err(AppError::Forbidden);
    }

    let role = request.role.unwrap_or(Role::Viewer);
    if role == Role::Owner && actor.role != Role::Owner {
        return Err(AppError::Forbidden);
    }

    let email = request.email.trim().to_ascii_lowercase();
    if email.is_empty() || !email.contains('@') {
        return Err(AppError::BadRequest("valid email is required".to_string()));
    }
    let existing_user: Option<Uuid> = sqlx::query_scalar("SELECT id FROM users WHERE email = $1")
        .bind(&email)
        .fetch_optional(&state.pool)
        .await?;
    if existing_user.is_some() {
        return Err(AppError::Conflict("email already exists".to_string()));
    }
    if request.password.len() < 8 {
        return Err(AppError::BadRequest(
            "password must be at least 8 characters".to_string(),
        ));
    }

    let display_name = request
        .display_name
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| email.clone())
        .trim()
        .to_string();
    ensure_unique_display_name(&state, &display_name, None).await?;

    let user_id = Uuid::new_v4();
    let password_hash = hash_password(&request.password)?;
    let mut tx = state.pool.begin().await?;

    let user = sqlx::query_as::<_, UserRecord>(
        r#"
        INSERT INTO users (id, email, display_name, password_hash, global_role)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id, email, display_name, global_role, is_active, created_at, updated_at
        "#,
    )
    .bind(user_id)
    .bind(email)
    .bind(display_name)
    .bind(password_hash)
    .bind(role.as_str())
    .fetch_one(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO activity_log (id, actor_user_id, action, target_type, target_id)
        VALUES ($1, $2, 'user.created', 'user', $3)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(actor.id)
    .bind(user_id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(Json(user))
}

pub async fn update_user(
    State(state): State<AppState>,
    actor: AuthUser,
    Path(user_id): Path<Uuid>,
    Json(request): Json<UpdateUserRequest>,
) -> AppResult<Json<UserRecord>> {
    if !actor.role.can_manage_server() {
        return Err(AppError::Forbidden);
    }

    let target = sqlx::query_as::<_, UserRecord>(
        r#"
        SELECT id, email, display_name, global_role, is_active, created_at, updated_at
        FROM users
        WHERE id = $1
        "#,
    )
    .bind(user_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::NotFound("user not found".to_string()))?;

    let current_role = target
        .global_role
        .parse::<Role>()
        .map_err(|_| AppError::Internal(anyhow::anyhow!("stored user role is invalid")))?;

    if actor.role != Role::Owner
        && (current_role == Role::Owner || request.role == Some(Role::Owner))
    {
        return Err(AppError::Forbidden);
    }

    let next_role = request.role.unwrap_or(current_role);
    if next_role == Role::Owner && actor.role != Role::Owner {
        return Err(AppError::Forbidden);
    }

    let next_is_active = request.is_active.unwrap_or(target.is_active);
    if current_role == Role::Owner && (next_role != Role::Owner || !next_is_active) {
        ensure_another_active_owner(&state, user_id).await?;
    }

    if let Some(password) = &request.password {
        if !password.is_empty() && password.len() < 8 {
            return Err(AppError::BadRequest(
                "password must be at least 8 characters".to_string(),
            ));
        }
    }

    let next_display_name = request
        .display_name
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(&target.display_name)
        .to_string();
    ensure_unique_display_name(&state, &next_display_name, Some(user_id)).await?;

    let password_hash = match request.password.as_deref().map(str::trim) {
        Some(password) if !password.is_empty() => Some(hash_password(password)?),
        _ => None,
    };
    let password_changed = password_hash.is_some();

    let mut tx = state.pool.begin().await?;

    let user = sqlx::query_as::<_, UserRecord>(
        r#"
        UPDATE users
        SET
            display_name = $2,
            global_role = $3,
            is_active = $4,
            password_hash = COALESCE($5, password_hash),
            updated_at = NOW()
        WHERE id = $1
        RETURNING id, email, display_name, global_role, is_active, created_at, updated_at
        "#,
    )
    .bind(user_id)
    .bind(&next_display_name)
    .bind(next_role.as_str())
    .bind(next_is_active)
    .bind(password_hash.as_deref())
    .fetch_one(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO activity_log (id, actor_user_id, action, target_type, target_id, details)
        VALUES ($1, $2, 'user.updated', 'user', $3, $4::jsonb)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(actor.id)
    .bind(user_id)
    .bind(serde_json::json!({
        "role": next_role.as_str(),
        "isActive": next_is_active,
        "passwordChanged": password_changed
    }))
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(Json(user))
}
