#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{
    env,
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};
use tauri::Manager;
use url::Url;
use uuid::Uuid;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

const DEFAULT_SERVER_PORT: u16 = 3789;

#[derive(Clone)]
struct ServerProcess {
    child: Arc<Mutex<Option<Child>>>,
    port: u16,
    control_token: String,
}

impl ServerProcess {
    fn stop(&self) {
        request_server_shutdown(self.port, &self.control_token);
        let deadline = Instant::now() + Duration::from_secs(12);
        while Instant::now() < deadline {
            let exited = self
                .child
                .lock()
                .ok()
                .and_then(|mut child| child.as_mut().and_then(|child| child.try_wait().ok()))
                .flatten()
                .is_some();
            if exited {
                return;
            }
            thread::sleep(Duration::from_millis(150));
        }
        if let Ok(mut child) = self.child.lock() {
            if let Some(child) = child.as_mut() {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
    }
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }))
        .setup(|app| {
            let resources = runtime_resources(app.handle())?;
            let data_home = machine_data_home()?;
            fs::create_dir_all(data_home.join("logs"))?;
            let port = configured_server_port(&data_home).unwrap_or(DEFAULT_SERVER_PORT);
            let control_token = Uuid::new_v4().simple().to_string();
            let child = spawn_server(&resources, &data_home, &control_token)?;
            let process = ServerProcess {
                child: Arc::new(Mutex::new(Some(child))),
                port,
                control_token,
            };
            app.manage(process.clone());

            let app_handle = app.handle().clone();
            thread::spawn(move || {
                if wait_for_server(port, &process.child, Duration::from_secs(45)) {
                    if let Some(window) = app_handle.get_webview_window("main") {
                        if let Ok(url) = Url::parse(&format!("http://127.0.0.1:{port}/admin/")) {
                            let _ = window.navigate(url);
                        }
                    }
                    wait_for_process_exit(&process.child);
                    app_handle.exit(0);
                } else {
                    show_startup_error(&app_handle, &data_home);
                }
            });
            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() == "main"
                && matches!(event, tauri::WindowEvent::CloseRequested { .. })
            {
                if let Some(process) = window.app_handle().try_state::<ServerProcess>() {
                    process.stop();
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("failed to run Mad Library Server desktop shell");
}

fn wait_for_process_exit(child: &Arc<Mutex<Option<Child>>>) {
    loop {
        let exited = child
            .lock()
            .ok()
            .and_then(|mut child| child.as_mut().and_then(|child| child.try_wait().ok()))
            .flatten()
            .is_some();
        if exited {
            return;
        }
        thread::sleep(Duration::from_millis(250));
    }
}

fn runtime_resources(app: &tauri::AppHandle) -> Result<PathBuf, Box<dyn std::error::Error>> {
    if let Ok(override_path) = env::var("MADLIBRARY_DESKTOP_RUNTIME") {
        return Ok(normalize_windows_path(PathBuf::from(override_path)));
    }
    Ok(normalize_windows_path(
        app.path().resource_dir()?.join("runtime"),
    ))
}

fn normalize_windows_path(path: PathBuf) -> PathBuf {
    #[cfg(windows)]
    {
        let value = path.to_string_lossy();
        if let Some(value) = value.strip_prefix(r"\\?\UNC\") {
            return PathBuf::from(format!(r"\\{value}"));
        }
        if let Some(value) = value.strip_prefix(r"\\?\") {
            return PathBuf::from(value);
        }
    }
    path
}

fn machine_data_home() -> Result<PathBuf, Box<dyn std::error::Error>> {
    if let Ok(override_path) = env::var("MADLIBRARY_HOME") {
        return Ok(PathBuf::from(override_path));
    }
    #[cfg(windows)]
    {
        let base = env::var_os("PROGRAMDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(r"C:\ProgramData"));
        return Ok(base.join("Mad Library Server"));
    }
    #[allow(unreachable_code)]
    Ok(env::current_dir()?.join(".madlibrary-server"))
}

fn spawn_server(
    resources: &Path,
    data_home: &Path,
    control_token: &str,
) -> Result<Child, Box<dyn std::error::Error>> {
    let server = resources.join("madlibrary-server.exe");
    let postgres = resources.join("postgresql");
    let admin_ui = resources.join("admin-ui");
    let log_path = data_home.join("logs").join("server.log");
    let stdout = File::options().create(true).append(true).open(&log_path)?;
    let stderr = stdout.try_clone()?;

    let mut command = Command::new(server);
    command
        .env("MADLIBRARY_HOME", data_home)
        .env("MADLIBRARY_POSTGRES_HOME", postgres)
        .env(
            "MADLIBRARY_POSTGRES_BIN_DIR",
            resources.join("postgresql").join("bin"),
        )
        .env("MADLIBRARY_ADMIN_ASSETS_DIR", admin_ui)
        .env("MADLIBRARY_DESKTOP_CONTROL_TOKEN", control_token)
        .stdin(Stdio::null())
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr));
    #[cfg(windows)]
    command.creation_flags(CREATE_NO_WINDOW);
    Ok(command.spawn()?)
}

fn configured_server_port(data_home: &Path) -> Option<u16> {
    let bytes = fs::read(data_home.join("data").join("config").join("runtime.json")).ok()?;
    let value: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
    value
        .get("serverPort")?
        .as_u64()
        .and_then(|port| u16::try_from(port).ok())
}

fn wait_for_server(port: u16, child: &Arc<Mutex<Option<Child>>>, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if http_request(port, "GET", "/health", &[]).is_ok() {
            return true;
        }
        if child
            .lock()
            .ok()
            .and_then(|mut child| child.as_mut().and_then(|child| child.try_wait().ok()))
            .flatten()
            .is_some()
        {
            return false;
        }
        thread::sleep(Duration::from_millis(250));
    }
    false
}

fn request_server_shutdown(port: u16, token: &str) {
    let header = format!("X-MadLibrary-Control-Token: {token}");
    let _ = http_request(port, "POST", "/api/v1/server/desktop/shutdown", &[header]);
}

fn http_request(port: u16, method: &str, path: &str, headers: &[String]) -> std::io::Result<()> {
    use std::io::Read;
    use std::net::TcpStream;
    let mut stream = TcpStream::connect_timeout(
        &format!("127.0.0.1:{port}")
            .parse()
            .expect("valid local address"),
        Duration::from_millis(500),
    )?;
    stream.set_read_timeout(Some(Duration::from_secs(2)))?;
    let mut request = format!("{method} {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\nContent-Length: 0\r\n");
    for header in headers {
        request.push_str(header);
        request.push_str("\r\n");
    }
    request.push_str("\r\n");
    stream.write_all(request.as_bytes())?;
    let mut response = String::new();
    stream.read_to_string(&mut response)?;
    if response.starts_with("HTTP/1.1 2") || response.starts_with("HTTP/1.0 2") {
        Ok(())
    } else {
        Err(std::io::Error::other("server returned an error"))
    }
}

fn show_startup_error(app: &tauri::AppHandle, data_home: &Path) {
    if let Some(window) = app.get_webview_window("main") {
        let log = data_home.join("logs").join("server.log");
        let escaped = log
            .display()
            .to_string()
            .replace('\\', "\\\\")
            .replace('`', "\\`");
        /*
            "document.querySelector('h1').textContent='服务启动失败';document.querySelector('p').textContent=`请检查日志：{escaped}`;document.querySelector('.progress').style.display='none';"
        );
        */
        let script = format!(
            "document.querySelector('h1').textContent='\u{670d}\u{52a1}\u{542f}\u{52a8}\u{5931}\u{8d25}';document.querySelector('p').textContent=`\u{8bf7}\u{68c0}\u{67e5}\u{65e5}\u{5fd7}\u{ff1a}{escaped}`;document.querySelector('.progress').style.display='none';"
        );
        let _ = window.eval(&script);
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    #[test]
    fn normalizes_extended_drive_path_for_postgres() {
        assert_eq!(
            normalize_windows_path(PathBuf::from(
                r"\\?\C:\Program Files\Mad Library Server\runtime"
            )),
            PathBuf::from(r"C:\Program Files\Mad Library Server\runtime")
        );
    }

    #[test]
    fn normalizes_extended_unc_path() {
        assert_eq!(
            normalize_windows_path(PathBuf::from(r"\\?\UNC\server\share\runtime")),
            PathBuf::from(r"\\server\share\runtime")
        );
    }
}
