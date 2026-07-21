use super::{build_asset_responses, AssetListResponse};
use crate::{
    auth::AuthUser,
    error::{AppError, AppResult},
    models::AssetRecord,
    routes::access::ensure_library_access,
    state::AppState,
};
use axum::{
    extract::{Path, State},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{Postgres, QueryBuilder};
use uuid::Uuid;

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct WorkspaceAssetQueryRequest {
    scope: WorkspaceAssetQueryScope,
    smart_folder: Option<Value>,
    filters: WorkspaceAssetQueryFilters,
    search: WorkspaceAssetQuerySearch,
    sort: WorkspaceAssetQuerySort,
    known_total: Option<i64>,
    limit: Option<i64>,
    offset: Option<i64>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct WorkspaceAssetQueryScope {
    trashed: bool,
    folder_mode: String,
    folder_ids: Vec<String>,
    tag_id: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct WorkspaceAssetQueryDateRange {
    start: String,
    end: String,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct WorkspaceAssetQueryFilters {
    color_hex: String,
    kinds: Vec<String>,
    excluded_kinds: Vec<String>,
    tag_ids: Vec<String>,
    excluded_tag_ids: Vec<String>,
    tag_rule: String,
    folder_ids: Vec<String>,
    excluded_folder_ids: Vec<String>,
    folder_rule: String,
    extensions: Vec<String>,
    excluded_extensions: Vec<String>,
    import_modes: Vec<String>,
    ratings: Vec<String>,
    shapes: Vec<String>,
    custom_shape_width: String,
    custom_shape_height: String,
    imported_date: WorkspaceAssetQueryDateRange,
    created_date: WorkspaceAssetQueryDateRange,
    updated_date: WorkspaceAssetQueryDateRange,
    width_min: String,
    width_max: String,
    height_min: String,
    height_max: String,
    size_min: String,
    size_max: String,
    size_unit: String,
    duration_min: String,
    duration_max: String,
    duration_unit: String,
    note_mode: String,
    note_keyword: String,
    url_mode: String,
    url_keyword: String,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct WorkspaceAssetQuerySearch {
    keyword: String,
}

#[derive(Debug, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct WorkspaceAssetQuerySort {
    field: String,
    order: String,
    random: bool,
    random_seed: i64,
}

impl Default for WorkspaceAssetQuerySort {
    fn default() -> Self {
        Self {
            field: "updatedAt".to_string(),
            order: "desc".to_string(),
            random: false,
            random_seed: 0,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetIdListResponse {
    ids: Vec<String>,
}

pub async fn query_assets(
    State(state): State<AppState>,
    user: AuthUser,
    Path(library_id): Path<String>,
    Json(request): Json<WorkspaceAssetQueryRequest>,
) -> AppResult<Json<AssetListResponse>> {
    ensure_library_access(&state, &user, &library_id).await?;
    ensure_supported_query(&request)?;

    let limit = request.limit.unwrap_or(240).clamp(1, 500);
    let offset = request.offset.unwrap_or(0).max(0);
    let total = if let Some(known_total) = request.known_total.filter(|_| offset > 0) {
        known_total.max(0)
    } else {
        query_total(&state, &library_id, user.id, &request).await?
    };
    let records = query_page(&state, &library_id, user.id, &request, limit, offset).await?;
    let items = build_asset_responses(&state, &library_id, user.id, records).await?;

    Ok(Json(AssetListResponse {
        items,
        total,
        limit,
        offset,
    }))
}

pub async fn query_asset_ids(
    State(state): State<AppState>,
    user: AuthUser,
    Path(library_id): Path<String>,
    Json(request): Json<WorkspaceAssetQueryRequest>,
) -> AppResult<Json<AssetIdListResponse>> {
    ensure_library_access(&state, &user, &library_id).await?;
    ensure_supported_query(&request)?;

    let mut query = QueryBuilder::<Postgres>::new("SELECT a.id FROM assets a WHERE ");
    push_conditions(&mut query, &library_id, user.id, &request);
    push_order(&mut query, &request.sort);
    let ids = query
        .build_query_scalar::<String>()
        .fetch_all(&state.pool)
        .await?;
    Ok(Json(AssetIdListResponse { ids }))
}

fn ensure_supported_query(request: &WorkspaceAssetQueryRequest) -> AppResult<()> {
    if request.smart_folder.is_some() {
        return Err(AppError::BadRequest(
            "smart-folder queries require compatibility mode".to_string(),
        ));
    }
    if !request.filters.color_hex.trim().is_empty() {
        return Err(AppError::BadRequest(
            "color queries require compatibility mode".to_string(),
        ));
    }
    if !request.search.keyword.trim().is_empty() {
        return Err(AppError::BadRequest(
            "text queries require compatibility mode".to_string(),
        ));
    }
    if !request.filters.kinds.is_empty() || !request.filters.excluded_kinds.is_empty() {
        return Err(AppError::BadRequest(
            "semantic asset-kind queries require compatibility mode".to_string(),
        ));
    }
    if request.sort.random
        || ["name", "assetKind", "extension"].contains(&request.sort.field.as_str())
    {
        return Err(AppError::BadRequest(
            "locale-sensitive sorting requires compatibility mode".to_string(),
        ));
    }
    Ok(())
}

async fn query_total(
    state: &AppState,
    library_id: &str,
    user_id: Uuid,
    request: &WorkspaceAssetQueryRequest,
) -> AppResult<i64> {
    let mut query = QueryBuilder::<Postgres>::new("SELECT COUNT(*) FROM assets a WHERE ");
    push_conditions(&mut query, library_id, user_id, request);
    Ok(query
        .build_query_scalar::<i64>()
        .fetch_one(&state.pool)
        .await?)
}

async fn query_page(
    state: &AppState,
    library_id: &str,
    user_id: Uuid,
    request: &WorkspaceAssetQueryRequest,
    limit: i64,
    offset: i64,
) -> AppResult<Vec<AssetRecord>> {
    let mut query = QueryBuilder::<Postgres>::new(
        r#"SELECT
            a.id, a.library_id, a.name, a.asset_kind, a.import_mode, a.storage_key,
            a.storage_root_id, a.relative_path, a.metadata, a.created_by_user_id,
            a.imported_by_user_id, a.updated_by_user_id, a.deleted_by_user_id,
            a.restored_by_user_id, a.created_at, a.imported_at, a.updated_at,
            a.deleted_at, a.restored_at
        FROM assets a WHERE "#,
    );
    push_conditions(&mut query, library_id, user_id, request);
    push_order(&mut query, &request.sort);
    query
        .push(" LIMIT ")
        .push_bind(limit)
        .push(" OFFSET ")
        .push_bind(offset);
    Ok(query
        .build_query_as::<AssetRecord>()
        .fetch_all(&state.pool)
        .await?)
}

fn push_conditions(
    query: &mut QueryBuilder<'_, Postgres>,
    library_id: &str,
    user_id: Uuid,
    request: &WorkspaceAssetQueryRequest,
) {
    query
        .push("a.library_id = ")
        .push_bind(library_id.to_string());
    if request.scope.trashed {
        query.push(" AND a.deleted_at IS NOT NULL");
    } else {
        query.push(" AND a.deleted_at IS NULL");
    }

    match request.scope.folder_mode.as_str() {
        "untagged" => {
            query.push(" AND NOT EXISTS (SELECT 1 FROM asset_tags rel WHERE rel.asset_id = a.id)");
        }
        "uncategorized" => {
            query.push(
                " AND NOT EXISTS (SELECT 1 FROM asset_folders rel WHERE rel.asset_id = a.id)",
            );
        }
        "favorites" => {
            query.push(" AND EXISTS (SELECT 1 FROM asset_favorites fav WHERE fav.asset_id = a.id AND fav.library_id = a.library_id AND fav.user_id = ")
                .push_bind(user_id)
                .push(")");
        }
        "tag" => {
            if let Some(tag_id) = normalized_text(request.scope.tag_id.as_deref()) {
                query.push(" AND EXISTS (SELECT 1 FROM asset_tags rel WHERE rel.asset_id = a.id AND rel.tag_id = ")
                    .push_bind(tag_id)
                    .push(")");
            }
        }
        "folders" => {
            let folder_ids = normalized_entries(&request.scope.folder_ids);
            if !folder_ids.is_empty() {
                query.push(" AND EXISTS (SELECT 1 FROM asset_folders rel WHERE rel.asset_id = a.id AND rel.folder_id = ANY(")
                    .push_bind(folder_ids)
                    .push("))");
            }
        }
        _ => {}
    }

    push_kind_filter(query, &request.filters.kinds, false);
    push_kind_filter(query, &request.filters.excluded_kinds, true);
    push_text_list_filter(
        query,
        "LOWER(COALESCE(a.metadata->>'extension', ''))",
        &request.filters.extensions,
        false,
        true,
    );
    push_text_list_filter(
        query,
        "LOWER(COALESCE(a.metadata->>'extension', ''))",
        &request.filters.excluded_extensions,
        true,
        true,
    );
    push_text_list_filter(
        query,
        "LOWER(a.import_mode)",
        &request.filters.import_modes,
        false,
        false,
    );
    push_relation_filter(
        query,
        "asset_tags",
        "tag_id",
        &request.filters.tag_ids,
        &request.filters.tag_rule,
    );
    push_excluded_relation_filter(
        query,
        "asset_tags",
        "tag_id",
        &request.filters.excluded_tag_ids,
    );
    push_relation_filter(
        query,
        "asset_folders",
        "folder_id",
        &request.filters.folder_ids,
        &request.filters.folder_rule,
    );
    push_excluded_relation_filter(
        query,
        "asset_folders",
        "folder_id",
        &request.filters.excluded_folder_ids,
    );
    push_rating_filter(query, &request.filters.ratings);

    push_number_range(
        query,
        "NULLIF(a.metadata->>'width', '')::double precision",
        &request.filters.width_min,
        &request.filters.width_max,
        1.0,
    );
    push_number_range(
        query,
        "NULLIF(a.metadata->>'height', '')::double precision",
        &request.filters.height_min,
        &request.filters.height_max,
        1.0,
    );
    let size_multiplier = if request.filters.size_unit == "MB" {
        1024.0 * 1024.0
    } else {
        1024.0
    };
    push_number_range(
        query,
        "NULLIF(a.metadata->>'sizeBytes', '')::double precision",
        &request.filters.size_min,
        &request.filters.size_max,
        size_multiplier,
    );
    let duration_multiplier = match request.filters.duration_unit.as_str() {
        "minutes" => 60.0,
        "hours" => 3600.0,
        _ => 1.0,
    };
    push_number_range(
        query,
        "NULLIF(a.metadata->>'duration', '')::double precision",
        &request.filters.duration_min,
        &request.filters.duration_max,
        duration_multiplier,
    );
    push_date_range(
        query,
        "COALESCE(a.imported_at, a.created_at)",
        &request.filters.imported_date,
    );
    push_date_range(query, "a.created_at", &request.filters.created_date);
    push_date_range(query, "a.updated_at", &request.filters.updated_date);
    push_text_mode_filter(
        query,
        "COALESCE(a.metadata->>'description', '')",
        &request.filters.note_mode,
        &request.filters.note_keyword,
    );
    push_text_mode_filter(
        query,
        "COALESCE(a.metadata->>'url', '')",
        &request.filters.url_mode,
        &request.filters.url_keyword,
    );
    push_shape_filter(query, &request.filters);
}

fn push_kind_filter(query: &mut QueryBuilder<'_, Postgres>, values: &[String], excluded: bool) {
    let values = normalized_entries(values);
    if values.is_empty() {
        return;
    }
    let operator = if excluded { " != ALL(" } else { " = ANY(" };
    query
        .push(" AND LOWER(a.asset_kind)")
        .push(operator)
        .push_bind(values)
        .push(")");
}

fn push_text_list_filter(
    query: &mut QueryBuilder<'_, Postgres>,
    expression: &str,
    values: &[String],
    excluded: bool,
    trim_dot: bool,
) {
    let values = values
        .iter()
        .filter_map(|value| {
            let value = if trim_dot {
                value.trim().trim_start_matches('.')
            } else {
                value.trim()
            };
            (!value.is_empty()).then(|| value.to_lowercase())
        })
        .collect::<Vec<_>>();
    if values.is_empty() {
        return;
    }
    query.push(" AND ").push(expression);
    if excluded {
        query.push(" != ALL(");
    } else {
        query.push(" = ANY(");
    }
    query.push_bind(values).push(")");
}

fn push_relation_filter(
    query: &mut QueryBuilder<'_, Postgres>,
    table: &str,
    column: &str,
    values: &[String],
    rule: &str,
) {
    let values = normalized_entries(values);
    if values.is_empty() {
        return;
    }
    if rule == "Any" {
        query
            .push(" AND EXISTS (SELECT 1 FROM ")
            .push(table)
            .push(" rel WHERE rel.asset_id = a.id AND rel.")
            .push(column)
            .push(" = ANY(")
            .push_bind(values)
            .push("))");
        return;
    }
    let expected_count = values.len() as i64;
    query
        .push(" AND (SELECT COUNT(DISTINCT rel.")
        .push(column)
        .push(") FROM ")
        .push(table)
        .push(" rel WHERE rel.asset_id = a.id AND rel.")
        .push(column)
        .push(" = ANY(")
        .push_bind(values)
        .push(")) = ")
        .push_bind(expected_count);
    if rule == "Equals" {
        query
            .push(" AND (SELECT COUNT(*) FROM ")
            .push(table)
            .push(" rel WHERE rel.asset_id = a.id) = ")
            .push_bind(expected_count);
    }
}

fn push_excluded_relation_filter(
    query: &mut QueryBuilder<'_, Postgres>,
    table: &str,
    column: &str,
    values: &[String],
) {
    let values = normalized_entries(values);
    if values.is_empty() {
        return;
    }
    query
        .push(" AND NOT EXISTS (SELECT 1 FROM ")
        .push(table)
        .push(" rel WHERE rel.asset_id = a.id AND rel.")
        .push(column)
        .push(" = ANY(")
        .push_bind(values)
        .push("))");
}

fn push_rating_filter(query: &mut QueryBuilder<'_, Postgres>, ratings: &[String]) {
    let has_none = ratings.iter().any(|rating| rating == "none");
    let values = ratings
        .iter()
        .filter_map(|rating| rating.parse::<i32>().ok())
        .collect::<Vec<_>>();
    if !has_none && values.is_empty() {
        return;
    }
    query.push(" AND (");
    if has_none {
        query.push("NULLIF(a.metadata->>'rating', '') IS NULL");
        if !values.is_empty() {
            query.push(" OR ");
        }
    }
    if !values.is_empty() {
        query
            .push("NULLIF(a.metadata->>'rating', '')::integer = ANY(")
            .push_bind(values)
            .push(")");
    }
    query.push(")");
}

fn push_number_range(
    query: &mut QueryBuilder<'_, Postgres>,
    expression: &str,
    minimum: &str,
    maximum: &str,
    multiplier: f64,
) {
    if let Some(value) = parse_number(minimum, multiplier) {
        query
            .push(" AND ")
            .push(expression)
            .push(" >= ")
            .push_bind(value);
    }
    if let Some(value) = parse_number(maximum, multiplier) {
        query
            .push(" AND ")
            .push(expression)
            .push(" <= ")
            .push_bind(value);
    }
}

fn push_date_range(
    query: &mut QueryBuilder<'_, Postgres>,
    expression: &str,
    range: &WorkspaceAssetQueryDateRange,
) {
    if let Some(start) = normalized_text(Some(&range.start)) {
        query
            .push(" AND ")
            .push(expression)
            .push(" >= ")
            .push_bind(start)
            .push("::timestamptz");
    }
    if let Some(end) = normalized_text(Some(&range.end)) {
        query
            .push(" AND ")
            .push(expression)
            .push(" < ")
            .push_bind(end)
            .push("::timestamptz");
    }
}

fn push_text_mode_filter(
    query: &mut QueryBuilder<'_, Postgres>,
    expression: &str,
    mode: &str,
    keyword: &str,
) {
    match mode {
        "has" => {
            query.push(" AND TRIM(").push(expression).push(") != ''");
            if let Some(keyword) = normalized_text(Some(keyword)) {
                query
                    .push(" AND LOWER(")
                    .push(expression)
                    .push(") LIKE ")
                    .push_bind(format!("%{}%", keyword.to_lowercase()));
            }
        }
        "none" => {
            query.push(" AND TRIM(").push(expression).push(") = ''");
        }
        _ => {}
    }
}

fn push_shape_filter(query: &mut QueryBuilder<'_, Postgres>, filters: &WorkspaceAssetQueryFilters) {
    let width = "NULLIF(a.metadata->>'width', '')::double precision";
    let height = "NULLIF(a.metadata->>'height', '')::double precision";
    let mut conditions = Vec::new();
    for shape in &filters.shapes {
        match shape.as_str() {
            "horizontal" => conditions.push(format!("({width} > {height})")),
            "vertical" => conditions.push(format!("({height} > {width})")),
            "square" => conditions.push(format!("(ABS({width} - {height}) <= 2)")),
            "panoramicHorizontal" => conditions.push(format!("({width} >= {height} * 2)")),
            "panoramicVertical" => conditions.push(format!("({height} >= {width} * 2)")),
            "custom" => {
                if let (Some(custom_width), Some(custom_height)) = (
                    parse_number(&filters.custom_shape_width, 1.0),
                    parse_number(&filters.custom_shape_height, 1.0),
                ) {
                    if custom_width > 0.0 && custom_height > 0.0 {
                        let ratio = custom_width / custom_height;
                        conditions.push(format!(
                            "({width} / NULLIF({height}, 0) BETWEEN {} AND {})",
                            ratio * 0.96,
                            ratio * 1.04
                        ));
                    }
                }
            }
            _ => {}
        }
    }
    if !conditions.is_empty() {
        query.push(" AND (").push(conditions.join(" OR ")).push(")");
    }
}

fn push_order(query: &mut QueryBuilder<'_, Postgres>, sort: &WorkspaceAssetQuerySort) {
    query.push(" ORDER BY ");
    if sort.random {
        query
            .push("MD5(")
            .push_bind(format!("{}:", sort.random_seed))
            .push(" || a.id)");
    } else {
        let expression = match sort.field.as_str() {
            "createdAt" => "a.created_at",
            "importedAt" => "COALESCE(a.imported_at, a.created_at)",
            "name" => "LOWER(a.name)",
            "assetKind" => "LOWER(a.asset_kind)",
            "extension" => "LOWER(COALESCE(a.metadata->>'extension', ''))",
            "dimensions" => "COALESCE(NULLIF(a.metadata->>'width', '')::double precision, 0) * COALESCE(NULLIF(a.metadata->>'height', '')::double precision, 0)",
            "sizeBytes" => "COALESCE(NULLIF(a.metadata->>'sizeBytes', '')::double precision, 0)",
            "rating" => "COALESCE(NULLIF(a.metadata->>'rating', '')::double precision, 0)",
            _ => "a.updated_at",
        };
        query
            .push(expression)
            .push(if sort.order == "asc" { " ASC" } else { " DESC" });
    }
    query.push(", a.updated_at DESC, LOWER(a.name) ASC, a.id ASC");
}

fn normalized_entries(values: &[String]) -> Vec<String> {
    let mut normalized = Vec::new();
    for value in values {
        let value = value.trim().to_lowercase();
        if !value.is_empty() && value != "all" && !normalized.contains(&value) {
            normalized.push(value);
        }
    }
    normalized
}

fn normalized_text(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn parse_number(value: &str, multiplier: f64) -> Option<f64> {
    let value = value.trim().parse::<f64>().ok()? * multiplier;
    value.is_finite().then_some(value.max(0.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_queries_that_require_compatibility_semantics() {
        let mut request = WorkspaceAssetQueryRequest::default();
        request.search.keyword = "asset".to_string();
        assert!(matches!(
            ensure_supported_query(&request),
            Err(AppError::BadRequest(_))
        ));

        request.search.keyword.clear();
        request.filters.color_hex = "#ffffff".to_string();
        assert!(matches!(
            ensure_supported_query(&request),
            Err(AppError::BadRequest(_))
        ));
    }

    #[test]
    fn builds_folder_scoped_queries_with_stable_pagination_order() {
        let mut request = WorkspaceAssetQueryRequest::default();
        request.scope.folder_mode = "folders".to_string();
        request.scope.folder_ids = vec!["folder-a".to_string(), "folder-b".to_string()];
        request.sort.field = "updatedAt".to_string();
        request.sort.order = "desc".to_string();

        let mut query = QueryBuilder::<Postgres>::new("SELECT a.id FROM assets a WHERE ");
        push_conditions(&mut query, "library-a", Uuid::nil(), &request);
        push_order(&mut query, &request.sort);
        let sql = query.sql();

        assert!(sql.contains("asset_folders"));
        assert!(sql.contains("rel.folder_id = ANY("));
        assert!(sql.contains("ORDER BY a.updated_at DESC"));
        assert!(sql.ends_with("a.id ASC"));
    }
}
