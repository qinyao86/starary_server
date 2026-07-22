use crate::{
    auth::AuthUser,
    error::{AppError, AppResult},
    ids::generate_id,
    models::PresetRecord,
    routes::access::ensure_library_access,
    state::AppState,
};
use axum::{
    extract::{Path, State},
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::collections::HashSet;

const MAX_QUICK_ACCESS_FOLDERS: usize = 500;
const MAX_FILTER_PRESETS: usize = 200;

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

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct StoredFilterPreset {
    id: String,
    name: String,
    value: Value,
    sort_order: i64,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateFilterPresetRequest {
    name: String,
    value: Value,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdateFilterPresetRequest {
    name: Option<String>,
    value: Option<Value>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReorderFilterPresetsRequest {
    preset_ids: Vec<String>,
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

pub async fn list_filter_presets(
    State(state): State<AppState>,
    user: AuthUser,
    Path(library_id): Path<String>,
) -> AppResult<Json<Vec<PresetRecord>>> {
    ensure_library_access(&state, &user, &library_id).await?;
    let preferences: Value = sqlx::query_scalar("SELECT preferences FROM users WHERE id = $1")
        .bind(user.id)
        .fetch_one(&state.pool)
        .await?;
    Ok(Json(filter_preset_responses(extract_filter_presets(
        &preferences,
        &library_id,
    ))))
}

pub async fn create_filter_preset(
    State(state): State<AppState>,
    user: AuthUser,
    Path(library_id): Path<String>,
    Json(request): Json<CreateFilterPresetRequest>,
) -> AppResult<Json<Vec<PresetRecord>>> {
    ensure_library_access(&state, &user, &library_id).await?;
    let name = normalize_filter_preset_name(&request.name)?;
    let mut tx = state.pool.begin().await?;
    let mut preferences = lock_user_preferences(&mut tx, user.id).await?;
    let mut presets = extract_filter_presets(&preferences, &library_id);
    if presets.len() >= MAX_FILTER_PRESETS {
        return Err(AppError::BadRequest("too many filter presets".to_string()));
    }
    let now = Utc::now();
    let sort_order = presets
        .iter()
        .map(|preset| preset.sort_order)
        .max()
        .unwrap_or(0)
        + 1000;
    presets.push(StoredFilterPreset {
        id: generate_id("preset_"),
        name,
        value: request.value,
        sort_order,
        created_at: now,
        updated_at: now,
    });
    set_filter_presets(&mut preferences, &library_id, &presets);
    save_user_preferences(&mut tx, user.id, &preferences).await?;
    tx.commit().await?;
    Ok(Json(filter_preset_responses(presets)))
}

pub async fn update_filter_preset(
    State(state): State<AppState>,
    user: AuthUser,
    Path((library_id, preset_id)): Path<(String, String)>,
    Json(request): Json<UpdateFilterPresetRequest>,
) -> AppResult<Json<Vec<PresetRecord>>> {
    ensure_library_access(&state, &user, &library_id).await?;
    let name = request
        .name
        .as_deref()
        .map(normalize_filter_preset_name)
        .transpose()?;
    let mut tx = state.pool.begin().await?;
    let mut preferences = lock_user_preferences(&mut tx, user.id).await?;
    let mut presets = extract_filter_presets(&preferences, &library_id);
    let preset = presets
        .iter_mut()
        .find(|preset| preset.id == preset_id)
        .ok_or_else(|| AppError::NotFound("filter preset not found".to_string()))?;
    if let Some(name) = name {
        preset.name = name;
    }
    if let Some(value) = request.value {
        preset.value = value;
    }
    preset.updated_at = Utc::now();
    set_filter_presets(&mut preferences, &library_id, &presets);
    save_user_preferences(&mut tx, user.id, &preferences).await?;
    tx.commit().await?;
    Ok(Json(filter_preset_responses(presets)))
}

pub async fn delete_filter_preset(
    State(state): State<AppState>,
    user: AuthUser,
    Path((library_id, preset_id)): Path<(String, String)>,
) -> AppResult<Json<Vec<PresetRecord>>> {
    ensure_library_access(&state, &user, &library_id).await?;
    let mut tx = state.pool.begin().await?;
    let mut preferences = lock_user_preferences(&mut tx, user.id).await?;
    let mut presets = extract_filter_presets(&preferences, &library_id);
    let previous_len = presets.len();
    presets.retain(|preset| preset.id != preset_id);
    if presets.len() == previous_len {
        return Err(AppError::NotFound("filter preset not found".to_string()));
    }
    set_filter_presets(&mut preferences, &library_id, &presets);
    save_user_preferences(&mut tx, user.id, &preferences).await?;
    tx.commit().await?;
    Ok(Json(filter_preset_responses(presets)))
}

pub async fn clear_filter_presets(
    State(state): State<AppState>,
    user: AuthUser,
    Path(library_id): Path<String>,
) -> AppResult<Json<Vec<PresetRecord>>> {
    ensure_library_access(&state, &user, &library_id).await?;
    let mut tx = state.pool.begin().await?;
    let mut preferences = lock_user_preferences(&mut tx, user.id).await?;
    set_filter_presets(&mut preferences, &library_id, &[]);
    save_user_preferences(&mut tx, user.id, &preferences).await?;
    tx.commit().await?;
    Ok(Json(Vec::new()))
}

pub async fn reorder_filter_presets(
    State(state): State<AppState>,
    user: AuthUser,
    Path(library_id): Path<String>,
    Json(request): Json<ReorderFilterPresetsRequest>,
) -> AppResult<Json<Vec<PresetRecord>>> {
    ensure_library_access(&state, &user, &library_id).await?;
    let preset_ids = normalize_unique_ids(request.preset_ids)?;
    let mut tx = state.pool.begin().await?;
    let mut preferences = lock_user_preferences(&mut tx, user.id).await?;
    let presets = extract_filter_presets(&preferences, &library_id);
    if preset_ids.len() != presets.len() {
        return Err(AppError::BadRequest(
            "filter preset order does not match the current list".to_string(),
        ));
    }
    let mut presets_by_id = presets
        .into_iter()
        .map(|preset| (preset.id.clone(), preset))
        .collect::<std::collections::HashMap<_, _>>();
    let mut reordered = Vec::with_capacity(preset_ids.len());
    for (index, preset_id) in preset_ids.iter().enumerate() {
        let mut preset = presets_by_id.remove(preset_id).ok_or_else(|| {
            AppError::BadRequest("filter preset order contains an unknown preset".to_string())
        })?;
        preset.sort_order = (index as i64 + 1) * 1000;
        preset.updated_at = Utc::now();
        reordered.push(preset);
    }
    set_filter_presets(&mut preferences, &library_id, &reordered);
    save_user_preferences(&mut tx, user.id, &preferences).await?;
    tx.commit().await?;
    Ok(Json(filter_preset_responses(reordered)))
}

async fn lock_user_preferences(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    user_id: uuid::Uuid,
) -> AppResult<Value> {
    Ok(
        sqlx::query_scalar("SELECT preferences FROM users WHERE id = $1 FOR UPDATE")
            .bind(user_id)
            .fetch_one(&mut **tx)
            .await?,
    )
}

async fn save_user_preferences(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    user_id: uuid::Uuid,
    preferences: &Value,
) -> AppResult<()> {
    sqlx::query("UPDATE users SET preferences = $2, updated_at = NOW() WHERE id = $1")
        .bind(user_id)
        .bind(preferences)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

fn normalize_filter_preset_name(name: &str) -> AppResult<String> {
    let name = name.trim();
    if name.is_empty() {
        return Err(AppError::BadRequest(
            "filter preset name is required".to_string(),
        ));
    }
    Ok(name.to_string())
}

fn normalize_unique_ids(ids: Vec<String>) -> AppResult<Vec<String>> {
    let mut seen = HashSet::new();
    let mut normalized = Vec::new();
    for id in ids {
        let id = id.trim();
        if id.is_empty() || !seen.insert(id.to_string()) {
            return Err(AppError::BadRequest(
                "filter preset order contains invalid ids".to_string(),
            ));
        }
        normalized.push(id.to_string());
    }
    Ok(normalized)
}

fn extract_filter_presets(preferences: &Value, library_id: &str) -> Vec<StoredFilterPreset> {
    let mut presets = preferences
        .get("libraries")
        .and_then(|libraries| libraries.get(library_id))
        .and_then(|library| library.get("filterPresets"))
        .and_then(Value::as_array)
        .map(|presets| {
            presets
                .iter()
                .filter_map(|preset| serde_json::from_value(preset.clone()).ok())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    presets.sort_by_key(|preset: &StoredFilterPreset| preset.sort_order);
    presets
}

fn filter_preset_responses(presets: Vec<StoredFilterPreset>) -> Vec<PresetRecord> {
    presets
        .into_iter()
        .map(|preset| PresetRecord {
            id: preset.id,
            r#type: "filter".to_string(),
            name: preset.name,
            value: preset.value,
            sort_order: preset.sort_order,
            created_at: preset.created_at,
            updated_at: preset.updated_at,
            asset_count: None,
        })
        .collect()
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
    library_preferences_mut(preferences, library_id)
        .insert("quickAccessFolderIds".to_string(), json!(folder_ids));
}

fn set_filter_presets(preferences: &mut Value, library_id: &str, presets: &[StoredFilterPreset]) {
    library_preferences_mut(preferences, library_id)
        .insert("filterPresets".to_string(), json!(presets));
}

fn library_preferences_mut<'a>(
    preferences: &'a mut Value,
    library_id: &str,
) -> &'a mut Map<String, Value> {
    if !preferences.is_object() {
        *preferences = json!({});
    }
    let root = preferences
        .as_object_mut()
        .expect("preferences must be an object");
    root.insert("version".to_string(), json!(2));
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
}

#[cfg(test)]
mod tests {
    use super::{
        extract_filter_presets, extract_quick_access_folder_ids, set_filter_presets,
        set_quick_access_folder_ids, StoredFilterPreset,
    };
    use chrono::Utc;
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

    #[test]
    fn stores_filter_presets_without_overwriting_other_library_preferences() {
        let mut preferences = json!({});
        set_quick_access_folder_ids(&mut preferences, "lib_a", &["folder_1".to_string()]);
        let now = Utc::now();
        set_filter_presets(
            &mut preferences,
            "lib_a",
            &[StoredFilterPreset {
                id: "preset_1".to_string(),
                name: "Images".to_string(),
                value: json!({ "kind": ["image"] }),
                sort_order: 1000,
                created_at: now,
                updated_at: now,
            }],
        );
        set_quick_access_folder_ids(&mut preferences, "lib_b", &["folder_2".to_string()]);

        assert_eq!(
            extract_quick_access_folder_ids(&preferences, "lib_a"),
            vec!["folder_1".to_string()]
        );
        assert_eq!(extract_filter_presets(&preferences, "lib_a").len(), 1);
        assert!(extract_filter_presets(&preferences, "lib_b").is_empty());
        assert_eq!(preferences.get("version"), Some(&json!(2)));
    }
}
