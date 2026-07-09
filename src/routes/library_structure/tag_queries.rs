use crate::{
    error::{AppError, AppResult},
    models::TagRecord,
    state::AppState,
};

pub async fn query_tags(state: &AppState, library_id: &str) -> AppResult<Vec<TagRecord>> {
    Ok(sqlx::query_as::<_, TagRecord>(
        r#"
        SELECT
            t.id,
            t.group_id,
            t.name,
            t.color,
            t.starred,
            COUNT(a.id)::BIGINT AS asset_count,
            t.sort_order,
            t.created_by_user_id,
            t.updated_by_user_id,
            t.created_at,
            t.updated_at
        FROM tags t
        LEFT JOIN asset_tags at ON at.tag_id = t.id
        LEFT JOIN assets a
            ON a.id = at.asset_id
           AND a.library_id = t.library_id
           AND a.deleted_at IS NULL
        WHERE t.library_id = $1
        GROUP BY t.id
        ORDER BY t.sort_order ASC, t.created_at ASC
        "#,
    )
    .bind(library_id)
    .fetch_all(&state.pool)
    .await?)
}

pub async fn next_tag_sort_order(
    state: &AppState,
    library_id: &str,
    group_id: Option<&str>,
) -> AppResult<i64> {
    Ok(sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COALESCE(MAX(sort_order), 0) + 1000
        FROM tags
        WHERE library_id = $1
          AND (($2::text IS NULL AND group_id IS NULL) OR group_id = $2)
        "#,
    )
    .bind(library_id)
    .bind(group_id)
    .fetch_one(&state.pool)
    .await?)
}

pub async fn query_group_color(
    state: &AppState,
    library_id: &str,
    group_id: Option<&str>,
) -> AppResult<Option<String>> {
    let Some(group_id) = group_id else {
        return Ok(None);
    };

    sqlx::query_scalar("SELECT color FROM tag_groups WHERE library_id = $1 AND id = $2")
        .bind(library_id)
        .bind(group_id)
        .fetch_optional(&state.pool)
        .await?
        .map(Some)
        .ok_or_else(|| AppError::NotFound("tag group not found".to_string()))
}

pub async fn ensure_unique_tag_name(
    state: &AppState,
    library_id: &str,
    name: &str,
    excluded_tag_id: Option<&str>,
) -> AppResult<()> {
    let existing: Option<String> = sqlx::query_scalar(
        r#"
        SELECT id
        FROM tags
        WHERE library_id = $1
          AND lower(name) = lower($2)
          AND ($3::text IS NULL OR id <> $3)
        LIMIT 1
        "#,
    )
    .bind(library_id)
    .bind(name)
    .bind(excluded_tag_id)
    .fetch_optional(&state.pool)
    .await?;

    if existing.is_some() {
        return Err(AppError::Conflict("tag name already exists".to_string()));
    }

    Ok(())
}

pub async fn query_tag_name(state: &AppState, library_id: &str, tag_id: &str) -> AppResult<String> {
    sqlx::query_scalar("SELECT name FROM tags WHERE library_id = $1 AND id = $2")
        .bind(library_id)
        .bind(tag_id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or_else(|| AppError::NotFound("tag not found".to_string()))
}

pub async fn query_tag_edit_state(
    state: &AppState,
    library_id: &str,
    tag_id: &str,
) -> AppResult<TagEditState> {
    sqlx::query_as::<_, TagEditState>(
        "SELECT name, group_id, color, starred FROM tags WHERE library_id = $1 AND id = $2",
    )
    .bind(library_id)
    .bind(tag_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::NotFound("tag not found".to_string()))
}

pub async fn ensure_tag_exists_in_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    library_id: &str,
    tag_id: &str,
) -> AppResult<()> {
    let exists: Option<String> =
        sqlx::query_scalar("SELECT id FROM tags WHERE library_id = $1 AND id = $2")
            .bind(library_id)
            .bind(tag_id)
            .fetch_optional(&mut **tx)
            .await?;

    if exists.is_none() {
        return Err(AppError::NotFound("tag not found".to_string()));
    }

    Ok(())
}

#[derive(sqlx::FromRow)]
pub struct TagEditState {
    pub name: String,
    pub group_id: Option<String>,
    pub color: Option<String>,
    pub starred: bool,
}
