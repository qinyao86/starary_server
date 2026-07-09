use crate::{
    error::{AppError, AppResult},
    models::Role,
    state::AppState,
};
use uuid::Uuid;

pub async fn current_library_role(
    state: &AppState,
    library_id: &str,
    user_id: Uuid,
) -> AppResult<Option<Role>> {
    let role_value: Option<String> = sqlx::query_scalar(
        r#"
        SELECT role
        FROM library_memberships
        WHERE library_id = $1 AND user_id = $2
        "#,
    )
    .bind(library_id)
    .bind(user_id)
    .fetch_optional(&state.pool)
    .await?;

    role_value
        .map(|value| value.parse::<Role>().map_err(|_| AppError::Forbidden))
        .transpose()
}

pub async fn ensure_another_library_manager(
    state: &AppState,
    library_id: &str,
    user_id: Uuid,
) -> AppResult<()> {
    let count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM library_memberships
        WHERE library_id = $1
            AND user_id <> $2
            AND role IN ('owner', 'admin', 'library_manager')
        "#,
    )
    .bind(library_id)
    .bind(user_id)
    .fetch_one(&state.pool)
    .await?;

    if count == 0 {
        return Err(AppError::Conflict(
            "at least one library manager is required".to_string(),
        ));
    }

    Ok(())
}
