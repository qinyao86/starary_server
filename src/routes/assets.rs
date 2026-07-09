use crate::{
    auth::AuthUser, error::AppResult, models::AssetRecord, routes::access::ensure_library_access,
    state::AppState,
};
use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListAssetsQuery {
    #[serde(default = "default_limit")]
    limit: i64,
    #[serde(default)]
    offset: i64,
    #[serde(default)]
    include_deleted: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetListResponse {
    items: Vec<AssetRecord>,
    total: i64,
    limit: i64,
    offset: i64,
}

pub async fn list_assets(
    State(state): State<AppState>,
    user: AuthUser,
    Path(library_id): Path<String>,
    Query(query): Query<ListAssetsQuery>,
) -> AppResult<Json<AssetListResponse>> {
    ensure_library_access(&state, &user, &library_id).await?;

    let limit = query.limit.clamp(1, 500);
    let offset = query.offset.max(0);

    let total: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM assets
        WHERE library_id = $1 AND ($2 OR deleted_at IS NULL)
        "#,
    )
    .bind(&library_id)
    .bind(query.include_deleted)
    .fetch_one(&state.pool)
    .await?;

    let items = sqlx::query_as::<_, AssetRecord>(
        r#"
        SELECT
            id,
            library_id,
            name,
            asset_kind,
            import_mode,
            storage_key,
            storage_root_id,
            relative_path,
            metadata,
            created_by_user_id,
            imported_by_user_id,
            updated_by_user_id,
            deleted_by_user_id,
            restored_by_user_id,
            created_at,
            imported_at,
            updated_at,
            deleted_at,
            restored_at
        FROM assets
        WHERE library_id = $1 AND ($2 OR deleted_at IS NULL)
        ORDER BY created_at DESC, id DESC
        LIMIT $3 OFFSET $4
        "#,
    )
    .bind(&library_id)
    .bind(query.include_deleted)
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(AssetListResponse {
        items,
        total,
        limit,
        offset,
    }))
}

fn default_limit() -> i64 {
    100
}
