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
        SELECT id, library_id, actor_user_id, action, target_type, target_id, details, created_at
        FROM activity_log
        WHERE library_id = $1
        ORDER BY created_at DESC, id DESC
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

fn default_limit() -> i64 {
    100
}
