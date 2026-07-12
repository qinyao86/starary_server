use crate::{
    auth::AuthUser,
    error::{AppError, AppResult},
    models::Role,
    state::AppState,
};
use axum::{extract::State, http::StatusCode};

pub async fn initialize(State(state): State<AppState>, user: AuthUser) -> AppResult<StatusCode> {
    if user.role != Role::Owner {
        return Err(AppError::Forbidden);
    }

    state.backup_service.create_pre_initialize().await?;

    let mut tx = state.pool.begin().await?;
    sqlx::query("TRUNCATE TABLE users CASCADE")
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;

    Ok(StatusCode::NO_CONTENT)
}
