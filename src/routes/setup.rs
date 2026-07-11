use crate::{
    auth::{hash_password, issue_token_for_user},
    error::{AppError, AppResult},
    models::{Role, UserRecord},
    state::AppState,
};
use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetupStatusResponse {
    needs_owner: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateOwnerRequest {
    email: String,
    password: String,
    display_name: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateOwnerResponse {
    user: UserRecord,
    access_token: String,
    token_type: &'static str,
}

pub async fn setup_status(State(state): State<AppState>) -> AppResult<Json<SetupStatusResponse>> {
    let owner_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE global_role = 'owner'")
            .fetch_one(&state.pool)
            .await?;

    Ok(Json(SetupStatusResponse {
        needs_owner: owner_count == 0,
    }))
}

pub async fn create_owner(
    State(state): State<AppState>,
    Json(request): Json<CreateOwnerRequest>,
) -> AppResult<Json<CreateOwnerResponse>> {
    let email = request.email.trim().to_ascii_lowercase();
    if email.is_empty() || !email.contains('@') {
        return Err(AppError::BadRequest("valid email is required".to_string()));
    }
    if request.password.len() < 8 {
        return Err(AppError::BadRequest(
            "password must be at least 8 characters".to_string(),
        ));
    }

    let owner_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE global_role = 'owner'")
            .fetch_one(&state.pool)
            .await?;
    if owner_count > 0 {
        return Err(AppError::Conflict(
            "owner account already exists".to_string(),
        ));
    }

    let user_id = Uuid::new_v4();
    let password_hash = hash_password(&request.password)?;
    let display_name = request
        .display_name
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| email.split('@').next().unwrap_or("owner").to_string());

    let mut tx = state.pool.begin().await?;

    let user = sqlx::query_as::<_, UserRecord>(
        r#"
        INSERT INTO users (id, email, display_name, password_hash, global_role, last_login_at, last_seen_at)
        VALUES ($1, $2, $3, $4, $5, NOW(), NOW())
        RETURNING id, email, display_name, global_role, is_active, created_at, updated_at
        "#,
    )
    .bind(user_id)
    .bind(email)
    .bind(display_name)
    .bind(password_hash)
    .bind(Role::Owner.as_str())
    .fetch_one(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO activity_log (id, actor_user_id, action, target_type, target_id)
        VALUES ($1, $2, 'server.owner_created', 'user', $3)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(user_id)
    .bind(user_id.to_string())
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    let access_token = issue_token_for_user(&state, user_id, Role::Owner.as_str())?;

    Ok(Json(CreateOwnerResponse {
        user,
        access_token,
        token_type: "Bearer",
    }))
}
