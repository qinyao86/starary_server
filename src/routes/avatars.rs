use crate::{
    auth::AuthUser,
    error::{AppError, AppResult},
    models::{UserRecord, UserWithMemberships},
    state::AppState,
    system_avatars,
};
use axum::{
    extract::{Multipart, Path, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use std::path::{Path as FilePath, PathBuf};
use tokio::fs;
use uuid::Uuid;

use super::users::attach_library_memberships;

pub const CUSTOM_AVATAR_PREFIX: &str = "custom:";
const MAX_CUSTOM_AVATAR_BYTES: usize = 2 * 1024 * 1024;

pub fn is_custom_avatar_key(value: &str) -> bool {
    value
        .strip_prefix(CUSTOM_AVATAR_PREFIX)
        .is_some_and(system_avatars::is_valid_key)
}

pub fn custom_avatar_fallback_key(value: Option<&str>, user_id: Uuid) -> String {
    value
        .and_then(|value| value.strip_prefix(CUSTOM_AVATAR_PREFIX))
        .filter(|key| system_avatars::is_valid_key(key))
        .or_else(|| value.filter(|key| system_avatars::is_valid_key(key)))
        .unwrap_or_else(|| system_avatars::default_key_for_user(user_id))
        .to_string()
}

pub fn custom_avatar_path(storage_dir: &FilePath, user_id: Uuid) -> PathBuf {
    storage_dir
        .join("avatars")
        .join("users")
        .join(format!("{user_id}.webp"))
}

fn ensure_avatar_permission(actor: &AuthUser, user_id: Uuid) -> AppResult<()> {
    if actor.id == user_id || actor.role.can_manage_server() {
        Ok(())
    } else {
        Err(AppError::Forbidden)
    }
}

fn is_webp(bytes: &[u8]) -> bool {
    bytes.len() >= 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP"
}

async fn avatar_user_response(
    state: &AppState,
    user_id: Uuid,
) -> AppResult<Json<UserWithMemberships>> {
    let user = sqlx::query_as::<_, UserRecord>(
        r#"
        SELECT id, email, display_name, avatar_key, global_role, is_active, created_at, updated_at
        FROM users
        WHERE id = $1
        "#,
    )
    .bind(user_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::NotFound("user not found".to_string()))?;

    let mut users = attach_library_memberships(state, vec![user], None).await?;
    Ok(Json(users.pop().expect("avatar user response is present")))
}

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

pub async fn read_user_avatar(
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
) -> AppResult<Response> {
    let avatar_key: Option<Option<String>> =
        sqlx::query_scalar("SELECT avatar_key FROM users WHERE id = $1 AND is_active = TRUE")
            .bind(user_id)
            .fetch_optional(&state.pool)
            .await?;
    let avatar_key = avatar_key
        .ok_or_else(|| AppError::NotFound("user not found".to_string()))?
        .unwrap_or_else(|| system_avatars::default_key_for_user(user_id).to_string());

    if is_custom_avatar_key(&avatar_key) {
        let path = custom_avatar_path(&state.config.storage_dir, user_id);
        let bytes = fs::read(&path).await.map_err(|error| match error.kind() {
            std::io::ErrorKind::NotFound => {
                AppError::NotFound("custom avatar not found".to_string())
            }
            _ => AppError::Internal(anyhow::anyhow!(
                "could not read custom avatar {}: {error}",
                path.display()
            )),
        })?;
        return Ok((
            StatusCode::OK,
            [
                (header::CONTENT_TYPE, "image/webp"),
                (header::CACHE_CONTROL, "no-cache"),
            ],
            bytes,
        )
            .into_response());
    }

    let fallback_key = custom_avatar_fallback_key(Some(&avatar_key), user_id);
    let avatar = system_avatars::get(&fallback_key)
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

pub async fn upload_user_avatar(
    State(state): State<AppState>,
    actor: AuthUser,
    Path(user_id): Path<Uuid>,
    mut multipart: Multipart,
) -> AppResult<Json<UserWithMemberships>> {
    ensure_avatar_permission(&actor, user_id)?;

    let current_avatar_key: Option<Option<String>> =
        sqlx::query_scalar("SELECT avatar_key FROM users WHERE id = $1 AND is_active = TRUE")
            .bind(user_id)
            .fetch_optional(&state.pool)
            .await?;
    let current_avatar_key =
        current_avatar_key.ok_or_else(|| AppError::NotFound("user not found".to_string()))?;

    let mut avatar_bytes = None;
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|error| AppError::BadRequest(format!("invalid avatar upload: {error}")))?
    {
        if field.name() != Some("file") {
            continue;
        }
        let bytes = field
            .bytes()
            .await
            .map_err(|error| AppError::BadRequest(format!("invalid avatar file: {error}")))?;
        if bytes.len() > MAX_CUSTOM_AVATAR_BYTES {
            return Err(AppError::BadRequest(
                "avatar file exceeds the 2 MB limit".to_string(),
            ));
        }
        avatar_bytes = Some(bytes);
        break;
    }

    let avatar_bytes = avatar_bytes
        .filter(|bytes| !bytes.is_empty())
        .ok_or_else(|| AppError::BadRequest("avatar file is required".to_string()))?;
    if !is_webp(&avatar_bytes) {
        return Err(AppError::BadRequest(
            "avatar file must be a WebP image".to_string(),
        ));
    }

    let avatar_path = custom_avatar_path(&state.config.storage_dir, user_id);
    let avatar_dir = avatar_path
        .parent()
        .expect("custom avatar path always has a parent");
    fs::create_dir_all(avatar_dir).await.map_err(|error| {
        AppError::Internal(anyhow::anyhow!(
            "could not create avatar directory {}: {error}",
            avatar_dir.display()
        ))
    })?;
    let temp_path = avatar_dir.join(format!(".{user_id}-{}.tmp", Uuid::new_v4()));
    fs::write(&temp_path, &avatar_bytes)
        .await
        .map_err(|error| {
            AppError::Internal(anyhow::anyhow!(
                "could not write avatar file {}: {error}",
                temp_path.display()
            ))
        })?;
    if fs::try_exists(&avatar_path).await.unwrap_or(false) {
        fs::remove_file(&avatar_path).await.map_err(|error| {
            AppError::Internal(anyhow::anyhow!(
                "could not replace avatar file {}: {error}",
                avatar_path.display()
            ))
        })?;
    }
    fs::rename(&temp_path, &avatar_path)
        .await
        .map_err(|error| {
            AppError::Internal(anyhow::anyhow!(
                "could not finalize avatar file {}: {error}",
                avatar_path.display()
            ))
        })?;

    let fallback_key = custom_avatar_fallback_key(current_avatar_key.as_deref(), user_id);
    let custom_key = format!("{CUSTOM_AVATAR_PREFIX}{fallback_key}");
    sqlx::query("UPDATE users SET avatar_key = $2, updated_at = NOW() WHERE id = $1")
        .bind(user_id)
        .bind(custom_key)
        .execute(&state.pool)
        .await?;

    avatar_user_response(&state, user_id).await
}

pub async fn delete_user_avatar(
    State(state): State<AppState>,
    actor: AuthUser,
    Path(user_id): Path<Uuid>,
) -> AppResult<Json<UserWithMemberships>> {
    ensure_avatar_permission(&actor, user_id)?;

    let avatar_key: Option<Option<String>> =
        sqlx::query_scalar("SELECT avatar_key FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_optional(&state.pool)
            .await?;
    let avatar_key = avatar_key.ok_or_else(|| AppError::NotFound("user not found".to_string()))?;
    let fallback_key = custom_avatar_fallback_key(avatar_key.as_deref(), user_id);

    sqlx::query("UPDATE users SET avatar_key = $2, updated_at = NOW() WHERE id = $1")
        .bind(user_id)
        .bind(fallback_key)
        .execute(&state.pool)
        .await?;

    let avatar_path = custom_avatar_path(&state.config.storage_dir, user_id);
    if let Err(error) = fs::remove_file(&avatar_path).await {
        if error.kind() != std::io::ErrorKind::NotFound {
            tracing::warn!(
                path = %avatar_path.display(),
                %error,
                "could not remove reset custom avatar"
            );
        }
    }

    avatar_user_response(&state, user_id).await
}
