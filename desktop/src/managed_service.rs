mod http;
mod identity;
mod runtime_config;
mod settings;

use http::{IdentityResponse, ServerInfoResponse};
use identity::ControlIdentity;
use runtime_config::{
    configured_server_port, is_port_available, update_server_port, DEFAULT_SERVER_PORT,
};
use serde::Serialize;
use settings::DesktopSettings;
use std::{
    fs::{self, File},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::Mutex,
    thread,
    time::{Duration, Instant},
};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceStatus {
    pub state: &'static str,
    pub managed: bool,
    pub port: u16,
    pub process_id: Option<u32>,
    pub local_url: String,
    pub admin_url: String,
    pub lan_url: Option<String>,
    pub database_status: String,
    pub storage_status: String,
    pub message: Option<String>,
    pub log_directory: String,
    pub data_directory: String,
}

pub struct ManagedService {
    resources: PathBuf,
    data_home: PathBuf,
    identity: ControlIdentity,
    settings: Mutex<DesktopSettings>,
    child: Mutex<Option<Child>>,
    cached_server_info: Mutex<Option<(u32, ServerInfoResponse)>>,
}

impl ManagedService {
    pub fn new(resources: PathBuf, data_home: PathBuf) -> Result<Self, String> {
        let identity = ControlIdentity::load_or_create(&data_home)?;
        let settings = DesktopSettings::load_or_create(&data_home)?;
        Ok(Self {
            resources,
            data_home,
            identity,
            settings: Mutex::new(settings),
            child: Mutex::new(None),
            cached_server_info: Mutex::new(None),
        })
    }

    pub fn data_home(&self) -> &Path {
        &self.data_home
    }

    pub fn log_directory(&self) -> PathBuf {
        self.settings
            .lock()
            .map(|settings| settings.log_directory.clone())
            .unwrap_or_else(|_| self.data_home.join("logs"))
    }

    pub fn log_path(&self) -> PathBuf {
        self.log_directory().join("server.log")
    }

    pub fn configured_port(&self) -> u16 {
        configured_server_port(&self.data_home).unwrap_or(DEFAULT_SERVER_PORT)
    }

    pub fn should_start_automatically(&self) -> bool {
        self.identity.desired_running
    }

    pub fn status(&self) -> ServiceStatus {
        self.reap_child();
        let port = self.configured_port();
        match self.probe_identity(port) {
            Ok(identity) => {
                let server_info = self.server_info(&identity);
                let (log_directory, data_directory) = self.status_paths();
                ServiceStatus {
                    state: "running",
                    managed: true,
                    port: identity.port,
                    process_id: Some(identity.process_id),
                    local_url: format!("http://127.0.0.1:{}", identity.port),
                    admin_url: format!("http://127.0.0.1:{}/admin/", identity.port),
                    lan_url: server_info.as_ref().and_then(|info| info.lan_url.clone()),
                    database_status: server_info
                        .as_ref()
                        .map(|_| "connected".to_string())
                        .unwrap_or_else(|| "unknown".to_string()),
                    storage_status: server_info
                        .as_ref()
                        .map(|info| info.storage_status.as_str())
                        .filter(|value| matches!(*value, "writable" | "read_only" | "missing"))
                        .unwrap_or("unknown")
                        .to_string(),
                    message: None,
                    log_directory,
                    data_directory,
                }
            }
            Err(ProbeError::Unavailable) if is_port_available(port) => self.stopped_status(port),
            Err(ProbeError::Unavailable) => {
                self.foreign_status(port, "该端口已被其他程序占用，控制中心不会接管或停止它。")
            }
            Err(ProbeError::Foreign) => self.foreign_status(
                port,
                "检测到另一套或非受管的 Mad Library 服务，控制中心不会接管它。",
            ),
        }
    }

    pub fn start(&self) -> Result<ServiceStatus, String> {
        self.set_desired_running(true)?;
        let status = self.status();
        if status.state == "running" {
            return Ok(status);
        }
        if status.state == "conflict" {
            return Err(status.message.unwrap_or_else(|| "服务端口冲突".to_string()));
        }

        fs::create_dir_all(self.data_home.join("logs")).map_err(|error| error.to_string())?;
        let mut child = self.spawn_server()?;
        let deadline = Instant::now() + Duration::from_secs(45);
        while Instant::now() < deadline {
            if self.probe_identity(self.configured_port()).is_ok() {
                if let Ok(mut slot) = self.child.lock() {
                    *slot = Some(child);
                }
                return Ok(self.status());
            }
            if child
                .try_wait()
                .map_err(|error| error.to_string())?
                .is_some()
            {
                return Err(format!(
                    "服务启动失败，请检查日志：{}",
                    self.log_path().display()
                ));
            }
            thread::sleep(Duration::from_millis(250));
        }

        let _ = child.kill();
        let _ = child.wait();
        Err(format!(
            "服务启动超时，请检查日志：{}",
            self.log_path().display()
        ))
    }

    pub fn stop(&self) -> Result<ServiceStatus, String> {
        self.set_desired_running(false)?;
        let status = self.status();
        if status.state == "stopped" {
            return Ok(status);
        }
        if status.state != "running" || !status.managed {
            return Err("当前端口上的进程不属于此控制中心，已拒绝停止。".to_string());
        }

        http::request_shutdown(self.configured_port(), &self.identity.control_token)
            .map_err(|_| "服务拒绝了停止请求。".to_string())?;
        let deadline = Instant::now() + Duration::from_secs(15);
        while Instant::now() < deadline {
            let http_stopped = matches!(
                self.probe_identity(self.configured_port()),
                Err(ProbeError::Unavailable)
            );
            if http_stopped && !self.bundled_postgres_running() {
                self.reap_child();
                return Ok(self.status());
            }
            thread::sleep(Duration::from_millis(200));
        }
        Err("服务未能在预期时间内停止，请检查服务日志。".to_string())
    }

    pub fn restart(&self) -> Result<ServiceStatus, String> {
        if self.status().state == "running" {
            self.stop()?;
        }
        self.start()
    }

    pub fn update_port(&self, port: u16) -> Result<(), String> {
        if !(1024..=65535).contains(&port) {
            return Err("端口必须在 1024 到 65535 之间。".to_string());
        }
        let current = self.status();
        if current.state == "running" {
            return Err("请先停止服务，再修改端口。".to_string());
        }
        if !is_port_available(port) {
            return Err(format!("端口 {port} 已被占用。"));
        }
        update_server_port(&self.data_home, port).map_err(|error| error.to_string())
    }

    pub fn update_log_directory(&self, directory: PathBuf) -> Result<(), String> {
        if self.status().state == "running" {
            return Err("请先停止服务，再修改日志目录。".to_string());
        }
        self.settings
            .lock()
            .map_err(|error| error.to_string())?
            .update_log_directory(&self.data_home, directory)
    }

    fn spawn_server(&self) -> Result<Child, String> {
        let server = self.resources.join("madlibrary-server.exe");
        if !server.is_file() {
            return Err(format!("服务程序不存在：{}", server.display()));
        }
        let postgres = self.resources.join("postgresql");
        let admin_ui = self.resources.join("admin-ui");
        let stdout = File::options()
            .create(true)
            .append(true)
            .open(self.log_path())
            .map_err(|error| error.to_string())?;
        let stderr = stdout.try_clone().map_err(|error| error.to_string())?;

        let mut command = Command::new(server);
        command
            .env("MADLIBRARY_HOME", &self.data_home)
            .env("MADLIBRARY_POSTGRES_HOME", &postgres)
            .env("MADLIBRARY_POSTGRES_BIN_DIR", postgres.join("bin"))
            .env("MADLIBRARY_ADMIN_ASSETS_DIR", admin_ui)
            .env(
                "MADLIBRARY_DESKTOP_CONTROL_TOKEN",
                &self.identity.control_token,
            )
            .env("MADLIBRARY_DESKTOP_INSTANCE_ID", &self.identity.instance_id)
            .stdin(Stdio::null())
            .stdout(Stdio::from(stdout))
            .stderr(Stdio::from(stderr));
        #[cfg(windows)]
        command.creation_flags(CREATE_NO_WINDOW);
        command.spawn().map_err(|error| error.to_string())
    }

    fn probe_identity(&self, port: u16) -> Result<IdentityResponse, ProbeError> {
        let header = format!(
            "X-MadLibrary-Control-Token: {}",
            self.identity.control_token
        );
        let response = http::request(port, "GET", "/api/v1/server/desktop/identity", &[header])
            .map_err(|_| ProbeError::Unavailable)?;
        if response.status != 200 {
            return Err(ProbeError::Foreign);
        }
        let identity: IdentityResponse =
            serde_json::from_str(&response.body).map_err(|_| ProbeError::Foreign)?;
        if identity.product != "Mad Library Team Server"
            || identity.instance_id != self.identity.instance_id
            || identity.port != port
        {
            return Err(ProbeError::Foreign);
        }
        Ok(identity)
    }

    fn stopped_status(&self, port: u16) -> ServiceStatus {
        let (log_directory, data_directory) = self.status_paths();
        ServiceStatus {
            state: "stopped",
            managed: true,
            port,
            process_id: None,
            local_url: format!("http://127.0.0.1:{port}"),
            admin_url: format!("http://127.0.0.1:{port}/admin/"),
            lan_url: None,
            database_status: "stopped".to_string(),
            storage_status: "unknown".to_string(),
            message: None,
            log_directory,
            data_directory,
        }
    }

    fn foreign_status(&self, port: u16, message: &str) -> ServiceStatus {
        let (log_directory, data_directory) = self.status_paths();
        ServiceStatus {
            state: "conflict",
            managed: false,
            port,
            process_id: None,
            local_url: format!("http://127.0.0.1:{port}"),
            admin_url: format!("http://127.0.0.1:{port}/admin/"),
            lan_url: None,
            database_status: "unknown".to_string(),
            storage_status: "unknown".to_string(),
            message: Some(message.to_string()),
            log_directory,
            data_directory,
        }
    }

    fn status_paths(&self) -> (String, String) {
        (
            self.log_directory().display().to_string(),
            self.data_home.display().to_string(),
        )
    }

    fn reap_child(&self) {
        if let Ok(mut child) = self.child.lock() {
            let exited = child
                .as_mut()
                .and_then(|process| process.try_wait().ok())
                .flatten()
                .is_some();
            if exited {
                *child = None;
            }
        }
    }

    fn bundled_postgres_running(&self) -> bool {
        let pg_ctl = self
            .resources
            .join("postgresql")
            .join("bin")
            .join("pg_ctl.exe");
        let data_dir = self.data_home.join("data").join("postgresql");
        if !pg_ctl.is_file() || !data_dir.join("PG_VERSION").is_file() {
            return false;
        }
        let mut command = Command::new(pg_ctl);
        command.arg("status").arg("-D").arg(data_dir);
        #[cfg(windows)]
        command.creation_flags(CREATE_NO_WINDOW);
        command
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|status| status.success())
    }

    fn server_info(&self, identity: &IdentityResponse) -> Option<ServerInfoResponse> {
        if let Ok(cache) = self.cached_server_info.lock() {
            if let Some((process_id, info)) = cache.as_ref() {
                if *process_id == identity.process_id {
                    return Some(info.clone());
                }
            }
        }

        let info = http::server_info(identity.port)?;
        if let Ok(mut cache) = self.cached_server_info.lock() {
            *cache = Some((identity.process_id, info.clone()));
        }
        Some(info)
    }

    fn set_desired_running(&self, desired_running: bool) -> Result<(), String> {
        self.identity
            .persist_desired_running(&self.data_home, desired_running)
    }
}

#[derive(Debug)]
enum ProbeError {
    Unavailable,
    Foreign,
}

pub fn configured_log_directory(data_home: &Path) -> PathBuf {
    DesktopSettings::configured_log_directory(data_home)
}
