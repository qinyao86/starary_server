mod queries;
mod requests;

use crate::{
    auth::AuthUser,
    error::{AppError, AppResult},
    models::PresetRecord,
    routes::{
        access::{ensure_library_access, ensure_library_write_access},
        presets::{
            queries::{
                next_preset_sort_order, normalize_preset_type, normalize_required_name,
                query_preset_edit_state, query_preset_name, query_presets, unique_ids,
                SMART_FOLDER_PRESET_TYPE,
            },
            requests::{
                CreatePresetRequest, ReorderPresetsRequest, UpdatePresetCountRequest,
                UpdatePresetRequest,
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
use serde_json::json;
use uuid::Uuid;

fn new_preset_id() -> String {
    format!("preset_{}", Uuid::new_v4().simple())
}

async fn insert_preset_activity(
    state: &AppState,
    library_id: Uuid,
    actor_user_id: Uuid,
    action: &str,
    preset_type: &str,
    preset_id: &str,
    preset_name: &str,
) -> AppResult<()> {
    sqlx::query(
        r#"
        INSERT INTO activity_log (id, library_id, actor_user_id, action, target_type, details)
        VALUES ($1, $2, $3, $4, 'preset', $5)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(library_id)
    .bind(actor_user_id)
    .bind(action)
    .bind(json!({
        "presetType": preset_type,
        "targetId": preset_id,
        "targetName": preset_name,
    }))
    .execute(&state.pool)
    .await?;

    Ok(())
}

pub async fn list_presets(
    State(state): State<AppState>,
    user: AuthUser,
    Path((library_id, preset_type)): Path<(Uuid, String)>,
) -> AppResult<Json<Vec<PresetRecord>>> {
    ensure_library_access(&state, &user, library_id).await?;
    let preset_type = normalize_preset_type(&preset_type)?;
    Ok(Json(query_presets(&state, library_id, &preset_type).await?))
}

pub async fn create_preset(
    State(state): State<AppState>,
    user: AuthUser,
    Path((library_id, preset_type)): Path<(Uuid, String)>,
    Json(request): Json<CreatePresetRequest>,
) -> AppResult<Json<Vec<PresetRecord>>> {
    ensure_library_write_access(&state, &user, library_id).await?;
    let preset_type = normalize_preset_type(&preset_type)?;
    let name = normalize_required_name(&request.name)?;
    let preset_id = new_preset_id();
    let sort_order = next_preset_sort_order(&state, library_id, &preset_type).await?;

    sqlx::query(
        r#"
        INSERT INTO presets (
            id, library_id, "type", name, value_json, sort_order,
            created_by_user_id, updated_by_user_id
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $7)
        "#,
    )
    .bind(&preset_id)
    .bind(library_id)
    .bind(&preset_type)
    .bind(&name)
    .bind(request.value)
    .bind(sort_order)
    .bind(user.id)
    .execute(&state.pool)
    .await?;

    insert_preset_activity(
        &state,
        library_id,
        user.id,
        "preset.created",
        &preset_type,
        &preset_id,
        &name,
    )
    .await?;

    Ok(Json(query_presets(&state, library_id, &preset_type).await?))
}

pub async fn update_preset(
    State(state): State<AppState>,
    user: AuthUser,
    Path((library_id, preset_type, preset_id)): Path<(Uuid, String, String)>,
    Json(request): Json<UpdatePresetRequest>,
) -> AppResult<Json<Vec<PresetRecord>>> {
    ensure_library_write_access(&state, &user, library_id).await?;
    let preset_type = normalize_preset_type(&preset_type)?;
    let (current_name, current_value) =
        query_preset_edit_state(&state, library_id, &preset_type, &preset_id).await?;
    let name = request
        .name
        .as_deref()
        .map(normalize_required_name)
        .transpose()?
        .unwrap_or(current_name);
    let value = request.value.unwrap_or(current_value);

    let updated = sqlx::query(
        r#"
        UPDATE presets
        SET name = $4,
            value_json = $5,
            updated_by_user_id = $6,
            updated_at = NOW()
        WHERE library_id = $1
          AND "type" = $2
          AND id = $3
        "#,
    )
    .bind(library_id)
    .bind(&preset_type)
    .bind(&preset_id)
    .bind(&name)
    .bind(value)
    .bind(user.id)
    .execute(&state.pool)
    .await?;

    if updated.rows_affected() == 0 {
        return Err(AppError::NotFound("preset not found".to_string()));
    }

    insert_preset_activity(
        &state,
        library_id,
        user.id,
        "preset.updated",
        &preset_type,
        &preset_id,
        &name,
    )
    .await?;

    Ok(Json(query_presets(&state, library_id, &preset_type).await?))
}

pub async fn delete_preset(
    State(state): State<AppState>,
    user: AuthUser,
    Path((library_id, preset_type, preset_id)): Path<(Uuid, String, String)>,
) -> AppResult<StatusCode> {
    ensure_library_write_access(&state, &user, library_id).await?;
    let preset_type = normalize_preset_type(&preset_type)?;
    let preset_name = query_preset_name(&state, library_id, &preset_type, &preset_id).await?;

    let deleted = sqlx::query(
        r#"
        DELETE FROM presets
        WHERE library_id = $1
          AND "type" = $2
          AND id = $3
        "#,
    )
    .bind(library_id)
    .bind(&preset_type)
    .bind(&preset_id)
    .execute(&state.pool)
    .await?;

    if deleted.rows_affected() == 0 {
        return Err(AppError::NotFound("preset not found".to_string()));
    }

    insert_preset_activity(
        &state,
        library_id,
        user.id,
        "preset.deleted",
        &preset_type,
        &preset_id,
        &preset_name,
    )
    .await?;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn clear_presets(
    State(state): State<AppState>,
    user: AuthUser,
    Path((library_id, preset_type)): Path<(Uuid, String)>,
) -> AppResult<Json<Vec<PresetRecord>>> {
    ensure_library_write_access(&state, &user, library_id).await?;
    let preset_type = normalize_preset_type(&preset_type)?;

    sqlx::query(
        r#"
        DELETE FROM presets
        WHERE library_id = $1
          AND "type" = $2
        "#,
    )
    .bind(library_id)
    .bind(&preset_type)
    .execute(&state.pool)
    .await?;

    insert_preset_activity(
        &state,
        library_id,
        user.id,
        "preset.cleared",
        &preset_type,
        "all",
        "all presets",
    )
    .await?;

    Ok(Json(Vec::new()))
}

pub async fn reorder_presets(
    State(state): State<AppState>,
    user: AuthUser,
    Path((library_id, preset_type)): Path<(Uuid, String)>,
    Json(request): Json<ReorderPresetsRequest>,
) -> AppResult<Json<Vec<PresetRecord>>> {
    ensure_library_write_access(&state, &user, library_id).await?;
    let preset_type = normalize_preset_type(&preset_type)?;
    let preset_ids = unique_ids(&request.preset_ids);
    if preset_ids.is_empty() {
        return Ok(Json(query_presets(&state, library_id, &preset_type).await?));
    }

    if preset_ids.len()
        != request
            .preset_ids
            .iter()
            .filter(|id| !id.trim().is_empty())
            .count()
    {
        return Err(AppError::BadRequest(
            "preset order contains duplicate ids".to_string(),
        ));
    }

    let mut tx = state.pool.begin().await?;
    let existing_count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM presets
        WHERE library_id = $1
          AND "type" = $2
        "#,
    )
    .bind(library_id)
    .bind(&preset_type)
    .fetch_one(&mut *tx)
    .await?;

    if existing_count != preset_ids.len() as i64 {
        return Err(AppError::BadRequest(
            "preset order does not match the current preset list".to_string(),
        ));
    }

    for (index, preset_id) in preset_ids.iter().enumerate() {
        let updated = sqlx::query(
            r#"
            UPDATE presets
            SET sort_order = $4,
                updated_by_user_id = $5,
                updated_at = NOW()
            WHERE library_id = $1
              AND "type" = $2
              AND id = $3
            "#,
        )
        .bind(library_id)
        .bind(&preset_type)
        .bind(preset_id)
        .bind((index as i64 + 1) * 1000)
        .bind(user.id)
        .execute(&mut *tx)
        .await?;

        if updated.rows_affected() != 1 {
            return Err(AppError::BadRequest(
                "preset order contains an unknown preset".to_string(),
            ));
        }
    }

    tx.commit().await?;

    insert_preset_activity(
        &state,
        library_id,
        user.id,
        "preset.reordered",
        &preset_type,
        "multiple",
        "multiple presets",
    )
    .await?;

    Ok(Json(query_presets(&state, library_id, &preset_type).await?))
}

pub async fn update_preset_count(
    State(state): State<AppState>,
    user: AuthUser,
    Path((library_id, preset_type, preset_id)): Path<(Uuid, String, String)>,
    Json(request): Json<UpdatePresetCountRequest>,
) -> AppResult<Json<PresetRecord>> {
    ensure_library_write_access(&state, &user, library_id).await?;
    let preset_type = normalize_preset_type(&preset_type)?;
    if preset_type != SMART_FOLDER_PRESET_TYPE {
        return Err(AppError::BadRequest(
            "preset count is only supported for smart folders".to_string(),
        ));
    }
    let asset_count = request.asset_count.max(0);

    let updated = sqlx::query(
        r#"
        UPDATE presets
        SET item_count = $4,
            updated_by_user_id = $5,
            updated_at = NOW()
        WHERE library_id = $1
          AND "type" = $2
          AND id = $3
        "#,
    )
    .bind(library_id)
    .bind(&preset_type)
    .bind(&preset_id)
    .bind(asset_count)
    .bind(user.id)
    .execute(&state.pool)
    .await?;

    if updated.rows_affected() == 0 {
        return Err(AppError::NotFound("preset not found".to_string()));
    }

    let record = query_presets(&state, library_id, &preset_type)
        .await?
        .into_iter()
        .find(|preset| preset.id == preset_id)
        .ok_or_else(|| AppError::NotFound("preset not found".to_string()))?;

    Ok(Json(record))
}
