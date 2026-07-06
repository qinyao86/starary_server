use crate::{
    auth::{issue_token, verify_password, AuthUser},
    error::{AppError, AppResult},
    models::UserWithPassword,
    state::AppState,
};
use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginRequest {
    email: String,
    password: String,
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
