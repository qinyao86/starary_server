use crate::{
    auth::AuthUser,
    error::{AppError, AppResult},
    system_avatars,
};
use axum::{
    extract::Path,
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    Json,
};

pub async fn list_system_avatars(_user: AuthUser) -> Json<Vec<system_avatars::SystemAvatarOption>> {
    Json(system_avatars::options())
}

pub async fn read_system_avatar(Path(key): Path<String>) -> AppResult<Response> {
    let avatar = system_avatars::get(key.trim())
        .ok_or_else(|| AppError::NotFound("avatar not found".to_string()))?;
    Ok((
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "image/svg+xml; charset=utf-8"),
            (header::CACHE_CONTROL, "public, max-age=31536000, immutable"),
        ],
        avatar.bytes,
    )
        .into_response())
}
