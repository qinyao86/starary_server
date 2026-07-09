use crate::{
    error::{AppError, AppResult},
    models::PresetRecord,
    state::AppState,
};

pub const FILTER_PRESET_TYPE: &str = "filter";
pub const SMART_FOLDER_PRESET_TYPE: &str = "smart_folder";
pub const SMART_IMPORT_PRESET_TYPE: &str = "smart_import";

pub fn normalize_preset_type(value: &str) -> AppResult<String> {
    let preset_type = value.trim();
    if preset_type == FILTER_PRESET_TYPE
        || preset_type == SMART_FOLDER_PRESET_TYPE
        || preset_type == SMART_IMPORT_PRESET_TYPE
    {
        return Ok(preset_type.to_string());
    }

    Err(AppError::BadRequest(
        "preset type is not supported".to_string(),
    ))
}

pub fn normalize_required_name(value: &str) -> AppResult<String> {
    let normalized = value.trim().to_string();
    if normalized.is_empty() {
        return Err(AppError::BadRequest("preset name is required".to_string()));
    }
    Ok(normalized)
}

pub async fn query_presets(
    state: &AppState,
    library_id: &str,
    preset_type: &str,
) -> AppResult<Vec<PresetRecord>> {
    let records = sqlx::query_as::<_, PresetRecord>(
        r#"
        SELECT
            id,
            "type",
            name,
            value_json AS value,
            sort_order,
            created_at,
            updated_at,
            CASE WHEN "type" = 'smart_folder' THEN item_count ELSE NULL END AS asset_count
        FROM presets
        WHERE library_id = $1
          AND "type" = $2
        ORDER BY sort_order ASC, created_at ASC
        "#,
    )
    .bind(library_id)
    .bind(preset_type)
    .fetch_all(&state.pool)
    .await?;

    Ok(records)
}

pub async fn query_preset_name(
    state: &AppState,
    library_id: &str,
    preset_type: &str,
    preset_id: &str,
) -> AppResult<String> {
    sqlx::query_scalar(
        r#"
        SELECT name
        FROM presets
        WHERE library_id = $1
          AND "type" = $2
          AND id = $3
        "#,
    )
    .bind(library_id)
    .bind(preset_type)
    .bind(preset_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::NotFound("preset not found".to_string()))
}

pub async fn query_preset_edit_state(
    state: &AppState,
    library_id: &str,
    preset_type: &str,
    preset_id: &str,
) -> AppResult<(String, serde_json::Value)> {
    sqlx::query_as(
        r#"
        SELECT name, value_json
        FROM presets
        WHERE library_id = $1
          AND "type" = $2
          AND id = $3
        "#,
    )
    .bind(library_id)
    .bind(preset_type)
    .bind(preset_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::NotFound("preset not found".to_string()))
}

pub async fn next_preset_sort_order(
    state: &AppState,
    library_id: &str,
    preset_type: &str,
) -> AppResult<i64> {
    let sort_order = sqlx::query_scalar(
        r#"
        SELECT COALESCE(MAX(sort_order), 0) + 1000
        FROM presets
        WHERE library_id = $1
          AND "type" = $2
        "#,
    )
    .bind(library_id)
    .bind(preset_type)
    .fetch_one(&state.pool)
    .await?;

    Ok(sort_order)
}

pub fn unique_ids(ids: &[String]) -> Vec<String> {
    let mut unique = Vec::new();
    for id in ids {
        let normalized = id.trim();
        if !normalized.is_empty() && !unique.iter().any(|existing| existing == normalized) {
            unique.push(normalized.to_string());
        }
    }
    unique
}
