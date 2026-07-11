use crate::{
    auth::{issue_token, verify_password, AuthUser},
    error::{AppError, AppResult},
    models::UserWithPassword,
    routes::users::guards::ensure_unique_display_name,
    state::AppState,
};
use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};

use super::access::ensure_library_membership;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginRequest {
    email: String,
    password: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PresenceRequest {
    library_id: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCurrentUserRequest {
    display_name: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginResponse {
    access_token: String,
    token_type: &'static str,
    user: CurrentUserResponse,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CurrentUserResponse {
    id: String,
    email: String,
    display_name: String,
    role: String,
}

pub async fn login(
    State(state): State<AppState>,
    Json(request): Json<LoginRequest>,
) -> AppResult<Json<LoginResponse>> {
    let email = request.email.trim().to_ascii_lowercase();
    let user = sqlx::query_as::<_, UserWithPassword>(
        r#"
        SELECT id, email, display_name, password_hash, global_role, is_active
        FROM users
        WHERE email = $1
        "#,
    )
    .bind(email)
    .fetch_optional(&state.pool)
    .await?
    .ok_or(AppError::Unauthorized)?;

    if !user.is_active || !verify_password(&request.password, &user.password_hash) {
        return Err(AppError::Unauthorized);
    }

    let token = issue_token(&state, &user)?;

    sqlx::query(
        r#"
        UPDATE users
        SET last_login_at = NOW(), last_seen_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(user.id)
    .execute(&state.pool)
    .await?;

    Ok(Json(LoginResponse {
        access_token: token,
        token_type: "Bearer",
        user: CurrentUserResponse {
            id: user.id.to_string(),
            email: user.email,
            display_name: user.display_name,
            role: user.global_role,
        },
    }))
}

pub async fn me(user: AuthUser) -> Json<CurrentUserResponse> {
    Json(CurrentUserResponse {
        id: user.id.to_string(),
        email: user.email,
        display_name: user.display_name,
        role: user.role.to_string(),
    })
}

pub async fn update_me(
    State(state): State<AppState>,
    user: AuthUser,
    Json(request): Json<UpdateCurrentUserRequest>,
) -> AppResult<Json<CurrentUserResponse>> {
    let display_name = request.display_name.trim();
    if display_name.is_empty() {
        return Err(AppError::BadRequest("display name is required".to_string()));
    }

    ensure_unique_display_name(&state, display_name, Some(user.id)).await?;

    let display_name: String = sqlx::query_scalar(
        r#"
        UPDATE users
        SET display_name = $2, updated_at = NOW()
        WHERE id = $1
        RETURNING display_name
        "#,
    )
    .bind(user.id)
    .bind(display_name)
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(CurrentUserResponse {
        id: user.id.to_string(),
        email: user.email,
        display_name,
        role: user.role.to_string(),
    }))
}

pub async fn update_presence(
    State(state): State<AppState>,
    user: AuthUser,
    Json(request): Json<PresenceRequest>,
) -> AppResult<StatusCode> {
    if let Some(library_id) = request.library_id.as_deref() {
        ensure_library_membership(&state, &user, library_id).await?;
    }

    sqlx::query(
        r#"
        UPDATE users
        SET
            last_seen_at = NOW(),
            last_seen_library_id = COALESCE($2, last_seen_library_id)
        WHERE id = $1
        "#,
    )
    .bind(user.id)
    .bind(request.library_id)
    .execute(&state.pool)
    .await?;

    Ok(StatusCode::NO_CONTENT)
}
