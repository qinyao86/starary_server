use crate::{
    error::{AppError, AppResult},
    state::AppState,
};
use uuid::Uuid;

pub async fn ensure_unique_display_name(
    state: &AppState,
    display_name: &str,
    excluded_user_id: Option<Uuid>,
) -> AppResult<()> {
    let existing_user_id: Option<Uuid> = sqlx::query_scalar(
        r#"
        SELECT id
        FROM users
        WHERE lower(display_name) = lower($1)
          AND ($2::uuid IS NULL OR id <> $2)
        LIMIT 1
        "#,
    )
    .bind(display_name)
    .bind(excluded_user_id)
    .fetch_optional(&state.pool)
    .await?;

    if existing_user_id.is_some() {
        return Err(AppError::Conflict(
            "display name already exists".to_string(),
        ));
    }

    Ok(())
}

pub async fn ensure_another_active_owner(state: &AppState, user_id: Uuid) -> AppResult<()> {
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
