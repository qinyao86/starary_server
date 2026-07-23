use crate::{
    error::{AppError, AppResult},
    models::TagGroupRecord,
    state::AppState,
};

pub async fn query_tag_groups(
    state: &AppState,
    library_id: &str,
) -> AppResult<Vec<TagGroupRecord>> {
    Ok(sqlx::query_as::<_, TagGroupRecord>(
        r#"
        SELECT
            g.id,
            g.name,
            g.color,
            COUNT(DISTINCT t.id)::BIGINT AS tag_count,
            COUNT(DISTINCT CASE WHEN a.id IS NOT NULL THEN t.id END)::BIGINT AS used_tag_count,
            g.sort_order,
            g.created_by_user_id,
            g.updated_by_user_id,
            g.created_at,
            g.updated_at
        FROM tag_groups g
        LEFT JOIN tags t ON t.group_id = g.id
        LEFT JOIN asset_tags at ON at.tag_id = t.id
        LEFT JOIN assets a
            ON a.id = at.asset_id
           AND a.library_id = g.library_id
           AND a.deleted_at IS NULL
        WHERE g.library_id = $1
        GROUP BY g.id
        ORDER BY g.sort_order ASC, g.created_at ASC
        "#,
    )
    .bind(library_id)
    .fetch_all(&state.pool)
    .await?)
}

pub async fn next_group_sort_order(state: &AppState, library_id: &str) -> AppResult<i64> {
    Ok(sqlx::query_scalar::<_, i64>(
        "SELECT COALESCE(MAX(sort_order), 0) + 1000 FROM tag_groups WHERE library_id = $1",
    )
    .bind(library_id)
    .fetch_one(&state.pool)
    .await?)
}

pub async fn ensure_unique_group_name(
    state: &AppState,
    library_id: &str,
    name: &str,
    excluded_group_id: Option<&str>,
) -> AppResult<()> {
    let existing: Option<String> = sqlx::query_scalar(
        r#"
        SELECT id
        FROM tag_groups
        WHERE library_id = $1
          AND lower(name) = lower($2)
          AND ($3::text IS NULL OR id <> $3)
        LIMIT 1
        "#,
    )
    .bind(library_id)
    .bind(name)
    .bind(excluded_group_id)
    .fetch_optional(&state.pool)
    .await?;

    if existing.is_some() {
        return Err(AppError::Conflict(
            "tag group name already exists".to_string(),
        ));
    }

    Ok(())
}

pub async fn query_group_name(
    state: &AppState,
    library_id: &str,
    group_id: &str,
) -> AppResult<String> {
    sqlx::query_scalar("SELECT name FROM tag_groups WHERE library_id = $1 AND id = $2")
        .bind(library_id)
        .bind(group_id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or_else(|| AppError::NotFound("tag group not found".to_string()))
}

pub async fn query_group_tag_ids(
    state: &AppState,
    library_id: &str,
    group_id: &str,
) -> AppResult<Vec<String>> {
    Ok(
        sqlx::query_scalar("SELECT id FROM tags WHERE library_id = $1 AND group_id = $2")
            .bind(library_id)
            .bind(group_id)
            .fetch_all(&state.pool)
            .await?,
    )
}

pub async fn query_group_edit_state(
    state: &AppState,
    library_id: &str,
    group_id: &str,
) -> AppResult<TagGroupEditState> {
    sqlx::query_as::<_, TagGroupEditState>(
        "SELECT name, color FROM tag_groups WHERE library_id = $1 AND id = $2",
    )
    .bind(library_id)
    .bind(group_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::NotFound("tag group not found".to_string()))
}

#[derive(sqlx::FromRow)]
pub struct TagGroupEditState {
    pub name: String,
    pub color: String,
}
