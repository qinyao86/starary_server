use crate::{
    auth::AuthUser,
    backup::{BackupRecord, BackupSettings, BackupStatus},
    error::{AppError, AppResult},
    state::AppState,
};
use axum::{
    body::Body,
    extract::{Path, State},
    http::{header, StatusCode},
    response::Response,
    Json,
};
use serde::Deserialize;
use tokio::fs::File;
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

fn ensure_server_manager(user: &AuthUser) -> AppResult<()> {
    if user.role.can_manage_server() {
        Ok(())
    } else {
        Err(AppError::Forbidden)
    }
}
