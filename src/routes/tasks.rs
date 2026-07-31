use crate::{
    auth::AuthUser,
    error::{AppError, AppResult},
    routes::access::ensure_library_access,
    state::AppState,
};
use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListTasksQuery {
    #[serde(default = "default_limit")]
    limit: i64,
    #[serde(default)]
    offset: i64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportTaskRequest {
    library_id: Option<String>,
    client_id: Option<String>,
    job_type: String,
    title: String,
    status: String,
    total: i64,
    processed: i64,
    failed: i64,
    progress: i32,
    message: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskListResponse {
    items: Vec<ServerTaskRecord>,
    limit: i64,
    offset: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportTaskResponse {
    task: ServerTaskRecord,
    delete_requested: bool,
}

#[derive(Clone, FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerTaskRecord {
    id: String,
    library_id: Option<String>,
    library_name: Option<String>,
    user_id: Option<Uuid>,
    user_display_name: Option<String>,
    user_email: Option<String>,
    user_avatar_key: Option<String>,
    client_id: String,
    job_type: String,
    title: String,
    status: String,
    total_count: i64,
    processed_count: i64,
    failed_count: i64,
    progress: i32,
    message: Option<String>,
    delete_requested_at: Option<DateTime<Utc>>,
    deleted_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    last_heartbeat_at: DateTime<Utc>,
}

pub async fn list_tasks(
    State(state): State<AppState>,
    user: AuthUser,
    Query(query): Query<ListTasksQuery>,
) -> AppResult<Json<TaskListResponse>> {
    ensure_server_task_admin(&user)?;
    let limit = query.limit.clamp(1, 500);
    let offset = query.offset.max(0);
    let items = sqlx::query_as::<_, ServerTaskRecord>(&task_select_sql(
        "t.deleted_at IS NULL",
        "ORDER BY t.updated_at DESC, t.created_at DESC LIMIT $1 OFFSET $2",
    ))
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(TaskListResponse {
        items,
        limit,
        offset,
    }))
}

pub async fn report_task(
    State(state): State<AppState>,
    user: AuthUser,
    Path(task_id): Path<String>,
    Json(payload): Json<ReportTaskRequest>,
) -> AppResult<Json<ReportTaskResponse>> {
    if task_id.trim().is_empty() {
        return Err(AppError::BadRequest("task id is required".to_string()));
    }
    if let Some(library_id) = payload.library_id.as_deref() {
        ensure_library_access(&state, &user, library_id).await?;
    }

    let progress = payload.progress.clamp(0, 100);
    let total = payload.total.max(0);
    let processed = payload.processed.clamp(0, total.max(payload.processed));
    let failed = payload.failed.max(0);
    let terminal = is_terminal_status(&payload.status);

    sqlx::query(
        r#"
        INSERT INTO server_tasks (
            id, library_id, user_id, client_id, job_type, title, status,
            total_count, processed_count, failed_count, progress, message,
            deleted_at, created_at, updated_at, last_heartbeat_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, NULL, NOW(), NOW(), NOW())
        ON CONFLICT (id) DO UPDATE
        SET
            library_id = EXCLUDED.library_id,
            user_id = EXCLUDED.user_id,
            client_id = EXCLUDED.client_id,
            job_type = EXCLUDED.job_type,
            title = EXCLUDED.title,
            status = EXCLUDED.status,
            total_count = EXCLUDED.total_count,
            processed_count = EXCLUDED.processed_count,
            failed_count = EXCLUDED.failed_count,
            progress = EXCLUDED.progress,
            message = EXCLUDED.message,
            deleted_at = CASE
                WHEN server_tasks.delete_requested_at IS NOT NULL AND $13 THEN NOW()
                ELSE server_tasks.deleted_at
            END,
            updated_at = NOW(),
            last_heartbeat_at = NOW()
        "#,
    )
    .bind(&task_id)
    .bind(payload.library_id.as_deref())
    .bind(user.id)
    .bind(payload.client_id.as_deref().unwrap_or(""))
    .bind(payload.job_type.trim())
    .bind(payload.title.trim())
    .bind(payload.status.trim())
    .bind(total)
    .bind(processed)
    .bind(failed)
    .bind(progress)
    .bind(payload.message.as_deref())
    .bind(terminal)
    .execute(&state.pool)
    .await?;

    let task = fetch_task(&state, &task_id).await?;
    Ok(Json(ReportTaskResponse {
        delete_requested: task.delete_requested_at.is_some() && task.deleted_at.is_none(),
        task,
    }))
}

pub async fn delete_task(
    State(state): State<AppState>,
    user: AuthUser,
    Path(task_id): Path<String>,
) -> AppResult<Json<ServerTaskRecord>> {
    ensure_server_task_admin(&user)?;
    let task = fetch_task(&state, &task_id).await?;
    let is_recent = Utc::now() - task.last_heartbeat_at < Duration::seconds(30);
    let is_active = matches!(
        task.status.as_str(),
        "running" | "paused" | "pausing" | "cancelling"
    );
    let updated = if is_recent && is_active {
        sqlx::query(
            r#"
            UPDATE server_tasks
            SET delete_requested_at = COALESCE(delete_requested_at, NOW()), updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(&task_id)
        .execute(&state.pool)
        .await?;
        fetch_task(&state, &task_id).await?
    } else {
        sqlx::query(
            r#"
            UPDATE server_tasks
            SET deleted_at = NOW(), delete_requested_at = COALESCE(delete_requested_at, NOW()), updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(&task_id)
        .execute(&state.pool)
        .await?;
        fetch_task(&state, &task_id).await?
    };
    Ok(Json(updated))
}

fn ensure_server_task_admin(user: &AuthUser) -> AppResult<()> {
    if user.role.can_manage_server() {
        Ok(())
    } else {
        Err(AppError::Forbidden)
    }
}

fn is_terminal_status(status: &str) -> bool {
    matches!(
        status,
        "completed" | "completed_with_errors" | "failed" | "cancelled"
    )
}

async fn fetch_task(state: &AppState, task_id: &str) -> AppResult<ServerTaskRecord> {
    sqlx::query_as::<_, ServerTaskRecord>(&task_select_sql("t.id = $1", "LIMIT 1"))
        .bind(task_id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or_else(|| AppError::NotFound("task not found".to_string()))
}

fn task_select_sql(where_clause: &str, tail: &str) -> String {
    format!(
        r#"
        SELECT
            t.id,
            t.library_id,
            l.display_name AS library_name,
            t.user_id,
            u.display_name AS user_display_name,
            u.email AS user_email,
            u.avatar_key AS user_avatar_key,
            t.client_id,
            t.job_type,
            t.title,
            t.status,
            t.total_count,
            t.processed_count,
            t.failed_count,
            t.progress,
            t.message,
            t.delete_requested_at,
            t.deleted_at,
            t.created_at,
            t.updated_at,
            t.last_heartbeat_at
        FROM server_tasks t
        LEFT JOIN libraries l ON l.id = t.library_id
        LEFT JOIN users u ON u.id = t.user_id
        WHERE {where_clause}
        {tail}
        "#
    )
}

fn default_limit() -> i64 {
    200
}
