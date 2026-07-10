use crate::{
    error::{AppError, AppResult},
    models::{Role, UserRecord, UserWithPassword},
    state::AppState,
};
use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use async_trait::async_trait;
use axum::{
    extract::FromRequestParts,
    http::{header::AUTHORIZATION, request::Parts},
};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use rand_core::OsRng;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct AuthUser {
    pub id: Uuid,
    pub email: String,
    pub display_name: String,
    pub role: Role,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct Claims {
    sub: String,
    role: String,
    exp: usize,
}

pub fn hash_password(password: &str) -> anyhow::Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)?
        .to_string();
    Ok(hash)
}

pub fn verify_password(password: &str, password_hash: &str) -> bool {
    let parsed = match PasswordHash::new(password_hash) {
        Ok(parsed) => parsed,
        Err(_) => return false,
    };

    Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok()
}

pub fn issue_token(state: &AppState, user: &UserWithPassword) -> AppResult<String> {
    issue_token_for_user(state, user.id, &user.global_role)
}

pub fn issue_token_for_user(state: &AppState, user_id: Uuid, role: &str) -> AppResult<String> {
    let exp = (Utc::now() + Duration::hours(state.config.token_ttl_hours)).timestamp() as usize;
    let claims = Claims {
        sub: user_id.to_string(),
        role: role.to_string(),
        exp,
    };

    Ok(encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(state.config.jwt_secret.as_bytes()),
    )?)
}

pub async fn auth_user_from_token(state: &AppState, token: &str) -> AppResult<AuthUser> {
    let token = decode::<Claims>(
        token,
        &DecodingKey::from_secret(state.config.jwt_secret.as_bytes()),
        &Validation::default(),
    )
    .map_err(|_| AppError::Unauthorized)?;

    let user_id = Uuid::parse_str(&token.claims.sub).map_err(|_| AppError::Unauthorized)?;
    let user = sqlx::query_as::<_, UserRecord>(
        r#"
        SELECT id, email, display_name, global_role, is_active, created_at, updated_at
        FROM users
        WHERE id = $1
        "#,
    )
    .bind(user_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or(AppError::Unauthorized)?;

    if !user.is_active {
        return Err(AppError::Unauthorized);
    }

    let role = user
        .global_role
        .parse::<Role>()
        .map_err(|_| AppError::Unauthorized)?;

    Ok(AuthUser {
        id: user.id,
        email: user.email,
        display_name: user.display_name,
        role,
    })
}

#[async_trait]
impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let token = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.strip_prefix("Bearer "))
            .ok_or(AppError::Unauthorized)?;

        auth_user_from_token(state, token).await
    }
}
