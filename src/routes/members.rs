use crate::{
    auth::AuthUser,
    error::{AppError, AppResult},
    models::LibraryMemberRecord,
    routes::access::ensure_library_manager,
    state::AppState,
};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use uuid::Uuid;

mod guards;
mod requests;

use guards::{current_library_role, ensure_another_library_manager};
use requests::UpsertMemberRequest;

pub async fn list_members(
    State(state): State<AppState>,
    user: AuthUser,
    Path(library_id): Path<Uuid>,
) -> AppResult<Json<Vec<LibraryMemberRecord>>> {
    ensure_library_manager(&state, &user, library_id).await?;

    let members = sqlx::query_as::<_, LibraryMemberRecord>(
        r#"
        SELECT
            m.library_id,
            m.user_id,
            u.email,
            u.display_name,
            m.role,
            m.created_at,
            m.updated_at
        FROM library_memberships m
        INNER JOIN users u ON u.id = m.user_id
        WHERE m.library_id = $1
        ORDER BY u.display_name ASC, u.email ASC
        "#,
    )
    .bind(library_id)
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(members))
}

pub async fn upsert_member(
    State(state): State<AppState>,
    actor: AuthUser,
    Path((library_id, user_id)): Path<(Uuid, Uuid)>,
    Json(request): Json<UpsertMemberRequest>,
) -> AppResult<Json<LibraryMemberRecord>> {
    ensure_library_manager(&state, &actor, library_id).await?;

    let target_is_active: Option<bool> =
        sqlx::query_scalar("SELECT is_active FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_optional(&state.pool)
            .await?;
    match target_is_active {
        Some(true) => {}
        Some(false) => return Err(AppError::BadRequest("user is disabled".to_string())),
        None => return Err(AppError::NotFound("user not found".to_string())),
    }

    if let Some(current_role) = current_library_role(&state, library_id, user_id).await? {
        if current_role.can_manage_library() && !request.role.can_manage_library() {
            ensure_another_library_manager(&state, library_id, user_id).await?;
        }
    }

    let mut tx = state.pool.begin().await?;

    sqlx::query(
        r#"
        INSERT INTO library_memberships (library_id, user_id, role)
        VALUES ($1, $2, $3)
        ON CONFLICT (library_id, user_id)
        DO UPDATE SET role = EXCLUDED.role, updated_at = NOW()
        "#,
    )
    .bind(library_id)
    .bind(user_id)
    .bind(request.role.as_str())
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO activity_log (id, library_id, actor_user_id, action, target_type, target_id, details)
        VALUES ($1, $2, $3, 'library.member_upserted', 'user', $4, $5::jsonb)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(library_id)
    .bind(actor.id)
    .bind(user_id)
    .bind(serde_json::json!({ "role": request.role.as_str() }))
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    let member = sqlx::query_as::<_, LibraryMemberRecord>(
        r#"
        SELECT
            m.library_id,
            m.user_id,
            u.email,
            u.display_name,
            m.role,
            m.created_at,
            m.updated_at
        FROM library_memberships m
        INNER JOIN users u ON u.id = m.user_id
        WHERE m.library_id = $1 AND m.user_id = $2
        "#,
    )
    .bind(library_id)
    .bind(user_id)
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(member))
}

pub async fn remove_member(
    State(state): State<AppState>,
    actor: AuthUser,
    Path((library_id, user_id)): Path<(Uuid, Uuid)>,
) -> AppResult<StatusCode> {
    ensure_library_manager(&state, &actor, library_id).await?;

    if let Some(current_role) = current_library_role(&state, library_id, user_id).await? {
        if current_role.can_manage_library() {
            ensure_another_library_manager(&state, library_id, user_id).await?;
        }
    }

    let mut tx = state.pool.begin().await?;

    let deleted = sqlx::query(
        r#"
        DELETE FROM library_memberships
        WHERE library_id = $1 AND user_id = $2
        "#,
    )
    .bind(library_id)
    .bind(user_id)
    .execute(&mut *tx)
    .await?;

    if deleted.rows_affected() == 0 {
        return Err(AppError::NotFound("library member not found".to_string()));
    }

    sqlx::query(
        r#"
        INSERT INTO activity_log (id, library_id, actor_user_id, action, target_type, target_id)
        VALUES ($1, $2, $3, 'library.member_removed', 'user', $4)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(library_id)
    .bind(actor.id)
    .bind(user_id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(StatusCode::NO_CONTENT)
}
