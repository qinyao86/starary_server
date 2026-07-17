use crate::{
    auth::AuthUser,
    error::{AppError, AppResult},
    ids::generate_id,
    models::{
        LibraryAccessMode, LibraryRecord, LibraryStatusRecord, LibraryWithRole, Role,
        StorageRootKind,
    },
    path_resolver::{
        ensure_storage_location_exists, ensure_storage_namespace_exists,
        normalize_existing_storage_namespace, normalize_storage_namespace,
        resolve_storage_location_with_policy, resolve_storage_namespace_with_policy,
        storage_identity, storage_locations_overlap, validate_aliases_with_policy,
        validate_storage_root_with_policy,
    },
    routes::{
        access::ensure_library_manager,
        assets::{
            build_asset_file_url, join_safe_relative_path,
            normalize_readable_storage_file_relative_path, storage_root_write_base_path,
            write_file_atomic,
        },
    },
    state::AppState,
};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use base64::{engine::general_purpose, Engine};
use sqlx::{FromRow, Postgres, Transaction};
use std::{fs, path::Path as FilePath, str::FromStr};
use uuid::Uuid;

mod queries;
mod requests;

use queries::{
    list_libraries_for_library_manager, list_libraries_for_member,
    list_libraries_for_server_manager, list_library_statuses_for_library_manager,
    list_library_statuses_for_member, list_library_statuses_for_server_manager,
};
use requests::{
    CreateDefaultStorageRootRequest, CreateLibraryRequest, DeleteLibraryRequest,
    SetLibraryIconFromAssetRequest, StorageBindingRequest, UpdateLibraryEnabledRequest,
    UpdateLibraryRequest, UploadLibraryIconRequest,
};

const LIBRARY_COVER_RELATIVE_PATH: &str = ".madlibrary/cover.webp";
const MAX_LIBRARY_COVER_BYTES: usize = 2 * 1024 * 1024;

#[derive(FromRow)]
struct StorageConnectionLocation {
    id: Uuid,
    kind: String,
    canonical_uri: String,
    windows_unc_path: Option<String>,
    windows_mapped_drive_aliases: serde_json::Value,
    macos_smb_url: Option<String>,
    macos_mount_aliases: serde_json::Value,
    enabled: bool,
}

#[derive(FromRow)]
struct LibraryStorageDeletionTarget {
    kind: String,
    canonical_uri: String,
    storage_identity: String,
    namespace: String,
    connection_kind: String,
    connection_canonical_uri: String,
    connection_windows_unc_path: Option<String>,
    connection_macos_smb_url: Option<String>,
}

fn normalize_optional_text(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn validate_default_storage_root(
    request: &CreateDefaultStorageRootRequest,
    fallback_name: &str,
    allow_personal_paths: bool,
) -> AppResult<String> {
    let name = request
        .name
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(fallback_name)
        .trim();
    if name.is_empty() {
        return Err(AppError::BadRequest(
            "workspace name is required".to_string(),
        ));
    }

    validate_storage_root_with_policy(request.kind, &request.canonical_uri, allow_personal_paths)?;
    validate_aliases_with_policy(&request.windows_mapped_drive_aliases, allow_personal_paths)?;
    validate_aliases_with_policy(&request.macos_mount_aliases, allow_personal_paths)?;
    if let Some(value) = &request.windows_unc_path {
        validate_aliases_with_policy(std::slice::from_ref(value), allow_personal_paths)?;
    }
    if let Some(value) = &request.macos_smb_url {
        validate_aliases_with_policy(std::slice::from_ref(value), allow_personal_paths)?;
    }

    Ok(name.to_string())
}

async fn ensure_unique_library_display_name(
    state: &AppState,
    display_name: &str,
    excluded_library_id: Option<&str>,
) -> AppResult<()> {
    let existing_library_id: Option<String> = sqlx::query_scalar(
        r#"
        SELECT id
        FROM libraries
        WHERE deleted_at IS NULL
          AND lower(display_name) = lower($1)
          AND ($2::text IS NULL OR id <> $2)
        LIMIT 1
        "#,
    )
    .bind(display_name)
    .bind(excluded_library_id)
    .fetch_optional(&state.pool)
    .await?;

    if existing_library_id.is_some() {
        return Err(AppError::Conflict(
            "library display name already exists".to_string(),
        ));
    }

    Ok(())
}

pub async fn list_libraries(
    State(state): State<AppState>,
    user: AuthUser,
) -> AppResult<Json<Vec<LibraryWithRole>>> {
    let libraries = if user.role.can_manage_server() {
        list_libraries_for_server_manager(&state, user.id, user.role.as_str()).await?
    } else {
        list_libraries_for_library_manager(&state, user.id).await?
    };

    Ok(Json(libraries))
}

pub async fn list_my_libraries(
    State(state): State<AppState>,
    user: AuthUser,
) -> AppResult<Json<Vec<LibraryWithRole>>> {
    Ok(Json(list_libraries_for_member(&state, user.id).await?))
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryStatusResponse {
    libraries: Vec<LibraryStatusRecord>,
}

pub async fn list_library_statuses(
    State(state): State<AppState>,
    user: AuthUser,
) -> AppResult<Json<LibraryStatusResponse>> {
    let libraries = if user.role.can_manage_server() {
        list_library_statuses_for_server_manager(&state).await?
    } else {
        list_library_statuses_for_library_manager(&state, user.id).await?
    };

    Ok(Json(LibraryStatusResponse { libraries }))
}

pub async fn list_my_library_statuses(
    State(state): State<AppState>,
    user: AuthUser,
) -> AppResult<Json<LibraryStatusResponse>> {
    let libraries = list_library_statuses_for_member(&state, user.id).await?;
    Ok(Json(LibraryStatusResponse { libraries }))
}

pub async fn create_library(
    State(state): State<AppState>,
    user: AuthUser,
    Json(request): Json<CreateLibraryRequest>,
) -> AppResult<Json<LibraryRecord>> {
    if !user.role.can_create_library() {
        return Err(AppError::Forbidden);
    }

    let display_name = request.display_name.trim();
    if display_name.is_empty() {
        return Err(AppError::BadRequest("library name is required".to_string()));
    }
    ensure_unique_library_display_name(&state, display_name, None).await?;
    let icon_url = normalize_optional_text(request.icon_url);
    let access_mode = request.access_mode.unwrap_or(LibraryAccessMode::Invite);
    if request.default_storage_root.is_some() && request.storage_binding.is_some() {
        return Err(AppError::BadRequest(
            "choose either a new storage location or an existing storage connection".to_string(),
        ));
    }
    let default_storage_root_name = request
        .default_storage_root
        .as_ref()
        .map(|root| {
            validate_default_storage_root(
                root,
                display_name,
                state.config.allow_personal_storage_paths,
            )
        })
        .transpose()?;

    let library_id = generate_id("lib_");
    let mut tx = state.pool.begin().await?;

    let library = sqlx::query_as::<_, LibraryRecord>(
        r#"
        INSERT INTO libraries (id, display_name, icon_url, enabled, access_mode, created_by_user_id)
        VALUES ($1, $2, $3, TRUE, $4, $5)
        RETURNING id, display_name, icon_url, enabled, access_mode, created_by_user_id, created_at, updated_at
        "#,
    )
    .bind(&library_id)
    .bind(display_name)
    .bind(icon_url)
    .bind(access_mode.as_str())
    .bind(user.id)
    .fetch_one(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO library_memberships (library_id, user_id, role)
        VALUES ($1, $2, $3)
        "#,
    )
    .bind(&library_id)
    .bind(user.id)
    .bind(Role::LibraryManager.as_str())
    .execute(&mut *tx)
    .await?;

    if let Some(default_storage_root) = request.default_storage_root {
        let root_name = default_storage_root_name.as_deref().unwrap_or("Workspace");
        let root_location = resolve_storage_location_with_policy(
            default_storage_root.kind,
            &default_storage_root.canonical_uri,
            default_storage_root.windows_unc_path.clone(),
            default_storage_root.macos_smb_url.clone(),
            state.config.allow_personal_storage_paths,
        )?;
        ensure_storage_location_exists(default_storage_root.kind, &root_location)?;
        let connection_id = Uuid::new_v4();
        let connection_name = unique_connection_name(&mut tx, display_name).await?;
        sqlx::query("SELECT pg_advisory_xact_lock(hashtext('madlibrary.default-storage'))")
            .execute(&mut *tx)
            .await?;
        let should_be_default: bool = sqlx::query_scalar(
            "SELECT NOT EXISTS(SELECT 1 FROM storage_connections WHERE is_default AND enabled)",
        )
        .fetch_one(&mut *tx)
        .await?;
        sqlx::query(
            r#"
            INSERT INTO storage_connections (
                id, name, kind, canonical_uri, windows_unc_path,
                windows_mapped_drive_aliases, macos_smb_url, macos_mount_aliases,
                is_default, created_by_user_id
            )
            VALUES ($1, $2, $3, $4, $5, $6::jsonb, $7, $8::jsonb, $9, $10)
            "#,
        )
        .bind(connection_id)
        .bind(connection_name)
        .bind(default_storage_root.kind.as_str())
        .bind(root_location.canonical_uri)
        .bind(root_location.windows_unc_path)
        .bind(serde_json::to_value(
            default_storage_root.windows_mapped_drive_aliases,
        )?)
        .bind(root_location.macos_smb_url)
        .bind(serde_json::to_value(
            default_storage_root.macos_mount_aliases,
        )?)
        .bind(should_be_default)
        .bind(user.id)
        .execute(&mut *tx)
        .await?;
        bind_library_storage(
            &state,
            &mut tx,
            &library_id,
            user.id,
            StorageBindingRequest {
                connection_id,
                namespace: None,
            },
            root_name,
        )
        .await?;
    } else if let Some(binding) = request.storage_binding {
        bind_library_storage(&state, &mut tx, &library_id, user.id, binding, display_name).await?;
    } else {
        let connection_id: Option<Uuid> = sqlx::query_scalar(
            "SELECT id FROM storage_connections WHERE is_default AND enabled LIMIT 1",
        )
        .fetch_optional(&mut *tx)
        .await?;
        let connection_id = connection_id.ok_or_else(|| {
            AppError::BadRequest("default storage connection is not configured".to_string())
        })?;
        bind_library_storage(
            &state,
            &mut tx,
            &library_id,
            user.id,
            StorageBindingRequest {
                connection_id,
                namespace: None,
            },
            display_name,
        )
        .await?;
    }

    sqlx::query(
        r#"
        INSERT INTO activity_log (id, library_id, actor_user_id, action, target_type, target_id)
        VALUES ($1, $2, $3, 'library.created', 'library', $2)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(&library_id)
    .bind(user.id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(Json(library))
}

pub async fn update_library(
    State(state): State<AppState>,
    user: AuthUser,
    Path(library_id): Path<String>,
    Json(request): Json<UpdateLibraryRequest>,
) -> AppResult<Json<LibraryRecord>> {
    ensure_library_manager(&state, &user, &library_id).await?;

    let display_name = request.display_name.trim();
    if display_name.is_empty() {
        return Err(AppError::BadRequest("library name is required".to_string()));
    }
    ensure_unique_library_display_name(&state, display_name, Some(&library_id)).await?;
    let access_mode = request.access_mode.map(|mode| mode.as_str().to_string());

    let mut tx = state.pool.begin().await?;
    let library = sqlx::query_as::<_, LibraryRecord>(
        r#"
        UPDATE libraries
        SET display_name = $2, access_mode = COALESCE($3, access_mode), updated_at = NOW()
        WHERE id = $1 AND deleted_at IS NULL
        RETURNING id, display_name, icon_url, enabled, access_mode, created_by_user_id, created_at, updated_at
        "#,
    )
    .bind(&library_id)
    .bind(display_name)
    .bind(access_mode.as_deref())
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| AppError::NotFound("library not found".to_string()))?;

    sqlx::query(
        r#"
        INSERT INTO activity_log (id, library_id, actor_user_id, action, target_type, target_id)
        VALUES ($1, $2, $3, 'library.updated', 'library', $2)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(&library_id)
    .bind(user.id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(Json(library))
}

pub async fn upload_library_icon(
    State(state): State<AppState>,
    user: AuthUser,
    Path(library_id): Path<String>,
    Json(request): Json<UploadLibraryIconRequest>,
) -> AppResult<Json<LibraryRecord>> {
    ensure_library_manager(&state, &user, &library_id).await?;
    let bytes = decode_library_cover(&request)?;
    let storage_root_id = primary_library_storage_root_id(&state, &library_id).await?;
    let base_path =
        storage_root_write_base_path(&state, storage_root_id, Some(&library_id)).await?;
    let cover_path = join_safe_relative_path(&base_path, LIBRARY_COVER_RELATIVE_PATH);
    write_file_atomic(&cover_path, &bytes)?;
    let icon_url = build_asset_file_url(&library_id, storage_root_id, LIBRARY_COVER_RELATIVE_PATH)
        .ok_or_else(|| AppError::BadRequest("library icon path is invalid".to_string()))?;
    Ok(Json(
        update_library_icon_reference(&state, &user, &library_id, Some(icon_url)).await?,
    ))
}

pub async fn set_library_icon_from_asset(
    State(state): State<AppState>,
    user: AuthUser,
    Path(library_id): Path<String>,
    Json(request): Json<SetLibraryIconFromAssetRequest>,
) -> AppResult<Json<LibraryRecord>> {
    ensure_library_manager(&state, &user, &library_id).await?;
    let asset_id = request.asset_id.trim();
    if asset_id.is_empty() {
        return Err(AppError::BadRequest("asset id is required".to_string()));
    }

    let row = sqlx::query(
        r#"
        SELECT asset_kind, storage_root_id, metadata->>'thumbnailPath' AS thumbnail_path
        FROM assets
        WHERE library_id = $1 AND id = $2 AND deleted_at IS NULL
        "#,
    )
    .bind(&library_id)
    .bind(asset_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::NotFound("asset not found".to_string()))?;
    let asset_kind: String = sqlx::Row::try_get(&row, "asset_kind")?;
    if asset_kind != "image" && asset_kind != "video" {
        return Err(AppError::BadRequest(
            "only image or video thumbnails can be used as a library icon".to_string(),
        ));
    }
    let storage_root_id: Option<Uuid> = sqlx::Row::try_get(&row, "storage_root_id")?;
    let storage_root_id = storage_root_id.ok_or_else(|| {
        AppError::BadRequest("asset thumbnail has no storage workspace".to_string())
    })?;
    let thumbnail_path: Option<String> = sqlx::Row::try_get(&row, "thumbnail_path")?;
    let thumbnail_path = thumbnail_path
        .filter(|path| !path.trim().is_empty())
        .ok_or_else(|| AppError::BadRequest("asset has no thumbnail".to_string()))?;
    let thumbnail_path = normalize_readable_storage_file_relative_path(&thumbnail_path)?;
    let base_path =
        storage_root_write_base_path(&state, storage_root_id, Some(&library_id)).await?;
    if !join_safe_relative_path(&base_path, &thumbnail_path).is_file() {
        return Err(AppError::NotFound(
            "asset thumbnail file not found".to_string(),
        ));
    }
    let icon_url = build_asset_file_url(&library_id, storage_root_id, &thumbnail_path)
        .ok_or_else(|| AppError::BadRequest("asset thumbnail path is invalid".to_string()))?;

    remove_managed_library_covers(&state, &library_id).await;
    Ok(Json(
        update_library_icon_reference(&state, &user, &library_id, Some(icon_url)).await?,
    ))
}

pub async fn clear_library_icon(
    State(state): State<AppState>,
    user: AuthUser,
    Path(library_id): Path<String>,
) -> AppResult<Json<LibraryRecord>> {
    ensure_library_manager(&state, &user, &library_id).await?;
    remove_managed_library_covers(&state, &library_id).await;
    Ok(Json(
        update_library_icon_reference(&state, &user, &library_id, None).await?,
    ))
}

fn decode_library_cover(request: &UploadLibraryIconRequest) -> AppResult<Vec<u8>> {
    let encoded = request
        .content_base64
        .split_once(',')
        .map(|(_, payload)| payload)
        .unwrap_or(request.content_base64.as_str())
        .trim();
    let bytes = general_purpose::STANDARD
        .decode(encoded)
        .map_err(|_| AppError::BadRequest("library icon is not valid base64".to_string()))?;
    if bytes.is_empty() || bytes.len() > MAX_LIBRARY_COVER_BYTES {
        return Err(AppError::BadRequest(
            "library icon must be between 1 byte and 2 MB".to_string(),
        ));
    }
    if request
        .size_bytes
        .is_some_and(|size| size != bytes.len() as u64)
    {
        return Err(AppError::BadRequest(
            "library icon size does not match its payload".to_string(),
        ));
    }
    if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WEBP" {
        return Err(AppError::BadRequest(
            "library icon must be a WebP image".to_string(),
        ));
    }
    Ok(bytes)
}

async fn primary_library_storage_root_id(state: &AppState, library_id: &str) -> AppResult<Uuid> {
    sqlx::query_scalar(
        r#"
        SELECT sr.id
        FROM storage_roots sr
        INNER JOIN storage_connections sc ON sc.id = sr.storage_connection_id
        WHERE sr.library_id = $1 AND sr.enabled = TRUE AND sc.enabled = TRUE
        ORDER BY sr.created_at ASC
        LIMIT 1
        "#,
    )
    .bind(library_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::BadRequest("library has no enabled storage".to_string()))
}

async fn remove_managed_library_covers(state: &AppState, library_id: &str) {
    let root_ids: Vec<Uuid> =
        sqlx::query_scalar("SELECT id FROM storage_roots WHERE library_id = $1 AND enabled = TRUE")
            .bind(library_id)
            .fetch_all(&state.pool)
            .await
            .unwrap_or_default();
    for root_id in root_ids {
        if let Ok(base_path) = storage_root_write_base_path(state, root_id, Some(library_id)).await
        {
            let _ = fs::remove_file(join_safe_relative_path(
                &base_path,
                LIBRARY_COVER_RELATIVE_PATH,
            ));
        }
    }
}

async fn update_library_icon_reference(
    state: &AppState,
    user: &AuthUser,
    library_id: &str,
    icon_url: Option<String>,
) -> AppResult<LibraryRecord> {
    let mut tx = state.pool.begin().await?;
    let library = sqlx::query_as::<_, LibraryRecord>(
        r#"
        UPDATE libraries
        SET icon_url = $2, updated_at = NOW()
        WHERE id = $1 AND deleted_at IS NULL
        RETURNING id, display_name, icon_url, enabled, access_mode, created_by_user_id, created_at, updated_at
        "#,
    )
    .bind(library_id)
    .bind(icon_url)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| AppError::NotFound("library not found".to_string()))?;
    sqlx::query(
        r#"
        INSERT INTO activity_log (id, library_id, actor_user_id, action, target_type, target_id)
        VALUES ($1, $2, $3, 'library.icon_updated', 'library', $2)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(library_id)
    .bind(user.id)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(library)
}

pub async fn assign_library_storage(
    State(state): State<AppState>,
    user: AuthUser,
    Path(library_id): Path<String>,
    Json(binding): Json<StorageBindingRequest>,
) -> AppResult<StatusCode> {
    ensure_library_manager(&state, &user, &library_id).await?;

    let mut tx = state.pool.begin().await?;
    let display_name: String = sqlx::query_scalar(
        r#"
        SELECT display_name
        FROM libraries
        WHERE id = $1 AND deleted_at IS NULL
        FOR UPDATE
        "#,
    )
    .bind(&library_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| AppError::NotFound("library not found".to_string()))?;

    let has_storage: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM storage_roots WHERE library_id = $1)")
            .bind(&library_id)
            .fetch_one(&mut *tx)
            .await?;
    if has_storage {
        return Err(AppError::Conflict(
            "library storage is already assigned; use the storage migration workflow".to_string(),
        ));
    }

    bind_library_storage(
        &state,
        &mut tx,
        &library_id,
        user.id,
        binding,
        &display_name,
    )
    .await?;

    sqlx::query(
        r#"
        INSERT INTO activity_log (id, library_id, actor_user_id, action, target_type, target_id)
        VALUES ($1, $2, $3, 'library.storage_assigned', 'library', $2)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(&library_id)
    .bind(user.id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn bind_library_storage(
    state: &AppState,
    tx: &mut Transaction<'_, Postgres>,
    library_id: &str,
    user_id: Uuid,
    binding: StorageBindingRequest,
    workspace_name: &str,
) -> AppResult<()> {
    sqlx::query("SELECT pg_advisory_xact_lock(hashtext('madlibrary.storage-bindings'))")
        .execute(&mut **tx)
        .await?;
    let connection = get_enabled_connection(tx, binding.connection_id).await?;
    let kind = StorageRootKind::from_str(&connection.kind).map_err(AppError::BadRequest)?;
    let namespace = normalize_storage_namespace(binding.namespace.as_deref(), library_id)?;
    let location = resolve_storage_namespace_with_policy(
        kind,
        &connection.canonical_uri,
        &namespace,
        connection.windows_unc_path,
        connection.macos_smb_url,
        state.config.allow_personal_storage_paths,
    )?;
    ensure_exclusive_storage_location(tx, library_id, kind, &location.canonical_uri).await?;
    ensure_storage_namespace_exists(kind, &location)?;
    let identity = storage_identity(&location.canonical_uri);
    let root_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO storage_roots (
            id, library_id, storage_connection_id, namespace, name, kind,
            canonical_uri, storage_identity, windows_unc_path, windows_mapped_drive_aliases,
            macos_smb_url, macos_mount_aliases, created_by_user_id
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10::jsonb, $11, $12::jsonb, $13)
        "#,
    )
    .bind(root_id)
    .bind(library_id)
    .bind(connection.id)
    .bind(namespace)
    .bind(workspace_name)
    .bind(kind.as_str())
    .bind(location.canonical_uri)
    .bind(identity)
    .bind(location.windows_unc_path)
    .bind(connection.windows_mapped_drive_aliases)
    .bind(location.macos_smb_url)
    .bind(connection.macos_mount_aliases)
    .bind(user_id)
    .execute(&mut **tx)
    .await
    .map_err(map_storage_binding_error)?;
    Ok(())
}

async fn ensure_exclusive_storage_location(
    tx: &mut Transaction<'_, Postgres>,
    library_id: &str,
    kind: StorageRootKind,
    canonical_uri: &str,
) -> AppResult<()> {
    let existing_locations: Vec<(String, String)> = sqlx::query_as(
        r#"
        SELECT sr.canonical_uri, l.display_name
        FROM storage_roots sr
        INNER JOIN libraries l ON l.id = sr.library_id
        WHERE sr.library_id <> $1 AND sr.kind = $2
        "#,
    )
    .bind(library_id)
    .bind(kind.as_str())
    .fetch_all(&mut **tx)
    .await?;

    if let Some((_, library_name)) = existing_locations
        .into_iter()
        .find(|(existing, _)| storage_locations_overlap(existing, canonical_uri))
    {
        return Err(AppError::StorageLocationConflict(format!(
            "the final path overlaps the path reserved by library '{library_name}'"
        )));
    }
    Ok(())
}

fn map_storage_binding_error(error: sqlx::Error) -> AppError {
    if let sqlx::Error::Database(database_error) = &error {
        if database_error.code().as_deref() == Some("23505") {
            return AppError::StorageLocationConflict(
                "the final path is already reserved by another library".to_string(),
            );
        }
    }
    AppError::Database(error)
}

async fn get_enabled_connection(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
) -> AppResult<StorageConnectionLocation> {
    sqlx::query_as::<_, StorageConnectionLocation>(
        r#"
        SELECT id, kind, canonical_uri, windows_unc_path,
               windows_mapped_drive_aliases, macos_smb_url, macos_mount_aliases, enabled
        FROM storage_connections WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(&mut **tx)
    .await?
    .filter(|connection| connection.enabled)
    .ok_or_else(|| AppError::BadRequest("storage connection is missing or disabled".to_string()))
}

async fn unique_connection_name(
    tx: &mut Transaction<'_, Postgres>,
    library_name: &str,
) -> AppResult<String> {
    let base = format!("{library_name} Storage");
    for suffix in 0..1000 {
        let candidate = if suffix == 0 {
            base.clone()
        } else {
            format!("{base} {suffix}")
        };
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM storage_connections WHERE lower(name) = lower($1))",
        )
        .bind(&candidate)
        .fetch_one(&mut **tx)
        .await?;
        if !exists {
            return Ok(candidate);
        }
    }
    Err(AppError::Conflict(
        "could not allocate a unique storage connection name".to_string(),
    ))
}

pub async fn update_library_enabled(
    State(state): State<AppState>,
    user: AuthUser,
    Path(library_id): Path<String>,
    Json(request): Json<UpdateLibraryEnabledRequest>,
) -> AppResult<Json<LibraryRecord>> {
    ensure_library_manager(&state, &user, &library_id).await?;

    let mut tx = state.pool.begin().await?;
    let library = sqlx::query_as::<_, LibraryRecord>(
        r#"
        UPDATE libraries
        SET enabled = $2, updated_at = NOW()
        WHERE id = $1 AND deleted_at IS NULL
        RETURNING id, display_name, icon_url, enabled, access_mode, created_by_user_id, created_at, updated_at
        "#,
    )
    .bind(&library_id)
    .bind(request.enabled)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| AppError::NotFound("library not found".to_string()))?;

    sqlx::query(
        r#"
        INSERT INTO activity_log (id, library_id, actor_user_id, action, target_type, target_id, details)
        VALUES ($1, $2, $3, $4, 'library', $2, $5::jsonb)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(&library_id)
    .bind(user.id)
    .bind(if request.enabled {
        "library.enabled"
    } else {
        "library.disabled"
    })
    .bind(serde_json::json!({ "enabled": request.enabled }))
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(Json(library))
}

pub async fn join_library(
    State(state): State<AppState>,
    user: AuthUser,
    Path(library_id): Path<String>,
) -> AppResult<StatusCode> {
    let mut tx = state.pool.begin().await?;
    let library: Option<(String, bool)> = sqlx::query_as(
        r#"
        SELECT access_mode, enabled
        FROM libraries
        WHERE id = $1 AND deleted_at IS NULL
        FOR UPDATE
        "#,
    )
    .bind(&library_id)
    .fetch_optional(&mut *tx)
    .await?;
    let Some((access_mode, enabled)) = library else {
        return Err(AppError::NotFound("library not found".to_string()));
    };
    if access_mode != LibraryAccessMode::Public.as_str() {
        return Err(AppError::BadRequest("library is invite only".to_string()));
    }
    if !enabled {
        return Err(AppError::LibraryDisabled(library_id));
    }

    let inserted = sqlx::query(
        r#"
        INSERT INTO library_memberships (library_id, user_id, role)
        VALUES ($1, $2, $3)
        ON CONFLICT (library_id, user_id) DO NOTHING
        "#,
    )
    .bind(&library_id)
    .bind(user.id)
    .bind(Role::Viewer.as_str())
    .execute(&mut *tx)
    .await?
    .rows_affected();

    if inserted > 0 {
        sqlx::query(
            r#"
            INSERT INTO activity_log (id, library_id, actor_user_id, action, target_type, target_id, details)
            VALUES ($1, $2, $3, 'library.member_joined', 'user', $3, $4::jsonb)
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(&library_id)
        .bind(user.id)
        .bind(serde_json::json!({ "role": Role::Viewer.as_str() }))
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn delete_library(
    State(state): State<AppState>,
    user: AuthUser,
    Path(library_id): Path<String>,
    request: Option<Json<DeleteLibraryRequest>>,
) -> AppResult<StatusCode> {
    if !user.role.can_manage_server() {
        return Err(AppError::Forbidden);
    }
    let request = request.map(|Json(request)| request).unwrap_or_default();

    let mut tx = state.pool.begin().await?;
    let display_name: Option<String> = sqlx::query_scalar(
        "SELECT display_name FROM libraries WHERE id = $1 AND deleted_at IS NULL FOR UPDATE",
    )
    .bind(&library_id)
    .fetch_optional(&mut *tx)
    .await?;
    let display_name =
        display_name.ok_or_else(|| AppError::NotFound("library not found".to_string()))?;

    if request.delete_files {
        delete_library_storage_files(&state, &mut tx, &library_id).await?;
    }

    sqlx::query(
        r#"
        INSERT INTO activity_log (id, actor_user_id, action, target_type, target_id, details)
        VALUES ($1, $2, 'library.deleted', 'library', $3, $4::jsonb)
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(user.id)
    .bind(&library_id)
    .bind(serde_json::json!({
        "deleteFiles": request.delete_files,
        "name": display_name,
    }))
    .execute(&mut *tx)
    .await?;

    let deleted = sqlx::query("DELETE FROM libraries WHERE id = $1 AND deleted_at IS NULL")
        .bind(&library_id)
        .execute(&mut *tx)
        .await?;
    if deleted.rows_affected() == 0 {
        return Err(AppError::NotFound("library not found".to_string()));
    }

    tx.commit().await?;

    Ok(StatusCode::NO_CONTENT)
}

async fn delete_library_storage_files(
    state: &AppState,
    tx: &mut Transaction<'_, Postgres>,
    library_id: &str,
) -> AppResult<()> {
    let targets = sqlx::query_as::<_, LibraryStorageDeletionTarget>(
        r#"
        SELECT
            sr.kind,
            sr.canonical_uri,
            sr.storage_identity,
            sr.namespace,
            sc.kind AS connection_kind,
            sc.canonical_uri AS connection_canonical_uri,
            sc.windows_unc_path AS connection_windows_unc_path,
            sc.macos_smb_url AS connection_macos_smb_url
        FROM storage_roots sr
        INNER JOIN storage_connections sc ON sc.id = sr.storage_connection_id
        WHERE sr.library_id = $1
        FOR UPDATE
        "#,
    )
    .bind(library_id)
    .fetch_all(&mut **tx)
    .await?;

    if targets.len() > 1 {
        return Err(AppError::Conflict(
            "library has multiple storage bindings; remove files with the storage migration workflow"
                .to_string(),
        ));
    }

    let Some(target) = targets.into_iter().next() else {
        return Ok(());
    };
    let kind = StorageRootKind::from_str(&target.kind).map_err(AppError::BadRequest)?;
    if target.connection_kind != target.kind {
        return Err(AppError::Conflict(
            "storage binding no longer matches its storage connection".to_string(),
        ));
    }

    let namespace = normalize_existing_storage_namespace(&target.namespace)?;
    if namespace.is_empty() {
        return Err(AppError::BadRequest(
            "library files can only be deleted from an isolated library storage folder".to_string(),
        ));
    }
    let expected_location = resolve_storage_namespace_with_policy(
        kind,
        &target.connection_canonical_uri,
        &namespace,
        target.connection_windows_unc_path,
        target.connection_macos_smb_url,
        state.config.allow_personal_storage_paths,
    )?;
    if storage_identity(&expected_location.canonical_uri) != target.storage_identity
        || storage_identity(&target.canonical_uri) != target.storage_identity
    {
        return Err(AppError::Conflict(
            "storage binding does not resolve to its recorded library folder".to_string(),
        ));
    }

    let path = match kind {
        StorageRootKind::ServerFilesystem => expected_location.canonical_uri,
        StorageRootKind::Smb => expected_location.windows_unc_path.ok_or_else(|| {
            AppError::BadRequest(
                "shared folder files can only be deleted by a Windows server with a resolved UNC path"
                    .to_string(),
            )
        })?,
        StorageRootKind::S3 => {
            return Err(AppError::BadRequest(
                "deleting object storage files is not supported yet; remove them with the object storage lifecycle workflow"
                    .to_string(),
            ));
        }
    };

    remove_library_storage_directory(&path)
}

fn remove_library_storage_directory(path: &str) -> AppResult<()> {
    let path = FilePath::new(path);
    if path.as_os_str().is_empty() || path.parent().is_none() {
        return Err(AppError::BadRequest(
            "library storage folder is not a deletable directory".to_string(),
        ));
    }

    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(AppError::Internal(anyhow::anyhow!(
                "could not inspect library storage folder {}: {error}",
                path.display()
            )));
        }
    };
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(AppError::BadRequest(
            "library storage folder must be a real directory before it can be deleted".to_string(),
        ));
    }

    fs::remove_dir_all(path).map_err(|error| {
        AppError::Internal(anyhow::anyhow!(
            "could not delete library storage folder {}: {error}",
            path.display()
        ))
    })
}
