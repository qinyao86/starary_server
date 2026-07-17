use crate::{
    auth::AuthUser,
    error::{AppError, AppResult},
    routes::access::ensure_library_access,
    state::AppState,
};
use axum::{
    extract::{Path, State},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::collections::HashSet;

const MAX_QUICK_ACCESS_FOLDERS: usize = 500;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdateQuickAccessRequest {
    folder_ids: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryPreferencesResponse {
    quick_access_folder_ids: Vec<String>,
}

pub async fn get_library_preferences(
    State(state): State<AppState>,
    user: AuthUser,
    Path(library_id): Path<String>,
) -> AppResult<Json<LibraryPreferencesResponse>> {
    ensure_library_access(&state, &user, &library_id).await?;
    let preferences: Value = sqlx::query_scalar("SELECT preferences FROM users WHERE id = $1")
        .bind(user.id)
        .fetch_one(&state.pool)
        .await?;
    let folder_ids = extract_quick_access_folder_ids(&preferences, &library_id);
    Ok(Json(LibraryPreferencesResponse {
        quick_access_folder_ids: filter_existing_folder_ids(&state, &library_id, folder_ids)
            .await?,
    }))
}

pub async fn update_quick_access(
    State(state): State<AppState>,
    user: AuthUser,
    Path(library_id): Path<String>,
    Json(request): Json<UpdateQuickAccessRequest>,
) -> AppResult<Json<LibraryPreferencesResponse>> {
    ensure_library_access(&state, &user, &library_id).await?;
    let folder_ids = normalize_folder_ids(request.folder_ids)?;
    let folder_ids = filter_existing_folder_ids(&state, &library_id, folder_ids).await?;

    let mut tx = state.pool.begin().await?;
    let mut preferences: Value =
        sqlx::query_scalar("SELECT preferences FROM users WHERE id = $1 FOR UPDATE")
            .bind(user.id)
            .fetch_one(&mut *tx)
            .await?;
    set_quick_access_folder_ids(&mut preferences, &library_id, &folder_ids);
    sqlx::query("UPDATE users SET preferences = $2, updated_at = NOW() WHERE id = $1")
        .bind(user.id)
        .bind(preferences)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;

    Ok(Json(LibraryPreferencesResponse {
        quick_access_folder_ids: folder_ids,
    }))
}

fn normalize_folder_ids(folder_ids: Vec<String>) -> AppResult<Vec<String>> {
    let mut seen = HashSet::new();
    let mut normalized = Vec::new();
    for folder_id in folder_ids {
        let folder_id = folder_id.trim();
        if folder_id.is_empty() || !seen.insert(folder_id.to_string()) {
            continue;
        }
        normalized.push(folder_id.to_string());
        if normalized.len() > MAX_QUICK_ACCESS_FOLDERS {
            return Err(AppError::BadRequest(
                "too many quick access folders".to_string(),
            ));
        }
    }
    Ok(normalized)
}

async fn filter_existing_folder_ids(
    state: &AppState,
    library_id: &str,
    folder_ids: Vec<String>,
) -> AppResult<Vec<String>> {
    if folder_ids.is_empty() {
        return Ok(folder_ids);
    }
    let existing_ids: HashSet<String> =
        sqlx::query_scalar("SELECT id FROM folders WHERE library_id = $1 AND id = ANY($2)")
            .bind(library_id)
            .bind(&folder_ids)
            .fetch_all(&state.pool)
            .await?
            .into_iter()
            .collect();
    Ok(folder_ids
        .into_iter()
        .filter(|folder_id| existing_ids.contains(folder_id))
        .collect())
}

fn extract_quick_access_folder_ids(preferences: &Value, library_id: &str) -> Vec<String> {
    preferences
        .get("libraries")
        .and_then(|libraries| libraries.get(library_id))
        .and_then(|library| library.get("quickAccessFolderIds"))
        .and_then(Value::as_array)
        .map(|folder_ids| {
            folder_ids
                .iter()
                .filter_map(Value::as_str)
                .map(ToOwned::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

fn set_quick_access_folder_ids(preferences: &mut Value, library_id: &str, folder_ids: &[String]) {
    if !preferences.is_object() {
        *preferences = json!({});
    }
    let root = preferences
        .as_object_mut()
        .expect("preferences must be an object");
    root.insert("version".to_string(), json!(1));
    let libraries = root
        .entry("libraries".to_string())
        .or_insert_with(|| Value::Object(Map::new()));
    if !libraries.is_object() {
        *libraries = Value::Object(Map::new());
    }
    let libraries = libraries
        .as_object_mut()
        .expect("libraries must be an object");
    let library = libraries
        .entry(library_id.to_string())
        .or_insert_with(|| Value::Object(Map::new()));
    if !library.is_object() {
        *library = Value::Object(Map::new());
    }
    library
        .as_object_mut()
        .expect("library preference must be an object")
        .insert("quickAccessFolderIds".to_string(), json!(folder_ids));
}

#[cfg(test)]
mod tests {
    use super::{extract_quick_access_folder_ids, set_quick_access_folder_ids};
    use serde_json::json;

    #[test]
    fn stores_preferences_by_library() {
        let mut preferences = json!({});
        set_quick_access_folder_ids(
            &mut preferences,
            "lib_a",
            &["folder_2".to_string(), "folder_1".to_string()],
        );
        assert_eq!(
            extract_quick_access_folder_ids(&preferences, "lib_a"),
            vec!["folder_2".to_string(), "folder_1".to_string()]
        );
        assert!(extract_quick_access_folder_ids(&preferences, "lib_b").is_empty());
    }
}
