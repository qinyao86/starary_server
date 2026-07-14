use crate::{
    auth::{hash_password, issue_token, issue_token_for_user, verify_password, AuthUser},
    error::{AppError, AppResult},
    models::{UserRecord, UserWithPassword},
    routes::users::guards::ensure_unique_display_name,
    state::{AppState, BrowserHandoff},
};
use axum::{extract::State, http::StatusCode, Json};
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

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

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangeCurrentUserPasswordRequest {
    current_password: String,
    new_password: String,
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
pub struct BrowserHandoffResponse {
    code: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RedeemBrowserHandoffRequest {
    code: String,
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

pub async fn create_browser_handoff(
    State(state): State<AppState>,
    user: AuthUser,
) -> Json<BrowserHandoffResponse> {
    let code = Uuid::new_v4().simple().to_string();
    state.browser_handoffs.issue(
        code.clone(),
        BrowserHandoff {
            user_id: user.id,
            expires_at: Utc::now() + Duration::seconds(60),
        },
    );
    Json(BrowserHandoffResponse { code })
}

pub async fn redeem_browser_handoff(
    State(state): State<AppState>,
    Json(request): Json<RedeemBrowserHandoffRequest>,
) -> AppResult<Json<LoginResponse>> {
    let handoff = state
        .browser_handoffs
        .redeem(request.code.trim())
        .ok_or(AppError::Unauthorized)?;
    let user = sqlx::query_as::<_, UserRecord>(
        r#"
        SELECT id, email, display_name, global_role, is_active, created_at, updated_at
        FROM users
        WHERE id = $1
        "#,
    )
    .bind(handoff.user_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or(AppError::Unauthorized)?;
    if !user.is_active {
        return Err(AppError::Unauthorized);
    }

    let token = issue_token_for_user(&state, user.id, &user.global_role)?;
    sqlx::query("UPDATE users SET last_seen_at = NOW() WHERE id = $1")
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

pub async fn change_my_password(
    State(state): State<AppState>,
    user: AuthUser,
    Json(request): Json<ChangeCurrentUserPasswordRequest>,
) -> AppResult<StatusCode> {
    if request.new_password.len() < 8 {
        return Err(AppError::BadRequest(
            "password must be at least 8 characters".to_string(),
        ));
    }

    let password_hash: String = sqlx::query_scalar("SELECT password_hash FROM users WHERE id = $1")
        .bind(user.id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or(AppError::Unauthorized)?;
    if !verify_password(&request.current_password, &password_hash) {
        return Err(AppError::Unauthorized);
    }

    let new_password_hash = hash_password(&request.new_password)?;
    let mut tx = state.pool.begin().await?;
    sqlx::query(
        r#"
        UPDATE users
        SET password_hash = $2, updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(user.id)
    .bind(new_password_hash)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        r#"
        INSERT INTO activity_log (id, actor_user_id, action, target_type, target_id)
        VALUES ($1, $2, 'user.password_changed', 'user', $2)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(user.id)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    Ok(StatusCode::NO_CONTENT)
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
