use std::fs::OpenOptions;

use crate::{error::AppResult, state::AppState};
use axum::{extract::State, Json};
use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthResponse {
    status: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerInfoResponse {
    product: &'static str,
    version: &'static str,
    api_version: &'static str,
    deployment_mode: String,
    server_url: String,
    storage_dir: String,
    admin_available: bool,
    database_status: &'static str,
    storage_status: &'static str,
    storage_writable: bool,
}

pub async fn health() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}

pub async fn server_info(State(state): State<AppState>) -> AppResult<Json<ServerInfoResponse>> {
    let storage_exists = state.config.storage_dir.exists();
    let storage_writable = storage_exists && storage_is_writable(&state.config.storage_dir);

    Ok(Json(ServerInfoResponse {
        product: "Mad Library Team Server",
        version: env!("CARGO_PKG_VERSION"),
        api_version: "v1",
        deployment_mode: state.config.deployment_mode.clone(),
        server_url: format!("http://{}", state.config.bind_addr()),
        storage_dir: state.config.storage_dir.display().to_string(),
        admin_available: state
            .config
            .resolved_admin_assets_dir()
            .join("index.html")
            .exists(),
        database_status: "connected",
        storage_status: if storage_writable {
            "writable"
        } else if storage_exists {
            "read_only"
        } else {
            "missing"
        },
        storage_writable,
    }))
}

fn storage_is_writable(path: &std::path::Path) -> bool {
    let probe = path.join(".madlibrary-server-write-test");
    let opened = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&probe);

    if opened.is_err() {
        return false;
    }

    let _ = std::fs::remove_file(probe);
    true
}
