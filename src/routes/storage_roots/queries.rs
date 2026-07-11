use crate::{
    error::{AppError, AppResult},
    models::StorageRootRecord,
    state::AppState,
};
use uuid::Uuid;

pub async fn list_storage_root_records(
    state: &AppState,
    library_id: &str,
) -> AppResult<Vec<StorageRootRecord>> {
    Ok(sqlx::query_as::<_, StorageRootRecord>(
        r#"
        SELECT
            sr.id,
            sr.library_id,
            sr.storage_connection_id,
            sc.name AS storage_connection_name,
            sr.namespace,
            sr.name,
            sr.kind,
            sr.canonical_uri,
            sr.windows_unc_path,
            sr.windows_mapped_drive_aliases,
            sr.macos_smb_url,
            sr.macos_mount_aliases,
            sr.enabled,
            sr.created_by_user_id,
            sr.created_at,
            sr.updated_at
        FROM storage_roots sr
        INNER JOIN storage_connections sc ON sc.id = sr.storage_connection_id
        WHERE sr.library_id = $1
        ORDER BY sr.name ASC
        "#,
    )
    .bind(library_id)
    .fetch_all(&state.pool)
    .await?)
}

pub async fn get_storage_root_record(
    state: &AppState,
    root_id: Uuid,
) -> AppResult<StorageRootRecord> {
    sqlx::query_as::<_, StorageRootRecord>(
        r#"
        SELECT
            sr.id,
            sr.library_id,
            sr.storage_connection_id,
            sc.name AS storage_connection_name,
            sr.namespace,
            sr.name,
            sr.kind,
            sr.canonical_uri,
            sr.windows_unc_path,
            sr.windows_mapped_drive_aliases,
            sr.macos_smb_url,
            sr.macos_mount_aliases,
            sr.enabled,
            sr.created_by_user_id,
            sr.created_at,
            sr.updated_at
        FROM storage_roots sr
        INNER JOIN storage_connections sc ON sc.id = sr.storage_connection_id
        WHERE sr.id = $1
        "#,
    )
    .bind(root_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::NotFound("workspace not found".to_string()))
}
