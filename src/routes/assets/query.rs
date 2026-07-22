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
use sqlx::{Postgres, QueryBuilder};
use uuid::Uuid;

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct WorkspaceAssetQueryRequest {
    scope: WorkspaceAssetQueryScope,
    smart_folder: Option<WorkspaceAssetQuerySmartFolder>,
    filters: WorkspaceAssetQueryFilters,
    search: WorkspaceAssetQuerySearch,
    sort: WorkspaceAssetQuerySort,
    known_total: Option<i64>,
    limit: Option<i64>,
    offset: Option<i64>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct WorkspaceAssetQuerySmartFolder {
    id: String,
    rule_groups: Vec<WorkspaceAssetQuerySmartFolderRuleGroup>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct WorkspaceAssetQuerySmartFolderRuleGroup {
    id: String,
    match_mode: String,
    polarity: String,
    conditions: Vec<WorkspaceAssetQuerySmartFolderCondition>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct WorkspaceAssetQuerySmartFolderCondition {
    id: String,
    field: String,
    operator: String,
    value: String,
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
    color_threshold: f64,
    color_coverage: f64,
    color_mode: String,
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
    scopes: WorkspaceAssetQuerySearchScopes,
}

#[derive(Debug, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct WorkspaceAssetQuerySearchScopes {
    name: bool,
    tag: bool,
    folder_name: bool,
    folder_description: bool,
    #[serde(alias = "format")]
    r#type: bool,
    note: bool,
    url: bool,
}

impl Default for WorkspaceAssetQuerySearchScopes {
    fn default() -> Self {
        Self {
            name: true,
            tag: true,
            folder_name: true,
            folder_description: true,
            r#type: true,
            note: true,
            url: true,
        }
    }
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
    let mut query = QueryBuilder::<Postgres>::new("SELECT a.id FROM assets a WHERE ");
    push_conditions(&mut query, &library_id, user.id, &request)?;
    push_order(&mut query, &request);
    let ids = query
        .build_query_scalar::<String>()
        .fetch_all(&state.pool)
        .await?;
    Ok(Json(AssetIdListResponse { ids }))
}

async fn query_total(
    state: &AppState,
    library_id: &str,
    user_id: Uuid,
    request: &WorkspaceAssetQueryRequest,
) -> AppResult<i64> {
    let mut query = QueryBuilder::<Postgres>::new("SELECT COUNT(*) FROM assets a WHERE ");
    push_conditions(&mut query, library_id, user_id, request)?;
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
    push_conditions(&mut query, library_id, user_id, request)?;
    push_order(&mut query, request);
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
) -> AppResult<()> {
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

    push_smart_folder_conditions(query, request.smart_folder.as_ref())?;
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
    push_color_filter(query, &request.filters);
    push_search_filter(query, &request.search);
    Ok(())
}

fn push_kind_filter(query: &mut QueryBuilder<'_, Postgres>, values: &[String], excluded: bool) {
    let values = normalized_entries(values);
    if values.is_empty() {
        return;
    }
    let operator = if excluded { " != ALL(" } else { " = ANY(" };
    query
        .push(" AND ")
        .push(semantic_kind_expression())
        .push(operator)
        .push_bind(values)
        .push(")");
}

fn semantic_kind_expression() -> &'static str {
    r#"CASE
        WHEN LOWER(a.asset_kind) = 'image' AND LOWER(COALESCE(a.metadata->>'subtype', a.metadata->>'assetSubType', '')) = 'sequence' THEN 'imagesequence'
        WHEN LOWER(a.asset_kind) = 'package' AND LOWER(COALESCE(a.metadata->>'subtype', a.metadata->>'assetSubType', '')) = 'texture' THEN 'texture'
        WHEN LOWER(a.asset_kind) = 'package' AND LOWER(COALESCE(a.metadata->>'subtype', a.metadata->>'assetSubType', '')) = 'model' THEN 'modelpackage'
        ELSE LOWER(a.asset_kind)
    END"#
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

fn smart_folder_error(message: impl Into<String>) -> AppError {
    AppError::BadRequest(message.into())
}

fn smart_condition_is_candidate(condition: &WorkspaceAssetQuerySmartFolderCondition) -> bool {
    matches!(
        condition.operator.as_str(),
        "none" | "notEmpty" | "monochrome"
    ) || !condition.value.trim().is_empty()
}

fn push_smart_folder_conditions(
    query: &mut QueryBuilder<'_, Postgres>,
    smart_folder: Option<&WorkspaceAssetQuerySmartFolder>,
) -> AppResult<()> {
    let Some(smart_folder) = smart_folder else {
        return Ok(());
    };
    let groups = smart_folder
        .rule_groups
        .iter()
        .map(|group| {
            let conditions = group
                .conditions
                .iter()
                .filter(|condition| smart_condition_is_candidate(condition))
                .collect::<Vec<_>>();
            (group, conditions)
        })
        .filter(|(_, conditions)| !conditions.is_empty())
        .collect::<Vec<_>>();
    if groups.is_empty() {
        return Ok(());
    }

    query.push(" AND (");
    for (group_index, (group, conditions)) in groups.iter().enumerate() {
        if group_index > 0 {
            query.push(" AND ");
        }
        if group.polarity == "exclude" {
            query.push("NOT ");
        }
        query.push("(");
        let joiner = if group.match_mode == "any" {
            " OR "
        } else {
            " AND "
        };
        for (condition_index, condition) in conditions.iter().enumerate() {
            if condition_index > 0 {
                query.push(joiner);
            }
            push_smart_folder_condition(query, condition)?;
        }
        query.push(")");
    }
    query.push(")");
    Ok(())
}

fn push_smart_folder_condition(
    query: &mut QueryBuilder<'_, Postgres>,
    condition: &WorkspaceAssetQuerySmartFolderCondition,
) -> AppResult<()> {
    match condition.field.as_str() {
        "name" => push_smart_text_condition(query, condition, "a.name", "name"),
        "note" => push_smart_text_condition(
            query,
            condition,
            "COALESCE(a.metadata->>'description', '')",
            "note",
        ),
        "url" => {
            push_smart_text_condition(query, condition, "COALESCE(a.metadata->>'url', '')", "url")
        }
        "kind" => {
            push_smart_enum_condition(query, condition, semantic_kind_expression(), "kind", false)
        }
        "subtype" => push_smart_enum_condition(
            query,
            condition,
            "LOWER(COALESCE(a.metadata->>'subtype', a.metadata->>'assetSubType', ''))",
            "subtype",
            false,
        ),
        "importMode" => push_smart_enum_condition(
            query,
            condition,
            "LOWER(a.import_mode)",
            "import mode",
            false,
        ),
        "extension" => push_smart_enum_condition(
            query,
            condition,
            "LOWER(COALESCE(a.metadata->>'extension', ''))",
            "extension",
            true,
        ),
        "tag" => push_smart_relation_condition(query, condition, "asset_tags", "tag_id", "tag"),
        "folder" => {
            push_smart_relation_condition(query, condition, "asset_folders", "folder_id", "folder")
        }
        "width" => push_smart_number_condition(
            query,
            condition,
            "NULLIF(a.metadata->>'width', '')::double precision",
            true,
        ),
        "height" => push_smart_number_condition(
            query,
            condition,
            "NULLIF(a.metadata->>'height', '')::double precision",
            true,
        ),
        "size" => push_smart_number_condition(
            query,
            condition,
            "NULLIF(a.metadata->>'sizeBytes', '')::double precision",
            false,
        ),
        "duration" => push_smart_number_condition(
            query,
            condition,
            "NULLIF(a.metadata->>'duration', '')::double precision",
            false,
        ),
        "shape" => push_smart_shape_condition(query, condition),
        "rating" => push_smart_rating_condition(query, condition),
        "color" => push_smart_color_condition(query, condition),
        "createdDate" | "updatedDate" | "importedDate" => {
            push_smart_date_condition(query, condition)
        }
        field => Err(smart_folder_error(format!(
            "unsupported smart-folder field: {field}"
        ))),
    }
}

fn push_smart_text_condition(
    query: &mut QueryBuilder<'_, Postgres>,
    condition: &WorkspaceAssetQuerySmartFolderCondition,
    expression: &str,
    field_name: &str,
) -> AppResult<()> {
    match condition.operator.as_str() {
        "none" => {
            query.push("BTRIM(").push(expression).push(") = ''");
        }
        "notEmpty" => {
            query.push("BTRIM(").push(expression).push(") != ''");
        }
        "contains" | "notContains" | "beginsWith" | "endsWith" | "equals" => {
            let escaped = escape_like_pattern(condition.value.trim());
            let pattern = match condition.operator.as_str() {
                "contains" | "notContains" => format!("%{escaped}%"),
                "beginsWith" => format!("{escaped}%"),
                "endsWith" => format!("%{escaped}"),
                _ => escaped,
            };
            query.push(expression);
            if condition.operator == "notContains" {
                query.push(" NOT");
            }
            query
                .push(" ILIKE ")
                .push_bind(pattern)
                .push(" ESCAPE '\\'");
        }
        "regex" => {
            query
                .push(expression)
                .push(" ~ ")
                .push_bind(condition.value.trim().to_string());
        }
        operator => {
            return Err(smart_folder_error(format!(
                "unsupported smart-folder {field_name} operator: {operator}"
            )));
        }
    }
    Ok(())
}

fn push_smart_enum_condition(
    query: &mut QueryBuilder<'_, Postgres>,
    condition: &WorkspaceAssetQuerySmartFolderCondition,
    expression: &str,
    field_name: &str,
    trim_dot: bool,
) -> AppResult<()> {
    let value = if trim_dot {
        condition.value.trim().trim_start_matches('.')
    } else {
        condition.value.trim()
    }
    .to_lowercase();
    match condition.operator.as_str() {
        "equals" => query.push(expression).push(" = ").push_bind(value),
        "none" => query.push(expression).push(" != ").push_bind(value),
        operator => {
            return Err(smart_folder_error(format!(
                "unsupported smart-folder {field_name} operator: {operator}"
            )));
        }
    };
    Ok(())
}

fn smart_relation_ids(value: &str) -> Vec<String> {
    let mut ids = Vec::new();
    for id in value.split(',').map(str::trim).filter(|id| !id.is_empty()) {
        if !ids.iter().any(|current| current == id) {
            ids.push(id.to_string());
        }
    }
    ids
}

fn push_smart_relation_condition(
    query: &mut QueryBuilder<'_, Postgres>,
    condition: &WorkspaceAssetQuerySmartFolderCondition,
    table: &str,
    column: &str,
    field_name: &str,
) -> AppResult<()> {
    let ids = smart_relation_ids(&condition.value);
    match condition.operator.as_str() {
        "none" => {
            query
                .push("NOT EXISTS (SELECT 1 FROM ")
                .push(table)
                .push(" rel WHERE rel.asset_id = a.id)");
        }
        "notEmpty" => {
            query
                .push("EXISTS (SELECT 1 FROM ")
                .push(table)
                .push(" rel WHERE rel.asset_id = a.id)");
        }
        "contains" if !ids.is_empty() => {
            query
                .push("EXISTS (SELECT 1 FROM ")
                .push(table)
                .push(" rel WHERE rel.asset_id = a.id AND rel.")
                .push(column)
                .push(" = ANY(")
                .push_bind(ids)
                .push("))");
        }
        "equals" if !ids.is_empty() => {
            let expected = ids.len() as i64;
            query
                .push("(SELECT COUNT(DISTINCT rel.")
                .push(column)
                .push(") FROM ")
                .push(table)
                .push(" rel WHERE rel.asset_id = a.id AND rel.")
                .push(column)
                .push(" = ANY(")
                .push_bind(ids)
                .push(")) = ")
                .push_bind(expected);
        }
        "has" if !ids.is_empty() => {
            let expected = ids.len() as i64;
            query
                .push("((SELECT COUNT(DISTINCT rel.")
                .push(column)
                .push(") FROM ")
                .push(table)
                .push(" rel WHERE rel.asset_id = a.id AND rel.")
                .push(column)
                .push(" = ANY(")
                .push_bind(ids)
                .push(")) = ")
                .push_bind(expected)
                .push(" AND (SELECT COUNT(*) FROM ")
                .push(table)
                .push(" rel WHERE rel.asset_id = a.id) = ")
                .push_bind(expected)
                .push(")");
        }
        "notContains" if !ids.is_empty() => {
            query
                .push("NOT EXISTS (SELECT 1 FROM ")
                .push(table)
                .push(" rel WHERE rel.asset_id = a.id AND rel.")
                .push(column)
                .push(" = ANY(")
                .push_bind(ids)
                .push("))");
        }
        "contains" | "equals" | "has" | "notContains" => {
            query.push("FALSE");
        }
        operator => {
            return Err(smart_folder_error(format!(
                "unsupported smart-folder {field_name} operator: {operator}"
            )));
        }
    };
    Ok(())
}

fn parse_smart_number(value: &str, field: &str) -> (Option<f64>, Option<f64>) {
    let mut value_parts = value.splitn(2, '|');
    let range = value_parts.next().unwrap_or("").trim();
    let unit = value_parts.next().map(str::trim);
    let multiplier = match field {
        "size" if unit == Some("MB") => 1024.0 * 1024.0,
        "size" => 1024.0,
        "duration" if unit == Some("minutes") => 60.0,
        "duration" if unit == Some("hours") => 3600.0,
        _ => 1.0,
    };
    let mut range_parts = range.splitn(2, "..");
    let parse = |part: Option<&str>| {
        part.and_then(|part| part.trim().parse::<f64>().ok())
            .filter(|value| value.is_finite())
            .map(|value| (value.max(0.0) * multiplier).round())
    };
    (parse(range_parts.next()), parse(range_parts.next()))
}

fn push_smart_number_condition(
    query: &mut QueryBuilder<'_, Postgres>,
    condition: &WorkspaceAssetQuerySmartFolderCondition,
    expression: &str,
    require_positive: bool,
) -> AppResult<()> {
    let (start, end) = parse_smart_number(&condition.value, &condition.field);
    let Some(start) = start else {
        query.push("FALSE");
        return Ok(());
    };
    query.push("(").push(expression).push(" IS NOT NULL");
    if require_positive {
        query.push(" AND ").push(expression).push(" > 0");
    }
    match condition.operator.as_str() {
        "greaterThan" => query
            .push(" AND ")
            .push(expression)
            .push(" > ")
            .push_bind(start),
        "greaterThanOrEqual" => query
            .push(" AND ")
            .push(expression)
            .push(" >= ")
            .push_bind(start),
        "equals" => query
            .push(" AND ")
            .push(expression)
            .push(" = ")
            .push_bind(start),
        "lessThan" => query
            .push(" AND ")
            .push(expression)
            .push(" < ")
            .push_bind(start),
        "lessThanOrEqual" => query
            .push(" AND ")
            .push(expression)
            .push(" <= ")
            .push_bind(start),
        "between" => {
            let Some(end) = end else {
                query.push(" AND FALSE)");
                return Ok(());
            };
            query
                .push(" AND ")
                .push(expression)
                .push(" >= ")
                .push_bind(start)
                .push(" AND ")
                .push(expression)
                .push(" <= ")
                .push_bind(end)
        }
        operator => {
            return Err(smart_folder_error(format!(
                "unsupported smart-folder number operator: {operator}"
            )));
        }
    };
    query.push(")");
    Ok(())
}

fn smart_shape_expression(value: &str) -> Option<String> {
    let mut parts = value.splitn(2, '|');
    let shape = parts.next()?.trim();
    let width = "NULLIF(a.metadata->>'width', '')::double precision";
    let height = "NULLIF(a.metadata->>'height', '')::double precision";
    match shape {
        "horizontal" => Some(format!(
            "({width} > 0 AND {height} > 0 AND {width} > {height})"
        )),
        "vertical" => Some(format!(
            "({width} > 0 AND {height} > 0 AND {height} > {width})"
        )),
        "square" => Some(format!(
            "({width} > 0 AND {height} > 0 AND ABS({width} - {height}) <= 2)"
        )),
        "panoramicHorizontal" => Some(format!(
            "({width} > 0 AND {height} > 0 AND {width} >= {height} * 2)"
        )),
        "panoramicVertical" => Some(format!(
            "({width} > 0 AND {height} > 0 AND {height} >= {width} * 2)"
        )),
        "custom" => {
            let mut ratio = parts.next()?.splitn(2, ':');
            let custom_width = ratio.next()?.trim().parse::<f64>().ok()?;
            let custom_height = ratio.next()?.trim().parse::<f64>().ok()?;
            if custom_width <= 0.0 || custom_height <= 0.0 {
                return None;
            }
            let ratio = custom_width / custom_height;
            Some(format!("({width} > 0 AND {height} > 0 AND {width} / NULLIF({height}, 0) BETWEEN {} AND {})", ratio * 0.96, ratio * 1.04))
        }
        _ => None,
    }
}

fn push_smart_shape_condition(
    query: &mut QueryBuilder<'_, Postgres>,
    condition: &WorkspaceAssetQuerySmartFolderCondition,
) -> AppResult<()> {
    let Some(expression) = smart_shape_expression(&condition.value) else {
        query.push("FALSE");
        return Ok(());
    };
    match condition.operator.as_str() {
        "equals" => query.push(expression),
        "none" => query.push("NOT ").push(expression),
        operator => {
            return Err(smart_folder_error(format!(
                "unsupported smart-folder shape operator: {operator}"
            )));
        }
    };
    Ok(())
}

fn push_smart_rating_condition(
    query: &mut QueryBuilder<'_, Postgres>,
    condition: &WorkspaceAssetQuerySmartFolderCondition,
) -> AppResult<()> {
    let expression = "NULLIF(a.metadata->>'rating', '')::integer";
    match condition.operator.as_str() {
        "none" => {
            query.push(expression).push(" IS NULL");
        }
        "notEmpty" => {
            query.push(expression).push(" IS NOT NULL");
        }
        "equals" | "notContains" | "greaterThan" | "greaterThanOrEqual" | "lessThan"
        | "lessThanOrEqual" => {
            let Some(value) = condition
                .value
                .trim()
                .parse::<i32>()
                .ok()
                .filter(|value| (1..=5).contains(value))
            else {
                query.push("FALSE");
                return Ok(());
            };
            let operator = match condition.operator.as_str() {
                "equals" => "=",
                "notContains" => "!=",
                "greaterThan" => ">",
                "greaterThanOrEqual" => ">=",
                "lessThan" => "<",
                _ => "<=",
            };
            query
                .push(expression)
                .push(" ")
                .push(operator)
                .push(" ")
                .push_bind(value);
        }
        operator => {
            return Err(smart_folder_error(format!(
                "unsupported smart-folder rating operator: {operator}"
            )))
        }
    };
    Ok(())
}

fn push_smart_color_condition(
    query: &mut QueryBuilder<'_, Postgres>,
    condition: &WorkspaceAssetQuerySmartFolderCondition,
) -> AppResult<()> {
    if condition.operator == "monochrome" {
        let ratio = color_ratio_expression();
        let mono = monochrome_color_predicate();
        query.push(format!("COALESCE((SELECT SUM(CASE WHEN {mono} THEN {ratio} ELSE 0 END) FROM {}), 0) >= 0.97 AND COALESCE((SELECT SUM(CASE WHEN {mono} THEN {ratio} ELSE 0 END) / NULLIF(SUM({ratio}), 0) FROM {}), 0) >= 0.98", color_palette_rows(), color_palette_rows()));
        return Ok(());
    }
    let threshold = match condition.operator.as_str() {
        "similar" => 72.0,
        "almostEqual" => 90.0,
        operator => {
            return Err(smart_folder_error(format!(
                "unsupported smart-folder color operator: {operator}"
            )))
        }
    };
    let Some((predicate, _)) = color_distance_predicate(&condition.value, threshold) else {
        query.push("FALSE");
        return Ok(());
    };
    query
        .push("EXISTS (SELECT 1 FROM ")
        .push(color_palette_rows())
        .push(" WHERE ")
        .push(color_ratio_expression())
        .push(" >= 0.005 AND ")
        .push(predicate)
        .push(")");
    Ok(())
}

fn valid_date(value: &str) -> bool {
    let parts = value.split('-').collect::<Vec<_>>();
    if parts.len() != 3
        || parts[0].len() != 4
        || parts[1].len() != 2
        || parts[2].len() != 2
        || !parts
            .iter()
            .all(|part| part.chars().all(|character| character.is_ascii_digit()))
    {
        return false;
    }
    let Ok(year) = parts[0].parse::<u32>() else {
        return false;
    };
    let Ok(month) = parts[1].parse::<u32>() else {
        return false;
    };
    let Ok(day) = parts[2].parse::<u32>() else {
        return false;
    };
    let maximum_day = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if (year % 4 == 0 && year % 100 != 0) || year % 400 == 0 => 29,
        2 => 28,
        _ => return false,
    };
    year > 0 && day > 0 && day <= maximum_day
}

fn push_smart_date_condition(
    query: &mut QueryBuilder<'_, Postgres>,
    condition: &WorkspaceAssetQuerySmartFolderCondition,
) -> AppResult<()> {
    let column = match condition.field.as_str() {
        "createdDate" => "a.created_at",
        "updatedDate" => "a.updated_at",
        _ => "COALESCE(a.imported_at, a.created_at)",
    };
    let value = condition.value.trim();
    match condition.operator.as_str() {
        "preset" => {
            let (start, end) = match value {
                "today" => ("CURRENT_DATE", "CURRENT_DATE + INTERVAL '1 day'"),
                "yesterday" => ("CURRENT_DATE - INTERVAL '1 day'", "CURRENT_DATE"),
                "last5" => (
                    "CURRENT_DATE - INTERVAL '4 days'",
                    "CURRENT_DATE + INTERVAL '1 day'",
                ),
                "last7" => (
                    "CURRENT_DATE - INTERVAL '6 days'",
                    "CURRENT_DATE + INTERVAL '1 day'",
                ),
                "last30" => (
                    "CURRENT_DATE - INTERVAL '29 days'",
                    "CURRENT_DATE + INTERVAL '1 day'",
                ),
                "last90" => (
                    "CURRENT_DATE - INTERVAL '89 days'",
                    "CURRENT_DATE + INTERVAL '1 day'",
                ),
                "last365" => (
                    "CURRENT_DATE - INTERVAL '364 days'",
                    "CURRENT_DATE + INTERVAL '1 day'",
                ),
                _ => {
                    query.push("FALSE");
                    return Ok(());
                }
            };
            query
                .push("(")
                .push(column)
                .push(" >= ")
                .push(start)
                .push(" AND ")
                .push(column)
                .push(" < ")
                .push(end)
                .push(")");
        }
        "before" | "after" | "equals" if valid_date(value) => match condition.operator.as_str() {
            "before" => {
                query
                    .push(column)
                    .push(" < ")
                    .push_bind(value.to_string())
                    .push("::date");
            }
            "after" => {
                query
                    .push(column)
                    .push(" >= ")
                    .push_bind(value.to_string())
                    .push("::date + INTERVAL '1 day'");
            }
            _ => {
                query
                    .push("(")
                    .push(column)
                    .push(" >= ")
                    .push_bind(value.to_string())
                    .push("::date AND ")
                    .push(column)
                    .push(" < ")
                    .push_bind(value.to_string())
                    .push("::date + INTERVAL '1 day')");
            }
        },
        "between" => {
            let mut dates = value.splitn(2, "..");
            let start = dates.next().unwrap_or("").trim();
            let end = dates.next().unwrap_or("").trim();
            if !valid_date(start) || !valid_date(end) {
                query.push("FALSE");
                return Ok(());
            }
            query
                .push("(")
                .push(column)
                .push(" >= ")
                .push_bind(start.to_string())
                .push("::date AND ")
                .push(column)
                .push(" < ")
                .push_bind(end.to_string())
                .push("::date + INTERVAL '1 day')");
        }
        "before" | "after" | "equals" => {
            query.push("FALSE");
        }
        operator => {
            return Err(smart_folder_error(format!(
                "unsupported smart-folder date operator: {operator}"
            )))
        }
    }
    Ok(())
}

fn push_search_filter(query: &mut QueryBuilder<'_, Postgres>, search: &WorkspaceAssetQuerySearch) {
    let keyword = search.keyword.trim();
    if keyword.is_empty() {
        return;
    }

    let pattern = format!("%{}%", escape_like_pattern(keyword));
    let mut condition_count = 0usize;
    query.push(" AND (");
    let mut push_text_match = |query: &mut QueryBuilder<'_, Postgres>, expression: &str| {
        if condition_count > 0 {
            query.push(" OR ");
        }
        query
            .push(expression)
            .push(" ILIKE ")
            .push_bind(pattern.clone())
            .push(" ESCAPE '\\'");
        condition_count += 1;
    };

    if search.scopes.name {
        push_text_match(query, "a.name");
        push_text_match(query, "COALESCE(a.metadata->>'namePinyin', '')");
        push_text_match(query, "COALESCE(a.metadata->>'namePinyinInitials', '')");
    }
    if search.scopes.r#type {
        push_text_match(query, semantic_kind_expression());
        push_text_match(query, "a.asset_kind");
        push_text_match(
            query,
            "COALESCE(a.metadata->>'subtype', a.metadata->>'assetSubType', '')",
        );
        push_text_match(query, "COALESCE(a.metadata->>'extension', '')");
    }
    if search.scopes.note {
        push_text_match(query, "COALESCE(a.metadata->>'description', '')");
    }
    if search.scopes.url {
        push_text_match(query, "COALESCE(a.metadata->>'url', '')");
        push_text_match(query, "COALESCE(a.metadata->>'sourcePath', '')");
        push_text_match(query, "COALESCE(a.metadata->>'storedPath', '')");
        push_text_match(query, "COALESCE(a.relative_path, '')");
        push_text_match(query, "COALESCE(a.storage_key, '')");
    }
    if search.scopes.tag {
        if condition_count > 0 {
            query.push(" OR ");
        }
        query
            .push("EXISTS (SELECT 1 FROM asset_tags rel JOIN tags t ON t.id = rel.tag_id WHERE rel.asset_id = a.id AND t.name ILIKE ")
            .push_bind(pattern.clone())
            .push(" ESCAPE '\\')");
        condition_count += 1;
    }
    if search.scopes.folder_name {
        if condition_count > 0 {
            query.push(" OR ");
        }
        query
            .push("EXISTS (SELECT 1 FROM asset_folders rel JOIN folders f ON f.id = rel.folder_id WHERE rel.asset_id = a.id AND f.name ILIKE ")
            .push_bind(pattern.clone())
            .push(" ESCAPE '\\')");
        condition_count += 1;
    }
    if search.scopes.folder_description {
        if condition_count > 0 {
            query.push(" OR ");
        }
        query
            .push("EXISTS (SELECT 1 FROM asset_folders rel JOIN folders f ON f.id = rel.folder_id WHERE rel.asset_id = a.id AND COALESCE(f.description, '') ILIKE ")
            .push_bind(pattern)
            .push(" ESCAPE '\\')");
        condition_count += 1;
    }
    if condition_count == 0 {
        query.push("FALSE");
    }
    query.push(")");
}

fn escape_like_pattern(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

fn normalize_hex_color(value: &str) -> Option<[u8; 3]> {
    let value = value.trim().trim_start_matches('#');
    if value.len() != 6 {
        return None;
    }
    Some([
        u8::from_str_radix(&value[0..2], 16).ok()?,
        u8::from_str_radix(&value[2..4], 16).ok()?,
        u8::from_str_radix(&value[4..6], 16).ok()?,
    ])
}

fn color_palette_rows() -> &'static str {
    "jsonb_array_elements(CASE WHEN jsonb_typeof(a.metadata->'colorPalette') = 'array' THEN a.metadata->'colorPalette' ELSE '[]'::jsonb END) AS color(value)"
}

fn color_ratio_expression() -> &'static str {
    "COALESCE(NULLIF(color.value->>'ratio', '')::double precision, 0)"
}

fn color_rgb_expression(index: usize) -> String {
    format!("COALESCE(NULLIF(color.value->'rgb'->>{index}, '')::double precision, 0)")
}

fn monochrome_color_predicate() -> String {
    let red = color_rgb_expression(0);
    let green = color_rgb_expression(1);
    let blue = color_rgb_expression(2);
    let maximum = format!("GREATEST({red}, {green}, {blue})");
    let minimum = format!("LEAST({red}, {green}, {blue})");
    format!(
        "(({maximum} - {minimum}) <= 5 AND ({maximum} <= 0 OR ({maximum} - {minimum}) / NULLIF({maximum}, 0) <= 0.025))"
    )
}

fn color_distance_predicate(color_hex: &str, threshold: f64) -> Option<(String, f64)> {
    let [red, green, blue] = normalize_hex_color(color_hex)?;
    let max_distance =
        441.67295593_f64 * (1.0 - threshold.clamp(0.0, 100.0) / 100.0).clamp(0.05, 1.0);
    let max_distance_sq = (max_distance * max_distance).max(1.0);
    let red_value = color_rgb_expression(0);
    let green_value = color_rgb_expression(1);
    let blue_value = color_rgb_expression(2);
    let distance = format!(
        "((({red_value}) - {red}) * (({red_value}) - {red}) + (({green_value}) - {green}) * (({green_value}) - {green}) + (({blue_value}) - {blue}) * (({blue_value}) - {blue}))"
    );
    Some((format!("{distance} <= {max_distance_sq}"), max_distance_sq))
}

fn push_color_filter(query: &mut QueryBuilder<'_, Postgres>, filters: &WorkspaceAssetQueryFilters) {
    if filters.color_mode == "mono" {
        let ratio = color_ratio_expression();
        let mono = monochrome_color_predicate();
        query.push(format!(
            " AND COALESCE((SELECT SUM(CASE WHEN {mono} THEN {ratio} ELSE 0 END) FROM {}), 0) >= 0.97 AND COALESCE((SELECT SUM(CASE WHEN {mono} THEN {ratio} ELSE 0 END) / NULLIF(SUM({ratio}), 0) FROM {}), 0) >= 0.98",
            color_palette_rows(),
            color_palette_rows(),
        ));
        return;
    }

    let Some((predicate, _)) =
        color_distance_predicate(&filters.color_hex, filters.color_threshold)
    else {
        return;
    };
    let minimum_coverage = (filters.color_coverage.clamp(0.0, 100.0) / 100.0).max(0.0);
    query.push(format!(
        " AND COALESCE((SELECT SUM({}) FROM {} WHERE {} >= 0.005 AND {predicate}), 0) >= {minimum_coverage}",
        color_ratio_expression(),
        color_palette_rows(),
        color_ratio_expression(),
    ));
}

fn color_score_expression(filters: &WorkspaceAssetQueryFilters) -> Option<String> {
    if filters.color_mode == "mono" {
        let ratio = color_ratio_expression();
        let mono = monochrome_color_predicate();
        return Some(format!(
            "COALESCE((SELECT SUM(CASE WHEN {mono} THEN {ratio} ELSE 0 END) FROM {}), 0)",
            color_palette_rows(),
        ));
    }
    let (predicate, max_distance_sq) =
        color_distance_predicate(&filters.color_hex, filters.color_threshold)?;
    let [red, green, blue] = normalize_hex_color(&filters.color_hex)?;
    let red_value = color_rgb_expression(0);
    let green_value = color_rgb_expression(1);
    let blue_value = color_rgb_expression(2);
    let distance = format!(
        "((({red_value}) - {red}) * (({red_value}) - {red}) + (({green_value}) - {green}) * (({green_value}) - {green}) + (({blue_value}) - {blue}) * (({blue_value}) - {blue}))"
    );
    Some(format!(
        "COALESCE((SELECT SUM({ratio} * (0.35 + 0.65 * (1.0 - ({distance} / {max_distance_sq})))) FROM {rows} WHERE {ratio} >= 0.005 AND {predicate}), 0)",
        ratio = color_ratio_expression(),
        rows = color_palette_rows(),
    ))
}

fn push_order(query: &mut QueryBuilder<'_, Postgres>, request: &WorkspaceAssetQueryRequest) {
    query.push(" ORDER BY ");
    if let Some(color_score) = color_score_expression(&request.filters) {
        query.push(color_score).push(" DESC, ");
    }
    let sort = &request.sort;
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
            "assetKind" => semantic_kind_expression(),
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
    fn builds_advanced_queries_without_compatibility_fallback() {
        let mut request = WorkspaceAssetQueryRequest::default();
        request.search.keyword = "asset".to_string();
        request.filters.color_hex = "#336699".to_string();
        request.filters.color_threshold = 72.0;
        request.filters.color_coverage = 10.0;
        request.filters.color_mode = "similar".to_string();
        request.filters.kinds = vec!["imageSequence".to_string()];
        request.smart_folder = Some(WorkspaceAssetQuerySmartFolder {
            id: "smart-a".to_string(),
            rule_groups: vec![WorkspaceAssetQuerySmartFolderRuleGroup {
                id: "group-a".to_string(),
                match_mode: "all".to_string(),
                polarity: "include".to_string(),
                conditions: vec![WorkspaceAssetQuerySmartFolderCondition {
                    id: "condition-a".to_string(),
                    field: "name".to_string(),
                    operator: "contains".to_string(),
                    value: "preview".to_string(),
                }],
            }],
        });

        let mut query = QueryBuilder::<Postgres>::new("SELECT a.id FROM assets a WHERE ");
        push_conditions(&mut query, "library-a", Uuid::nil(), &request).expect("conditions");
        push_order(&mut query, &request);
        let sql = query.sql();

        assert!(sql.contains("colorPalette"));
        assert!(sql.contains("ILIKE"));
        assert!(sql.contains("imagesequence"));
        assert!(sql.contains("a.name ILIKE"));
        assert!(sql.contains("ORDER BY COALESCE((SELECT SUM"));
    }

    #[test]
    fn rejects_unknown_smart_folder_operators() {
        let mut request = WorkspaceAssetQueryRequest::default();
        request.smart_folder = Some(WorkspaceAssetQuerySmartFolder {
            id: "smart-a".to_string(),
            rule_groups: vec![WorkspaceAssetQuerySmartFolderRuleGroup {
                id: "group-a".to_string(),
                match_mode: "all".to_string(),
                polarity: "include".to_string(),
                conditions: vec![WorkspaceAssetQuerySmartFolderCondition {
                    id: "condition-a".to_string(),
                    field: "name".to_string(),
                    operator: "unsupported".to_string(),
                    value: "preview".to_string(),
                }],
            }],
        });

        let mut query = QueryBuilder::<Postgres>::new("SELECT a.id FROM assets a WHERE ");
        assert!(matches!(
            push_conditions(&mut query, "library-a", Uuid::nil(), &request),
            Err(AppError::BadRequest(_))
        ));
    }

    #[test]
    fn validates_smart_folder_calendar_dates() {
        assert!(valid_date("2024-02-29"));
        assert!(!valid_date("2025-02-29"));
        assert!(!valid_date("2026-13-01"));
        assert!(!valid_date("2026-04-31"));
    }

    #[test]
    fn builds_folder_scoped_queries_with_stable_pagination_order() {
        let mut request = WorkspaceAssetQueryRequest::default();
        request.scope.folder_mode = "folders".to_string();
        request.scope.folder_ids = vec!["folder-a".to_string(), "folder-b".to_string()];
        request.sort.field = "updatedAt".to_string();
        request.sort.order = "desc".to_string();

        let mut query = QueryBuilder::<Postgres>::new("SELECT a.id FROM assets a WHERE ");
        push_conditions(&mut query, "library-a", Uuid::nil(), &request).expect("conditions");
        push_order(&mut query, &request);
        let sql = query.sql();

        assert!(sql.contains("asset_folders"));
        assert!(sql.contains("rel.folder_id = ANY("));
        assert!(sql.contains("ORDER BY a.updated_at DESC"));
        assert!(sql.ends_with("a.id ASC"));
    }
}
