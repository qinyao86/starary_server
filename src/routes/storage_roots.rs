use crate::{
    auth::AuthUser, error::AppResult, models::StorageRootRecord,
    routes::access::ensure_library_access, state::AppState,
};
use axum::{
    extract::{Path, Query, State},
    Json,
};
use uuid::Uuid;

mod mutations;
mod queries;
mod requests;

pub use mutations::{create_storage_root, delete_storage_root, update_storage_root};
use queries::{get_storage_root_record, list_storage_root_records};
use requests::ListStorageRootsQuery;

pub async fn list_storage_roots(
    State(state): State<AppState>,
    user: AuthUser,
    Query(query): Query<ListStorageRootsQuery>,
) -> AppResult<Json<Vec<StorageRootRecord>>> {
    ensure_library_access(&state, &user, query.library_id).await?;

    let roots = list_storage_root_records(&state, query.library_id).await?;

    Ok(Json(roots))
}

pub async fn get_storage_root(
    State(state): State<AppState>,
    user: AuthUser,
    Path(root_id): Path<Uuid>,
) -> AppResult<Json<StorageRootRecord>> {
    let root = get_storage_root_record(&state, root_id).await?;

    ensure_library_access(&state, &user, root.library_id).await?;

    Ok(Json(root))
}
