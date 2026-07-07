use crate::{
    auth::AuthUser,
    error::{AppError, AppResult},
    models::StorageRootRecord,
    path_resolver::{validate_aliases, validate_storage_root},
    routes::access::ensure_library_access,
    state::AppState,
};
use axum::{
    extract::{Path, State},
    Json,
};
use uuid::Uuid;

use super::super::{
    queries::{ensure_storage_root_name_available, get_storage_root_record},
    requests::UpdateStorageRootRequest,
};
pub async fn update_storage_root(
    State(state): State<AppState>,
    user: AuthUser,
    Path(root_id): Path<Uuid>,
    Json(request): Json<UpdateStorageRootRequest>,
) -> AppResult<Json<StorageRootRecord>> {
    let existing = get_storage_root_record(&state, root_id).await?;
    let library_role = ensure_library_access(&state, &user, existing.library_id).await?;
    if !user.role.can_manage_server() && !library_role.can_manage_library() {
        return Err(AppError::Forbidden);
    }

    let name = request.name.trim();
    if name.is_empty() {
        return Err(AppError::BadRequest(
            "storage root name is required".to_string(),
        ));
    }
    ensure_storage_root_name_available(&state, existing.library_id, name, Some(root_id)).await?;

    let canonical_uri = request.canonical_uri.trim();
    validate_storage_root(request.kind, canonical_uri)?;
    validate_aliases(&request.windows_mapped_drive_aliases)?;
    validate_aliases(&request.macos_mount_aliases)?;
    if let Some(value) = &request.windows_unc_path {
        validate_aliases(std::slice::from_ref(value))?;
    }
    if let Some(value) = &request.macos_smb_url {
        validate_aliases(std::slice::from_ref(value))?;
    }

    let mut tx = state.pool.begin().await?;
    let root = sqlx::query_as::<_, StorageRootRecord>(
        r#"
        UPDATE storage_roots
        SET
            name = $2,
            kind = $3,
            canonical_uri = $4,
            windows_unc_path = $5,
            windows_mapped_drive_aliases = $6::jsonb,
            macos_smb_url = $7,
            macos_mount_aliases = $8::jsonb,
            enabled = $9,
            updated_at = NOW()
        WHERE id = $1
        RETURNING
            id,
            library_id,
            name,
            kind,
            canonical_uri,
            windows_unc_path,
            windows_mapped_drive_aliases,
            macos_smb_url,
            macos_mount_aliases,
            enabled,
            created_by_user_id,
            created_at,
            updated_at
        "#,
    )
    .bind(root_id)
    .bind(name)
    .bind(request.kind.as_str())
    .bind(canonical_uri)
    .bind(request.windows_unc_path)
    .bind(serde_json::to_value(request.windows_mapped_drive_aliases)?)
    .bind(request.macos_smb_url)
    .bind(serde_json::to_value(request.macos_mount_aliases)?)
    .bind(request.enabled)
    .fetch_one(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO activity_log (id, library_id, actor_user_id, action, target_type, target_id, details)
        VALUES ($1, $2, $3, 'storage_root.updated', 'storage_root', $4, $5::jsonb)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(existing.library_id)
    .bind(user.id)
    .bind(root_id)
    .bind(serde_json::json!({ "name": root.name, "enabled": root.enabled, "kind": root.kind }))
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(Json(root))
}
