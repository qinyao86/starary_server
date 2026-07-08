use crate::{
    auth::AuthUser,
    error::{AppError, AppResult},
    routes::{
        access::{ensure_library_access, ensure_library_write_access},
        library_structure::{
            common::{
                insert_activity, new_prefixed_id, normalize_required_name, normalize_required_text,
            },
            requests::{CreateTagGroupRequest, UpdateTagGroupRequest},
            tag_group_queries::{
                ensure_unique_group_name, next_group_sort_order, query_group_edit_state,
                query_group_name, query_tag_groups,
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
use uuid::Uuid;

pub async fn list_tag_groups(
    State(state): State<AppState>,
    user: AuthUser,
    Path(library_id): Path<Uuid>,
) -> AppResult<Json<Vec<crate::models::TagGroupRecord>>> {
    ensure_library_access(&state, &user, library_id).await?;
    Ok(Json(query_tag_groups(&state, library_id).await?))
}

pub async fn create_tag_group(
    State(state): State<AppState>,
    user: AuthUser,
    Path(library_id): Path<Uuid>,
    Json(request): Json<CreateTagGroupRequest>,
) -> AppResult<Json<Vec<crate::models::TagGroupRecord>>> {
    ensure_library_write_access(&state, &user, library_id).await?;

    let name = normalize_required_name(&request.name, "tag group name")?;
    let color = normalize_required_text(request.color, "default");
    ensure_unique_group_name(&state, library_id, &name, None).await?;

    let group_id = new_prefixed_id("tag_group_");
    let sort_order = next_group_sort_order(&state, library_id).await?;

    sqlx::query(
        r#"
        INSERT INTO tag_groups (
            id, library_id, name, color, sort_order, created_by_user_id, updated_by_user_id
        )
        VALUES ($1, $2, $3, $4, $5, $6, $6)
        "#,
    )
    .bind(&group_id)
    .bind(library_id)
    .bind(&name)
    .bind(&color)
    .bind(sort_order)
    .bind(user.id)
    .execute(&state.pool)
    .await?;

    insert_activity(
        &state,
        library_id,
        user.id,
        "tag_group.created",
        "tag_group",
        &group_id,
        &name,
    )
    .await?;

    Ok(Json(query_tag_groups(&state, library_id).await?))
}

pub async fn update_tag_group(
    State(state): State<AppState>,
    user: AuthUser,
    Path((library_id, group_id)): Path<(Uuid, String)>,
    Json(request): Json<UpdateTagGroupRequest>,
) -> AppResult<Json<Vec<crate::models::TagGroupRecord>>> {
    ensure_library_write_access(&state, &user, library_id).await?;

    let current = query_group_edit_state(&state, library_id, &group_id).await?;
    let name = request
        .name
        .as_deref()
        .map(|value| normalize_required_name(value, "tag group name"))
        .transpose()?
        .unwrap_or(current.name);
    let color = normalize_required_text(request.color, &current.color);

    ensure_unique_group_name(&state, library_id, &name, Some(&group_id)).await?;

    let mut tx = state.pool.begin().await?;
    sqlx::query(
        r#"
        UPDATE tag_groups
        SET name = $3,
            color = $4,
            updated_by_user_id = $5,
            updated_at = NOW()
        WHERE library_id = $1 AND id = $2
        "#,
    )
    .bind(library_id)
    .bind(&group_id)
    .bind(&name)
    .bind(&color)
    .bind(user.id)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        UPDATE tags
        SET color = $3,
            updated_by_user_id = $4,
            updated_at = NOW()
        WHERE library_id = $1 AND group_id = $2
        "#,
    )
    .bind(library_id)
    .bind(&group_id)
    .bind(&color)
    .bind(user.id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    insert_activity(
        &state,
        library_id,
        user.id,
        "tag_group.updated",
        "tag_group",
        &group_id,
        &name,
    )
    .await?;

    Ok(Json(query_tag_groups(&state, library_id).await?))
}

pub async fn delete_tag_group(
    State(state): State<AppState>,
    user: AuthUser,
    Path((library_id, group_id)): Path<(Uuid, String)>,
) -> AppResult<StatusCode> {
    ensure_library_write_access(&state, &user, library_id).await?;
    let name = query_group_name(&state, library_id, &group_id).await?;

    let mut tx = state.pool.begin().await?;
    sqlx::query(
        r#"
        UPDATE tags
        SET group_id = NULL,
            color = NULL,
            updated_by_user_id = $3,
            updated_at = NOW()
        WHERE library_id = $1 AND group_id = $2
        "#,
    )
    .bind(library_id)
    .bind(&group_id)
    .bind(user.id)
    .execute(&mut *tx)
    .await?;

    let deleted = sqlx::query("DELETE FROM tag_groups WHERE library_id = $1 AND id = $2")
        .bind(library_id)
        .bind(&group_id)
        .execute(&mut *tx)
        .await?;

    if deleted.rows_affected() == 0 {
        return Err(AppError::NotFound("tag group not found".to_string()));
    }

    tx.commit().await?;

    insert_activity(
        &state,
        library_id,
        user.id,
        "tag_group.deleted",
        "tag_group",
        &group_id,
        &name,
    )
    .await?;

    Ok(StatusCode::NO_CONTENT)
}
