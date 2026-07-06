use crate::{
    auth::{hash_password, AuthUser},
    error::{AppError, AppResult},
    models::{Role, UserRecord},
    state::AppState,
};
use axum::{extract::State, Json};
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
