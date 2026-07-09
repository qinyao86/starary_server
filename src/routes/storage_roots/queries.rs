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
        FROM storage_roots
        WHERE library_id = $1
        ORDER BY name ASC
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
        FROM storage_roots
        WHERE id = $1
        "#,
    )
    .bind(root_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::NotFound("workspace not found".to_string()))
}

pub async fn ensure_storage_root_name_available(
    state: &AppState,
    library_id: &str,
    name: &str,
    except_root_id: Option<Uuid>,
) -> AppResult<()> {
    let existing: Option<Uuid> = sqlx::query_scalar(
        r#"
        SELECT id
        FROM storage_roots
        WHERE library_id = $1 AND lower(name) = lower($2) AND ($3::uuid IS NULL OR id <> $3)
        "#,
    )
    .bind(library_id)
    .bind(name)
    .bind(except_root_id)
    .fetch_optional(&state.pool)
    .await?;

    if existing.is_some() {
        return Err(AppError::Conflict(
            "workspace name already exists in this library".to_string(),
        ));
    }

    Ok(())
}

pub async fn count_active_assets_for_root(state: &AppState, root_id: Uuid) -> AppResult<i64> {
    Ok(sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM assets
        WHERE storage_root_id = $1 AND deleted_at IS NULL
        "#,
    )
    .bind(root_id)
    .fetch_one(&state.pool)
    .await?)
}
