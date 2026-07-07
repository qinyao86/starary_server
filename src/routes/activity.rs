use crate::{
    auth::AuthUser, error::AppResult, models::ActivityLogRecord,
    routes::access::ensure_library_access, state::AppState,
};
use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListActivityQuery {
    #[serde(default = "default_limit")]
    limit: i64,
    #[serde(default)]
    offset: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivityListResponse {
    items: Vec<ActivityLogRecord>,
    limit: i64,
    offset: i64,
}

pub async fn list_activity(
    State(state): State<AppState>,
    user: AuthUser,
    Path(library_id): Path<Uuid>,
    Query(query): Query<ListActivityQuery>,
) -> AppResult<Json<ActivityListResponse>> {
    ensure_library_access(&state, &user, library_id).await?;

    let limit = query.limit.clamp(1, 500);
    let offset = query.offset.max(0);

    let items = sqlx::query_as::<_, ActivityLogRecord>(
        r#"
        SELECT
            a.id,
            a.library_id,
            a.actor_user_id,
            actor.display_name AS actor_display_name,
            actor.email AS actor_email,
            a.action,
            a.target_type,
            a.target_id,
            COALESCE(
                target_user.display_name,
                target_library.display_name,
                target_root.name,
                a.details->>'name'
            ) AS target_name,
            a.details,
            a.created_at
        FROM activity_log a
        LEFT JOIN users actor ON actor.id = a.actor_user_id
        LEFT JOIN users target_user ON a.target_type = 'user' AND target_user.id = a.target_id
        LEFT JOIN libraries target_library ON a.target_type = 'library' AND target_library.id = a.target_id
        LEFT JOIN storage_roots target_root ON a.target_type = 'storage_root' AND target_root.id = a.target_id
        WHERE a.library_id = $1
        ORDER BY a.created_at DESC, a.id DESC
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(library_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(ActivityListResponse {
        items,
        limit,
        offset,
    }))
}

pub async fn list_server_activity(
    State(state): State<AppState>,
    user: AuthUser,
    Query(query): Query<ListActivityQuery>,
) -> AppResult<Json<ActivityListResponse>> {
    let limit = query.limit.clamp(1, 500);
    let offset = query.offset.max(0);

    let items = if user.role.can_manage_server() {
        let sql = activity_select_sql("TRUE");
        sqlx::query_as::<_, ActivityLogRecord>(&sql)
            .bind(limit)
            .bind(offset)
            .fetch_all(&state.pool)
            .await?
    } else {
        let sql = activity_select_sql(
            "a.library_id IN (
                SELECT library_id
                FROM library_memberships
                WHERE user_id = $3
            )",
        );
        sqlx::query_as::<_, ActivityLogRecord>(&sql)
            .bind(limit)
            .bind(offset)
            .bind(user.id)
            .fetch_all(&state.pool)
            .await?
    };

    Ok(Json(ActivityListResponse {
        items,
        limit,
        offset,
    }))
}

fn activity_select_sql(where_clause: &str) -> String {
    format!(
        r#"
        SELECT
            a.id,
            a.library_id,
            a.actor_user_id,
            actor.display_name AS actor_display_name,
            actor.email AS actor_email,
            a.action,
            a.target_type,
            a.target_id,
            COALESCE(
                target_user.display_name,
                target_library.display_name,
                target_root.name,
                a.details->>'name'
            ) AS target_name,
            a.details,
            a.created_at
        FROM activity_log a
        LEFT JOIN users actor ON actor.id = a.actor_user_id
        LEFT JOIN users target_user ON a.target_type = 'user' AND target_user.id = a.target_id
        LEFT JOIN libraries target_library ON a.target_type = 'library' AND target_library.id = a.target_id
        LEFT JOIN storage_roots target_root ON a.target_type = 'storage_root' AND target_root.id = a.target_id
        WHERE {where_clause}
        ORDER BY a.created_at DESC, a.id DESC
        LIMIT $1 OFFSET $2
        "#
    )
}

fn default_limit() -> i64 {
    100
}
