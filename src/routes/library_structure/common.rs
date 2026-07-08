use crate::{
    error::{AppError, AppResult},
    state::AppState,
};
use serde_json::json;
use uuid::Uuid;

pub fn normalize_required_name(value: &str, label: &str) -> AppResult<String> {
    let normalized = value.trim().to_string();
    if normalized.is_empty() {
        return Err(AppError::BadRequest(format!("{label} is required")));
    }
    Ok(normalized)
}

pub fn normalize_optional_text(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

pub fn normalize_required_text(value: Option<String>, fallback: &str) -> String {
    normalize_optional_text(value).unwrap_or_else(|| fallback.to_string())
}

pub fn new_prefixed_id(prefix: &str) -> String {
    format!("{prefix}{}", Uuid::new_v4().simple())
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

pub async fn insert_activity(
    state: &AppState,
    library_id: Uuid,
    actor_user_id: Uuid,
    action: &str,
    target_type: &str,
    target_text_id: &str,
    target_name: &str,
) -> AppResult<()> {
    sqlx::query(
        r#"
        INSERT INTO activity_log (id, library_id, actor_user_id, action, target_type, details)
        VALUES ($1, $2, $3, $4, $5, $6)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(library_id)
    .bind(actor_user_id)
    .bind(action)
    .bind(target_type)
    .bind(json!({
        "targetId": target_text_id,
        "targetName": target_name,
    }))
    .execute(&state.pool)
    .await?;

    Ok(())
}

pub async fn ensure_asset_in_library(
    state: &AppState,
    library_id: Uuid,
    asset_id: Uuid,
) -> AppResult<()> {
    let exists: Option<Uuid> = sqlx::query_scalar(
        r#"
        SELECT id
        FROM assets
        WHERE id = $1 AND library_id = $2 AND deleted_at IS NULL
        "#,
    )
    .bind(asset_id)
    .bind(library_id)
    .fetch_optional(&state.pool)
    .await?;

    if exists.is_none() {
        return Err(AppError::NotFound("asset not found in library".to_string()));
    }

    Ok(())
}
