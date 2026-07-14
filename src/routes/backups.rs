use crate::{
    auth::AuthUser,
    backup::{BackupRecord, BackupSettings, BackupStatus},
    error::{AppError, AppResult},
    state::AppState,
};
use axum::{
    body::Body,
    extract::{Multipart, Path, State},
    http::{header, StatusCode},
    response::Response,
    Json,
};
use serde::Deserialize;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tokio_util::io::ReaderStream;

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupOverviewResponse {
    status: BackupStatus,
    backups: Vec<BackupRecord>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreBackupRequest {
    backup_id: String,
}

pub async fn overview(
    State(state): State<AppState>,
    user: AuthUser,
) -> AppResult<Json<BackupOverviewResponse>> {
    ensure_server_manager(&user)?;
    Ok(Json(BackupOverviewResponse {
        status: state.backup_service.status()?,
        backups: state.backup_service.list()?,
    }))
}

pub async fn update_settings(
    State(state): State<AppState>,
    user: AuthUser,
    Json(settings): Json<BackupSettings>,
) -> AppResult<Json<BackupStatus>> {
    ensure_server_manager(&user)?;
    Ok(Json(state.backup_service.update_settings(settings).await?))
}

pub async fn create(
    State(state): State<AppState>,
    user: AuthUser,
) -> AppResult<(StatusCode, Json<BackupRecord>)> {
    ensure_server_manager(&user)?;
    let record = state.backup_service.create_manual().await?;
    Ok((StatusCode::CREATED, Json(record)))
}

pub async fn download(
    State(state): State<AppState>,
    user: AuthUser,
    Path(backup_id): Path<String>,
) -> AppResult<Response> {
    ensure_server_manager(&user)?;
    let path = state.backup_service.backup_path(&backup_id)?;
    let file = File::open(&path)
        .await
        .map_err(|error| AppError::Internal(error.into()))?;
    let size = file
        .metadata()
        .await
        .map_err(|error| AppError::Internal(error.into()))?
        .len();
    let disposition = format!("attachment; filename=\"{backup_id}\"");
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/octet-stream")
        .header(header::CONTENT_LENGTH, size)
        .header(header::CONTENT_DISPOSITION, disposition)
        .body(Body::from_stream(ReaderStream::new(file)))
        .map_err(|error| AppError::Internal(error.into()))
}

pub async fn delete(
    State(state): State<AppState>,
    user: AuthUser,
    Path(backup_id): Path<String>,
) -> AppResult<StatusCode> {
    ensure_server_manager(&user)?;
    state.backup_service.delete(&backup_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn restore(
    State(state): State<AppState>,
    user: AuthUser,
    Json(request): Json<RestoreBackupRequest>,
) -> AppResult<StatusCode> {
    ensure_server_manager(&user)?;
    if !state.service_control.available() {
        return Err(AppError::BadRequest(
            "database restore requires the bundled service runtime".to_string(),
        ));
    }
    state
        .backup_service
        .queue_restore(&request.backup_id)
        .await?;
    state.service_control.request_shutdown()?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn restore_file(
    State(state): State<AppState>,
    user: AuthUser,
    mut multipart: Multipart,
) -> AppResult<StatusCode> {
    ensure_server_manager(&user)?;
    if !state.service_control.available() {
        return Err(AppError::BadRequest(
            "database restore requires the bundled service runtime".to_string(),
        ));
    }
    if !state.backup_service.available() {
        return Err(AppError::BadRequest(
            "PostgreSQL backup tools are not available".to_string(),
        ));
    }

    while let Some(mut field) = multipart
        .next_field()
        .await
        .map_err(|error| AppError::BadRequest(format!("invalid restore upload: {error}")))?
    {
        if field.name() != Some("file") {
            continue;
        }
        let file_name = field.file_name().unwrap_or("backup.dump").to_string();
        if !file_name.to_ascii_lowercase().ends_with(".dump") {
            return Err(AppError::BadRequest(
                "restore file must use the .dump extension".to_string(),
            ));
        }

        let (partial, destination) = state.backup_service.uploaded_restore_paths()?;
        let mut file = File::create(&partial)
            .await
            .map_err(|error| AppError::Internal(error.into()))?;
        let mut size = 0u64;
        while let Some(chunk) = field
            .chunk()
            .await
            .map_err(|error| AppError::BadRequest(format!("invalid restore upload: {error}")))?
        {
            size += chunk.len() as u64;
            file.write_all(&chunk)
                .await
                .map_err(|error| AppError::Internal(error.into()))?;
        }
        file.flush()
            .await
            .map_err(|error| AppError::Internal(error.into()))?;
        drop(file);
        if size == 0 {
            let _ = tokio::fs::remove_file(&partial).await;
            return Err(AppError::BadRequest(
                "restore file cannot be empty".to_string(),
            ));
        }
        tokio::fs::rename(&partial, &destination)
            .await
            .map_err(|error| AppError::Internal(error.into()))?;
        state
            .backup_service
            .queue_uploaded_restore(&destination)
            .await?;
        state.service_control.request_shutdown()?;
        return Ok(StatusCode::NO_CONTENT);
    }

    Err(AppError::BadRequest("restore file is required".to_string()))
}

fn ensure_server_manager(user: &AuthUser) -> AppResult<()> {
    if user.role.can_manage_server() {
        Ok(())
    } else {
        Err(AppError::Forbidden)
    }
}
