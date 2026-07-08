use crate::{
    auth::AuthUser,
    error::{AppError, AppResult},
    routes::{
        access::{ensure_library_access, ensure_library_write_access},
        library_structure::{
            common::{
                ensure_asset_in_library, insert_activity, new_prefixed_id, normalize_optional_text,
                normalize_required_name, normalize_required_text, unique_ids,
            },
            folder_queries::{
                ensure_folder_can_move, ensure_folder_exists_in_tx, ensure_parent_folder,
                next_folder_sort_order, query_folder_edit_state, query_folder_name, query_folders,
                query_sibling_folder_ids_in_tx,
            },
            requests::{CreateFolderRequest, ReorderFoldersRequest, UpdateFolderRequest},
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

pub async fn list_folders(
    State(state): State<AppState>,
    user: AuthUser,
    Path(library_id): Path<Uuid>,
) -> AppResult<Json<Vec<crate::models::FolderRecord>>> {
    ensure_library_access(&state, &user, library_id).await?;
    Ok(Json(query_folders(&state, library_id).await?))
}

pub async fn create_folder(
    State(state): State<AppState>,
    user: AuthUser,
    Path(library_id): Path<Uuid>,
    Json(request): Json<CreateFolderRequest>,
) -> AppResult<Json<Vec<crate::models::FolderRecord>>> {
    ensure_library_write_access(&state, &user, library_id).await?;

    let name = normalize_required_name(&request.name, "folder name")?;
    let icon = normalize_required_text(request.icon, "folder");
    let color = normalize_required_text(request.color, "default");
    ensure_parent_folder(&state, library_id, request.parent_id.as_deref()).await?;

    let folder_id = new_prefixed_id("folder_");
    let sort_order =
        next_folder_sort_order(&state, library_id, request.parent_id.as_deref()).await?;

    sqlx::query(
        r#"
        INSERT INTO folders (
            id, library_id, parent_id, name, description, icon, color, sort_order,
            created_by_user_id, updated_by_user_id
        )
        VALUES ($1, $2, $3, $4, '', $5, $6, $7, $8, $8)
        "#,
    )
    .bind(&folder_id)
    .bind(library_id)
    .bind(request.parent_id)
    .bind(&name)
    .bind(icon)
    .bind(color)
    .bind(sort_order)
    .bind(user.id)
    .execute(&state.pool)
    .await?;

    insert_activity(
        &state,
        library_id,
        user.id,
        "folder.created",
        "folder",
        &folder_id,
        &name,
    )
    .await?;

    Ok(Json(query_folders(&state, library_id).await?))
}

pub async fn update_folder(
    State(state): State<AppState>,
    user: AuthUser,
    Path((library_id, folder_id)): Path<(Uuid, String)>,
    Json(request): Json<UpdateFolderRequest>,
) -> AppResult<Json<Vec<crate::models::FolderRecord>>> {
    ensure_library_write_access(&state, &user, library_id).await?;

    let current = query_folder_edit_state(&state, library_id, &folder_id).await?;
    let name = request
        .name
        .as_deref()
        .map(|value| normalize_required_name(value, "folder name"))
        .transpose()?
        .unwrap_or(current.name);
    let description = request
        .description
        .map(|value| value.trim().to_string())
        .unwrap_or(current.description);
    let icon = normalize_optional_text(request.icon).unwrap_or(current.icon);
    let color = normalize_optional_text(request.color).unwrap_or(current.color);
    let cover_asset_id = if request.clear_cover_asset_id.unwrap_or(false) {
        None
    } else {
        request.cover_asset_id.or(current.cover_asset_id)
    };
    let smart_import_id = if request.clear_smart_import_id.unwrap_or(false) {
        None
    } else {
        normalize_optional_text(request.smart_import_id).or(current.smart_import_id)
    };

    if let Some(asset_id) = cover_asset_id {
        ensure_asset_in_library(&state, library_id, asset_id).await?;
    }

    sqlx::query(
        r#"
        UPDATE folders
        SET name = $3,
            description = $4,
            icon = $5,
            color = $6,
            cover_asset_id = $7,
            smart_import_id = $8,
            updated_by_user_id = $9,
            updated_at = NOW()
        WHERE library_id = $1 AND id = $2
        "#,
    )
    .bind(library_id)
    .bind(&folder_id)
    .bind(&name)
    .bind(description)
    .bind(icon)
    .bind(color)
    .bind(cover_asset_id)
    .bind(smart_import_id)
    .bind(user.id)
    .execute(&state.pool)
    .await?;

    insert_activity(
        &state,
        library_id,
        user.id,
        "folder.updated",
        "folder",
        &folder_id,
        &name,
    )
    .await?;

    Ok(Json(query_folders(&state, library_id).await?))
}

pub async fn reorder_folders(
    State(state): State<AppState>,
    user: AuthUser,
    Path(library_id): Path<Uuid>,
    Json(request): Json<ReorderFoldersRequest>,
) -> AppResult<Json<Vec<crate::models::FolderRecord>>> {
    ensure_library_write_access(&state, &user, library_id).await?;
    ensure_parent_folder(&state, library_id, request.parent_id.as_deref()).await?;

    let requested_folder_ids = unique_ids(&request.folder_ids);
    if requested_folder_ids.is_empty() {
        return Ok(Json(query_folders(&state, library_id).await?));
    }

    let mut tx = state.pool.begin().await?;
    let existing_sibling_ids =
        query_sibling_folder_ids_in_tx(&mut tx, library_id, request.parent_id.as_deref()).await?;
    let mut ordered_folder_ids = requested_folder_ids;
    for sibling_id in existing_sibling_ids {
        if !ordered_folder_ids
            .iter()
            .any(|folder_id| folder_id == &sibling_id)
        {
            ordered_folder_ids.push(sibling_id);
        }
    }

    for (index, folder_id) in ordered_folder_ids.iter().enumerate() {
        ensure_folder_exists_in_tx(&mut tx, library_id, folder_id).await?;
        ensure_folder_can_move(&mut tx, library_id, folder_id, request.parent_id.as_deref())
            .await?;
        sqlx::query(
            r#"
            UPDATE folders
            SET parent_id = $3,
                sort_order = $4,
                updated_by_user_id = $5,
                updated_at = NOW()
            WHERE library_id = $1 AND id = $2
            "#,
        )
        .bind(library_id)
        .bind(folder_id)
        .bind(&request.parent_id)
        .bind((index as i64 + 1) * 1000)
        .bind(user.id)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;

    Ok(Json(query_folders(&state, library_id).await?))
}

pub async fn delete_folder(
    State(state): State<AppState>,
    user: AuthUser,
    Path((library_id, folder_id)): Path<(Uuid, String)>,
) -> AppResult<StatusCode> {
    ensure_library_write_access(&state, &user, library_id).await?;
    let folder_name = query_folder_name(&state, library_id, &folder_id).await?;

    let deleted = sqlx::query("DELETE FROM folders WHERE library_id = $1 AND id = $2")
        .bind(library_id)
        .bind(&folder_id)
        .execute(&state.pool)
        .await?;

    if deleted.rows_affected() == 0 {
        return Err(AppError::NotFound("folder not found".to_string()));
    }

    insert_activity(
        &state,
        library_id,
        user.id,
        "folder.deleted",
        "folder",
        &folder_id,
        &folder_name,
    )
    .await?;

    Ok(StatusCode::NO_CONTENT)
}
