use crate::{
    auth::AuthUser,
    error::{AppError, AppResult},
    models::{StorageRootKind, StorageRootRecord},
    path_resolver::{validate_aliases, validate_storage_root},
    routes::access::ensure_library_access,
    state::AppState,
};
use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;
use uuid::Uuid;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListStorageRootsQuery {
    library_id: Uuid,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateStorageRootRequest {
    library_id: Uuid,
    name: String,
    kind: StorageRootKind,
    canonical_uri: String,
    windows_unc_path: Option<String>,
    #[serde(default)]
    windows_mapped_drive_aliases: Vec<String>,
    macos_smb_url: Option<String>,
    #[serde(default)]
    macos_mount_aliases: Vec<String>,
}

pub async fn list_storage_roots(
    State(state): State<AppState>,
    user: AuthUser,
    Query(query): Query<ListStorageRootsQuery>,
) -> AppResult<Json<Vec<StorageRootRecord>>> {
    ensure_library_access(&state, &user, query.library_id).await?;

    let roots = sqlx::query_as::<_, StorageRootRecord>(
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
    .bind(query.library_id)
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(roots))
}

pub async fn get_storage_root(
    State(state): State<AppState>,
    user: AuthUser,
    Path(root_id): Path<Uuid>,
) -> AppResult<Json<StorageRootRecord>> {
    let root = sqlx::query_as::<_, StorageRootRecord>(
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
    .ok_or_else(|| AppError::NotFound("storage root not found".to_string()))?;

    ensure_library_access(&state, &user, root.library_id).await?;

    Ok(Json(root))
}

pub async fn create_storage_root(
    State(state): State<AppState>,
    user: AuthUser,
    Json(request): Json<CreateStorageRootRequest>,
) -> AppResult<Json<StorageRootRecord>> {
    let library_role = ensure_library_access(&state, &user, request.library_id).await?;
    if !user.role.can_manage_server() && !library_role.can_manage_library() {
        return Err(AppError::Forbidden);
    }

    let name = request.name.trim();
    if name.is_empty() {
        return Err(AppError::BadRequest(
            "storage root name is required".to_string(),
        ));
    }

    validate_storage_root(request.kind, &request.canonical_uri)?;
    validate_aliases(&request.windows_mapped_drive_aliases)?;
    validate_aliases(&request.macos_mount_aliases)?;
    if let Some(value) = &request.windows_unc_path {
        validate_aliases(std::slice::from_ref(value))?;
    }
    if let Some(value) = &request.macos_smb_url {
        validate_aliases(std::slice::from_ref(value))?;
    }

    let root_id = Uuid::new_v4();
    let mut tx = state.pool.begin().await?;
    let root = sqlx::query_as::<_, StorageRootRecord>(
        r#"
        INSERT INTO storage_roots (
            id,
            library_id,
            name,
            kind,
            canonical_uri,
            windows_unc_path,
            windows_mapped_drive_aliases,
            macos_smb_url,
            macos_mount_aliases,
            created_by_user_id
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7::jsonb, $8, $9::jsonb, $10)
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
    .bind(request.library_id)
    .bind(name)
    .bind(request.kind.as_str())
    .bind(request.canonical_uri.trim())
    .bind(request.windows_unc_path)
    .bind(serde_json::to_value(request.windows_mapped_drive_aliases)?)
    .bind(request.macos_smb_url)
    .bind(serde_json::to_value(request.macos_mount_aliases)?)
    .bind(user.id)
    .fetch_one(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO activity_log (id, library_id, actor_user_id, action, target_type, target_id)
        VALUES ($1, $2, $3, 'storage_root.created', 'storage_root', $4)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(request.library_id)
    .bind(user.id)
    .bind(root_id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(Json(root))
}
