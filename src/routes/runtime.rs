use crate::{
    auth::AuthUser,
    error::{AppError, AppResult},
    state::AppState,
};
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    Json,
};
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeSettingsResponse {
    current_port: u16,
    configured_port: u16,
    restart_required: bool,
    service_control_available: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateRuntimeSettingsRequest {
    port: u16,
}

pub async fn settings(
    State(state): State<AppState>,
    user: AuthUser,
) -> AppResult<Json<RuntimeSettingsResponse>> {
    ensure_server_manager(&user)?;
    Ok(Json(response(&state)))
}

pub async fn update_settings(
    State(state): State<AppState>,
    user: AuthUser,
    Json(request): Json<UpdateRuntimeSettingsRequest>,
) -> AppResult<Json<RuntimeSettingsResponse>> {
    ensure_server_manager(&user)?;
    ensure_controls_available(&state)?;
    if request.port < 1024 {
        return Err(AppError::BadRequest(
            "server port must be between 1024 and 65535".to_string(),
        ));
    }

    if request.port != state.config.port {
        let probe_addr = format!("{}:{}", state.config.host, request.port);
        let listener = TcpListener::bind(&probe_addr).await.map_err(|_| {
            AppError::Conflict(format!("server port {} is already in use", request.port))
        })?;
        drop(listener);
    }

    state.service_control.update_port(request.port)?;
    Ok(Json(response(&state)))
}

pub async fn shutdown(State(state): State<AppState>, user: AuthUser) -> AppResult<StatusCode> {
    ensure_server_manager(&user)?;
    ensure_controls_available(&state)?;
    state.service_control.request_shutdown()?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn desktop_shutdown(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<StatusCode> {
    let expected = state
        .config
        .desktop_control_token
        .as_deref()
        .ok_or_else(|| AppError::NotFound("desktop control is unavailable".to_string()))?;
    let provided = headers
        .get("x-madlibrary-control-token")
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();
    if provided != expected {
        return Err(AppError::Forbidden);
    }

    state.service_control.request_shutdown()?;
    Ok(StatusCode::NO_CONTENT)
}

fn response(state: &AppState) -> RuntimeSettingsResponse {
    let configured_port = state.service_control.configured_port();
    RuntimeSettingsResponse {
        current_port: state.config.port,
        configured_port,
        restart_required: configured_port != state.config.port,
        service_control_available: state.service_control.available(),
    }
}

fn ensure_server_manager(user: &AuthUser) -> AppResult<()> {
    if user.role.can_manage_server() {
        Ok(())
    } else {
        Err(AppError::Forbidden)
    }
}

fn ensure_controls_available(state: &AppState) -> AppResult<()> {
    if state.service_control.available() {
        Ok(())
    } else {
        Err(AppError::BadRequest(
            "service controls are managed by the deployment environment".to_string(),
        ))
    }
}
