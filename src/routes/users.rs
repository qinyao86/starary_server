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
use serde::Deserialize;
use uuid::Uuid;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateUserRequest {
    email: String,
    password: String,
    display_name: Option<String>,
    #[serde(default)]
    role: Option<Role>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateUserRequest {
    display_name: Option<String>,
    #[serde(default)]
    role: Option<Role>,
    is_active: Option<bool>,
    password: Option<String>,
}

pub async fn list_users(
    State(state): State<AppState>,
    user: AuthUser,
) -> AppResult<Json<Vec<UserRecord>>> {
    if !user.role.can_manage_server() {
        return Err(AppError::Forbidden);
    }

    let users = sqlx::query_as::<_, UserRecord>(
        r#"
        SELECT id, email, display_name, global_role, is_active, created_at, updated_at
        FROM users
        ORDER BY display_name ASC, email ASC
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
    if request.password.len() < 8 {
        return Err(AppError::BadRequest(
            "password must be at least 8 characters".to_string(),
        ));
    }

    let display_name = request
        .display_name
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| email.clone());

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

    if actor.role != Role::Owner && (current_role == Role::Owner || request.role == Some(Role::Owner)) {
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
        .unwrap_or(&target.display_name);

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
    .bind(next_display_name)
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

async fn ensure_another_active_owner(state: &AppState, user_id: Uuid) -> AppResult<()> {
    let count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM users
        WHERE id <> $1 AND global_role = 'owner' AND is_active = TRUE
        "#,
    )
    .bind(user_id)
    .fetch_one(&state.pool)
    .await?;

    if count == 0 {
        return Err(AppError::Conflict(
            "at least one active owner is required".to_string(),
        ));
    }

    Ok(())
}
