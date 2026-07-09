use crate::{
    auth::AuthUser,
    error::{AppError, AppResult},
    routes::{
        access::{ensure_library_access, ensure_library_write_access},
        library_structure::{
            common::{
                insert_activity, new_prefixed_id, normalize_optional_text, normalize_required_name,
                unique_ids,
            },
            requests::{CreateTagRequest, MoveTagsRequest, UpdateTagRequest},
            tag_queries::{
                ensure_tag_exists_in_tx, ensure_unique_tag_name, next_tag_sort_order,
                query_group_color, query_tag_edit_state, query_tag_name, query_tags,
            },
        },
    },
    state::AppState,
};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};

pub async fn list_tags(
    State(state): State<AppState>,
    user: AuthUser,
    Path(library_id): Path<String>,
) -> AppResult<Json<Vec<crate::models::TagRecord>>> {
    ensure_library_access(&state, &user, &library_id).await?;
    Ok(Json(query_tags(&state, &library_id).await?))
}

pub async fn create_tag(
    State(state): State<AppState>,
    user: AuthUser,
    Path(library_id): Path<String>,
    Json(request): Json<CreateTagRequest>,
) -> AppResult<Json<Vec<crate::models::TagRecord>>> {
    ensure_library_write_access(&state, &user, &library_id).await?;

    let name = normalize_required_name(&request.name, "tag name")?;
    ensure_unique_tag_name(&state, &library_id, &name, None).await?;
    let group_color = query_group_color(&state, &library_id, request.group_id.as_deref()).await?;
    let color = normalize_optional_text(request.color).or(group_color);
    let sort_order = next_tag_sort_order(&state, &library_id, request.group_id.as_deref()).await?;
    let tag_id = new_prefixed_id("tag_");

    sqlx::query(
        r#"
        INSERT INTO tags (
            id, library_id, group_id, name, color, starred, sort_order,
            created_by_user_id, updated_by_user_id
        )
        VALUES ($1, $2, $3, $4, $5, FALSE, $6, $7, $7)
        "#,
    )
    .bind(&tag_id)
    .bind(&library_id)
    .bind(request.group_id)
    .bind(&name)
    .bind(color)
    .bind(sort_order)
    .bind(user.id)
    .execute(&state.pool)
    .await?;

    insert_activity(
        &state,
        &library_id,
        user.id,
        "tag.created",
        "tag",
        &tag_id,
        &name,
    )
    .await?;

    Ok(Json(query_tags(&state, &library_id).await?))
}

pub async fn update_tag(
    State(state): State<AppState>,
    user: AuthUser,
    Path((library_id, tag_id)): Path<(String, String)>,
    Json(request): Json<UpdateTagRequest>,
) -> AppResult<Json<Vec<crate::models::TagRecord>>> {
    ensure_library_write_access(&state, &user, &library_id).await?;

    let current = query_tag_edit_state(&state, &library_id, &tag_id).await?;
    let name = request
        .name
        .as_deref()
        .map(|value| normalize_required_name(value, "tag name"))
        .transpose()?
        .unwrap_or(current.name);
    ensure_unique_tag_name(&state, &library_id, &name, Some(&tag_id)).await?;

    let group_id = if request.clear_group_id.unwrap_or(false) {
        None
    } else {
        request.group_id.or(current.group_id)
    };
    let group_color = query_group_color(&state, &library_id, group_id.as_deref()).await?;
    let color = if request.clear_color.unwrap_or(false) {
        None
    } else {
        normalize_optional_text(request.color)
            .or(group_color)
            .or(current.color)
    };
    let starred = request.starred.unwrap_or(current.starred);

    sqlx::query(
        r#"
        UPDATE tags
        SET name = $3,
            group_id = $4,
            color = $5,
            starred = $6,
            updated_by_user_id = $7,
            updated_at = NOW()
        WHERE library_id = $1 AND id = $2
        "#,
    )
    .bind(&library_id)
    .bind(&tag_id)
    .bind(&name)
    .bind(group_id)
    .bind(color)
    .bind(starred)
    .bind(user.id)
    .execute(&state.pool)
    .await?;

    insert_activity(
        &state,
        &library_id,
        user.id,
        "tag.updated",
        "tag",
        &tag_id,
        &name,
    )
    .await?;

    Ok(Json(query_tags(&state, &library_id).await?))
}

pub async fn delete_tag(
    State(state): State<AppState>,
    user: AuthUser,
    Path((library_id, tag_id)): Path<(String, String)>,
) -> AppResult<StatusCode> {
    ensure_library_write_access(&state, &user, &library_id).await?;
    let tag_name = query_tag_name(&state, &library_id, &tag_id).await?;

    let deleted = sqlx::query("DELETE FROM tags WHERE library_id = $1 AND id = $2")
        .bind(&library_id)
        .bind(&tag_id)
        .execute(&state.pool)
        .await?;

    if deleted.rows_affected() == 0 {
        return Err(AppError::NotFound("tag not found".to_string()));
    }

    insert_activity(
        &state,
        &library_id,
        user.id,
        "tag.deleted",
        "tag",
        &tag_id,
        &tag_name,
    )
    .await?;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn move_tags(
    State(state): State<AppState>,
    user: AuthUser,
    Path(library_id): Path<String>,
    Json(request): Json<MoveTagsRequest>,
) -> AppResult<Json<Vec<crate::models::TagRecord>>> {
    ensure_library_write_access(&state, &user, &library_id).await?;

    let tag_ids = unique_ids(&request.tag_ids);
    if tag_ids.is_empty() {
        return Ok(Json(query_tags(&state, &library_id).await?));
    }

    let group_color = query_group_color(&state, &library_id, request.group_id.as_deref()).await?;
    let mut tx = state.pool.begin().await?;
    for tag_id in &tag_ids {
        ensure_tag_exists_in_tx(&mut tx, &library_id, tag_id).await?;
        sqlx::query(
            r#"
            UPDATE tags
            SET group_id = $3,
                color = $4,
                updated_by_user_id = $5,
                updated_at = NOW()
            WHERE library_id = $1 AND id = $2
            "#,
        )
        .bind(&library_id)
        .bind(tag_id)
        .bind(&request.group_id)
        .bind(&group_color)
        .bind(user.id)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;

    insert_activity(
        &state,
        &library_id,
        user.id,
        "tag.moved",
        "tag",
        &tag_ids.join(","),
        "multiple tags",
    )
    .await?;

    Ok(Json(query_tags(&state, &library_id).await?))
}
