use crate::{
    auth::AuthUser,
    error::{AppError, AppResult},
    routes::access::ensure_library_access,
    state::AppState,
};
use axum::{
    extract::{Path, State},
    http::StatusCode,
};
use uuid::Uuid;

use super::super::queries::{count_active_assets_for_root, get_storage_root_record};
pub async fn delete_storage_root(
    State(state): State<AppState>,
    user: AuthUser,
    Path(root_id): Path<Uuid>,
) -> AppResult<StatusCode> {
    let existing = get_storage_root_record(&state, root_id).await?;
    let library_role = ensure_library_access(&state, &user, &existing.library_id).await?;
    if !user.role.can_manage_server() && !library_role.can_manage_library() {
        return Err(AppError::Forbidden);
    }

    let active_assets = count_active_assets_for_root(&state, root_id).await?;

    if active_assets > 0 {
        return Err(AppError::Conflict(
            "workspace is still referenced by assets; disable it instead".to_string(),
        ));
    }

    let mut tx = state.pool.begin().await?;
    let deleted = sqlx::query("DELETE FROM storage_roots WHERE id = $1")
        .bind(root_id)
        .execute(&mut *tx)
        .await?;

    if deleted.rows_affected() == 0 {
        return Err(AppError::NotFound("workspace not found".to_string()));
    }

    sqlx::query(
        r#"
        INSERT INTO activity_log (id, library_id, actor_user_id, action, target_type, target_id, details)
        VALUES ($1, $2, $3, 'storage_root.deleted', 'storage_root', $4, $5::jsonb)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(&existing.library_id)
    .bind(user.id)
    .bind(root_id.to_string())
    .bind(serde_json::json!({ "name": existing.name }))
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(StatusCode::NO_CONTENT)
}
