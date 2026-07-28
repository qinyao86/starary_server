use std::fs::OpenOptions;

use crate::{error::AppResult, state::AppState};
use axum::{extract::State, http::HeaderMap, Json};
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
    local_url: String,
    lan_url: Option<String>,
    local_admin_url: String,
    lan_admin_url: Option<String>,
    bind_address: String,
    storage_dir: String,
    admin_available: bool,
    database_status: &'static str,
    storage_status: &'static str,
    storage_writable: bool,
}

pub async fn health() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}

pub async fn server_info(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<ServerInfoResponse>> {
    let storage_exists = state.config.storage_dir.exists();
    let storage_writable = storage_exists && storage_is_writable(&state.config.storage_dir);
    let local_url = format!("http://127.0.0.1:{}", state.config.port);
    let request_url = request_url(&headers);
    let lan_url = if request_url
        .as_deref()
        .is_some_and(|url| !url_contains_loopback_host(url))
    {
        request_url
    } else {
        primary_lan_ipv4().map(|ip| format!("http://{ip}:{}", state.config.port))
    };
    let server_url = lan_url.clone().unwrap_or_else(|| local_url.clone());
    let local_admin_url = format!("{local_url}/admin/");
    let lan_admin_url = lan_url.as_ref().map(|url| format!("{url}/admin/"));

    Ok(Json(ServerInfoResponse {
        product: "Starary Server",
        version: env!("CARGO_PKG_VERSION"),
        api_version: "v1",
        deployment_mode: state.config.deployment_mode.clone(),
        server_url,
        local_url,
        lan_url,
        local_admin_url,
        lan_admin_url,
        bind_address: state.config.bind_addr(),
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

fn request_url(headers: &HeaderMap) -> Option<String> {
    let host = headers.get("host")?.to_str().ok()?.trim();
    if host.is_empty() {
        None
    } else {
        Some(format!("http://{host}"))
    }
}

fn url_contains_loopback_host(url: &str) -> bool {
    url.starts_with("http://127.")
        || url.starts_with("http://localhost")
        || url.starts_with("http://[::1]")
}

fn primary_lan_ipv4() -> Option<std::net::Ipv4Addr> {
    let socket = std::net::UdpSocket::bind((std::net::Ipv4Addr::UNSPECIFIED, 0)).ok()?;
    socket
        .connect((std::net::Ipv4Addr::new(192, 0, 2, 1), 80))
        .ok()?;
    match socket.local_addr().ok()?.ip() {
        std::net::IpAddr::V4(ip)
            if !ip.is_loopback() && !ip.is_unspecified() && !ip.is_link_local() =>
        {
            Some(ip)
        }
        _ => None,
    }
}

fn storage_is_writable(path: &std::path::Path) -> bool {
    let probe = path.join(".starary-server-write-test");
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
