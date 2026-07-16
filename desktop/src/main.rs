#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod managed_service;
mod tray;

use managed_service::{ManagedService, ServiceStatus};
use std::{env, fs, path::PathBuf, sync::Arc};
use tauri::Manager;
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            get_service_status,
            start_service,
            stop_service,
            restart_service,
            change_service_port,
            change_log_directory,
            select_log_directory,
            set_launch_at_login,
            open_admin,
            open_log,
            set_control_center_language,
        ])
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            write_control_center_log("single-instance activation");
            tray::show_main_window(app);
        }))
        .on_menu_event(|app, event| match event.id().as_ref() {
            tray::MENU_SHOW => tray::show_main_window(app),
            tray::MENU_OPEN_ADMIN => {
                if let Some(service) = app.try_state::<Arc<ManagedService>>() {
                    if service.status().state == "running" {
                        let _ = app
                            .opener()
                            .open_url(service.status().admin_url, None::<&str>);
                    } else {
                        tray::show_main_window(app);
                    }
                }
            }
            tray::MENU_TOGGLE_SERVICE => toggle_service_from_tray(app),
            tray::MENU_EXIT => app.exit(0),
            _ => {}
        })
        .setup(|app| {
            let resources = runtime_resources(app.handle())?;
            let data_home = machine_data_home()?;
            fs::create_dir_all(data_home.join("logs"))?;
            let service =
                Arc::new(ManagedService::new(resources, data_home).map_err(std::io::Error::other)?);
            app.manage(service.clone());
            write_control_center_log("control center setup completed");
            let status = service.status();
            tray::install(app.handle(), &status)?;
            if status.state == "stopped" && service.reserve_automatic_start() {
                schedule_automatic_service_start(
                    app.handle(),
                    service.clone(),
                    "automatic service start",
                );
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() == "main" {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    write_control_center_log("main window close requested; hiding to tray");
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("failed to run Mad Library Server control center");
}

#[tauri::command]
fn get_service_status(
    app: tauri::AppHandle,
    service: tauri::State<'_, Arc<ManagedService>>,
) -> ServiceStatus {
    let status = service.status();
    tray::sync_service_action(&app, &status);
    if status.state == "stopped" && service.reserve_automatic_start() {
        schedule_automatic_service_start(
            &app,
            service.inner().clone(),
            "automatic service restart",
        );
    }
    status
}

#[tauri::command]
async fn start_service(
    app: tauri::AppHandle,
    service: tauri::State<'_, Arc<ManagedService>>,
) -> Result<ServiceStatus, String> {
    run_service_task(app, service.inner().clone(), |service| service.start()).await
}

#[tauri::command]
async fn stop_service(
    app: tauri::AppHandle,
    service: tauri::State<'_, Arc<ManagedService>>,
) -> Result<ServiceStatus, String> {
    run_service_task(app, service.inner().clone(), |service| service.stop()).await
}

#[tauri::command]
async fn restart_service(
    app: tauri::AppHandle,
    service: tauri::State<'_, Arc<ManagedService>>,
) -> Result<ServiceStatus, String> {
    run_service_task(app, service.inner().clone(), |service| service.restart()).await
}

#[tauri::command]
fn change_service_port(
    port: u16,
    service: tauri::State<'_, Arc<ManagedService>>,
) -> Result<ServiceStatus, String> {
    service.update_port(port)?;
    Ok(service.status())
}

#[tauri::command]
fn change_log_directory(
    directory: String,
    service: tauri::State<'_, Arc<ManagedService>>,
) -> Result<ServiceStatus, String> {
    service.update_log_directory(PathBuf::from(directory))?;
    Ok(service.status())
}

#[tauri::command]
fn select_log_directory(
    app: tauri::AppHandle,
    service: tauri::State<'_, Arc<ManagedService>>,
) -> Result<Option<ServiceStatus>, String> {
    if service.status().state == "running" {
        return Err("请先停止服务，再修改日志目录。".to_string());
    }
    let Some(directory) = app.dialog().file().blocking_pick_folder() else {
        return Ok(None);
    };
    service.update_log_directory(directory.into_path().map_err(|error| error.to_string())?)?;
    Ok(Some(service.status()))
}

#[tauri::command]
fn set_launch_at_login(
    enabled: bool,
    service: tauri::State<'_, Arc<ManagedService>>,
) -> Result<ServiceStatus, String> {
    service.update_launch_at_login(enabled)?;
    Ok(service.status())
}

#[tauri::command]
fn open_admin(
    app: tauri::AppHandle,
    service: tauri::State<'_, Arc<ManagedService>>,
) -> Result<(), String> {
    let status = service.status();
    if status.state != "running" || !status.managed {
        return Err("服务尚未运行。".to_string());
    }
    app.opener()
        .open_url(status.admin_url, None::<&str>)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn open_log(
    app: tauri::AppHandle,
    service: tauri::State<'_, Arc<ManagedService>>,
) -> Result<(), String> {
    let target = service.log_path();
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&target)
        .map_err(|error| error.to_string())?;
    app.opener()
        .open_path(target.display().to_string(), None::<&str>)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn set_control_center_language(
    language: String,
    app: tauri::AppHandle,
    service: tauri::State<'_, Arc<ManagedService>>,
) -> Result<(), String> {
    tray::set_language(&app, &language, &service.status())
}

async fn run_service_task<F>(
    app: tauri::AppHandle,
    service: Arc<ManagedService>,
    action: F,
) -> Result<ServiceStatus, String>
where
    F: FnOnce(&ManagedService) -> Result<ServiceStatus, String> + Send + 'static,
{
    if !tray::begin_service_action(&app) {
        return Err("服务操作正在进行中。".to_string());
    }
    run_reserved_service_task(app, service, action).await
}

async fn run_reserved_service_task<F>(
    app: tauri::AppHandle,
    service: Arc<ManagedService>,
    action: F,
) -> Result<ServiceStatus, String>
where
    F: FnOnce(&ManagedService) -> Result<ServiceStatus, String> + Send + 'static,
{
    let worker = service.clone();
    let result = tauri::async_runtime::spawn_blocking(move || action(&worker)).await;
    match result {
        Ok(Ok(status)) => {
            tray::finish_service_action(&app, &status);
            Ok(status)
        }
        Ok(Err(error)) => {
            tray::finish_service_action(&app, &service.status());
            Err(error)
        }
        Err(error) => {
            tray::finish_service_action(&app, &service.status());
            Err(error.to_string())
        }
    }
}

fn toggle_service_from_tray(app: &tauri::AppHandle) {
    let Some(service) = app.try_state::<Arc<ManagedService>>() else {
        return;
    };
    let status = service.status();
    if status.state == "conflict" || !tray::begin_service_action(app) {
        tray::sync_service_action(app, &status);
        return;
    }

    let should_stop = status.state == "running" && status.managed;
    let app_handle = app.clone();
    let service = service.inner().clone();
    tauri::async_runtime::spawn(async move {
        let result = run_reserved_service_task(app_handle.clone(), service, move |service| {
            if should_stop {
                service.stop()
            } else {
                service.start()
            }
        })
        .await;
        match result {
            Ok(_) => write_control_center_log(if should_stop {
                "tray service stop completed"
            } else {
                "tray service start completed"
            }),
            Err(error) => {
                write_control_center_log(&format!("tray service action failed: {error}"));
                tray::show_main_window(&app_handle);
            }
        }
    });
}

fn schedule_automatic_service_start(
    app: &tauri::AppHandle,
    service: Arc<ManagedService>,
    reason: &'static str,
) {
    let app_handle = app.clone();
    std::thread::spawn(move || {
        if !tray::begin_service_action(&app_handle) {
            return;
        }
        match service.start() {
            Ok(status) => {
                write_control_center_log(&format!("{reason} completed"));
                tray::finish_service_action(&app_handle, &status);
            }
            Err(error) => {
                write_control_center_log(&format!("{reason} failed: {error}"));
                tray::finish_service_action(&app_handle, &service.status());
            }
        }
    });
}

fn runtime_resources(_app: &tauri::AppHandle) -> Result<PathBuf, Box<dyn std::error::Error>> {
    if let Ok(override_path) = env::var("MADLIBRARY_DESKTOP_RUNTIME") {
        return Ok(normalize_windows_path(PathBuf::from(override_path)));
    }
    #[cfg(debug_assertions)]
    {
        return Ok(PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("target")
            .join("desktop-runtime"));
    }
    #[cfg(not(debug_assertions))]
    Ok(normalize_windows_path(
        _app.path().resource_dir()?.join("runtime"),
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

fn write_control_center_log(message: &str) {
    let Ok(data_home) = machine_data_home() else {
        return;
    };
    let logs = managed_service::configured_log_directory(&data_home);
    let _ = fs::create_dir_all(&logs);
    let path = logs.join("control-center.log");
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|value| value.as_secs())
        .unwrap_or_default();
    use std::io::Write;
    if let Ok(mut file) = fs::OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(file, "{timestamp} {message}");
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    #[test]
    fn normalizes_extended_drive_path() {
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
