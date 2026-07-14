use serde::Deserialize;
use std::{
    io::{Read, Write},
    net::TcpStream,
    time::Duration,
};

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct IdentityResponse {
    pub(super) product: String,
    pub(super) instance_id: String,
    pub(super) process_id: u32,
    pub(super) port: u16,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ServerInfoResponse {
    pub(super) lan_url: Option<String>,
    pub(super) storage_status: String,
}

pub(super) struct HttpResponse {
    pub(super) status: u16,
    pub(super) body: String,
}

pub(super) fn request(
    port: u16,
    method: &str,
    path: &str,
    headers: &[String],
) -> std::io::Result<HttpResponse> {
    let mut stream = TcpStream::connect_timeout(
        &format!("127.0.0.1:{port}")
            .parse()
            .expect("valid local address"),
        Duration::from_millis(600),
    )?;
    stream.set_read_timeout(Some(Duration::from_secs(2)))?;
    let mut request = format!(
        "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\nContent-Length: 0\r\n"
    );
    for header in headers {
        request.push_str(header);
        request.push_str("\r\n");
    }
    request.push_str("\r\n");
    stream.write_all(request.as_bytes())?;

    let mut response = String::new();
    stream.read_to_string(&mut response)?;
    let status = response
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|value| value.parse::<u16>().ok())
        .ok_or_else(|| std::io::Error::other("invalid HTTP response"))?;
    let body = response
        .split_once("\r\n\r\n")
        .map(|(_, body)| body.to_string())
        .unwrap_or_default();
    Ok(HttpResponse { status, body })
}

pub(super) fn request_shutdown(port: u16, token: &str) -> std::io::Result<()> {
    let header = format!("X-MadLibrary-Control-Token: {token}");
    let response = request(port, "POST", "/api/v1/server/desktop/shutdown", &[header])?;
    if (200..300).contains(&response.status) {
        Ok(())
    } else {
        Err(std::io::Error::other("server rejected shutdown"))
    }
}

pub(super) fn server_info(port: u16) -> Option<ServerInfoResponse> {
    let response = request(port, "GET", "/api/v1/server/info", &[]).ok()?;
    if response.status != 200 {
        return None;
    }
    serde_json::from_str(&response.body).ok()
}
