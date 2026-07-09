use crate::{
    auth::AuthUser,
    error::{AppError, AppResult},
    models::Role,
    state::AppState,
};

pub async fn ensure_library_access(
    state: &AppState,
    user: &AuthUser,
    library_id: &str,
) -> AppResult<Role> {
    if user.role.can_manage_server() {
        let exists: Option<String> =
            sqlx::query_scalar("SELECT id FROM libraries WHERE id = $1 AND deleted_at IS NULL")
                .bind(library_id)
                .fetch_optional(&state.pool)
                .await?;

        if exists.is_none() {
            return Err(AppError::NotFound("library not found".to_string()));
        }

        return Ok(user.role);
    }

    let role_value: Option<String> = sqlx::query_scalar(
        r#"
        SELECT m.role
        FROM library_memberships m
        INNER JOIN libraries l ON l.id = m.library_id
        WHERE m.library_id = $1 AND m.user_id = $2 AND l.deleted_at IS NULL
        "#,
    )
    .bind(library_id)
    .bind(user.id)
    .fetch_optional(&state.pool)
    .await?;

    role_value
        .ok_or(AppError::Forbidden)?
        .parse::<Role>()
        .map_err(|_| AppError::Forbidden)
}

pub async fn ensure_library_manager(
    state: &AppState,
    user: &AuthUser,
    library_id: &str,
) -> AppResult<Role> {
    let role = ensure_library_access(state, user, library_id).await?;
    if user.role.can_manage_server() || role.can_manage_library() {
        Ok(role)
    } else {
        Err(AppError::Forbidden)
    }
}

pub async fn ensure_library_write_access(
    state: &AppState,
    user: &AuthUser,
    library_id: &str,
) -> AppResult<Role> {
    // The first team version keeps structure editing permissive for members.
    // Tighten this single gate later when folder/tag permissions are finalized.
    ensure_library_access(state, user, library_id).await
}
