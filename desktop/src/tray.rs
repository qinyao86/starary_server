use crate::managed_service::ServiceStatus;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{
    menu::{MenuBuilder, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager,
};

pub const MENU_SHOW: &str = "show";
pub const MENU_OPEN_ADMIN: &str = "open-admin";
pub const MENU_TOGGLE_SERVICE: &str = "toggle-service";
pub const MENU_EXIT: &str = "exit";

struct TrayMenuState {
    show: MenuItem<tauri::Wry>,
    open_admin: MenuItem<tauri::Wry>,
    service_action: MenuItem<tauri::Wry>,
    exit: MenuItem<tauri::Wry>,
    service_busy: AtomicBool,
    english: AtomicBool,
}

pub fn install(app: &AppHandle, status: &ServiceStatus) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, MENU_SHOW, "打开", true, None::<&str>)?;
    let open_admin = MenuItem::with_id(app, MENU_OPEN_ADMIN, "后台管理", true, None::<&str>)?;
    let service_action = MenuItem::with_id(
        app,
        MENU_TOGGLE_SERVICE,
        service_action_label(status, false),
        status.state != "conflict",
        None::<&str>,
    )?;
    let exit = MenuItem::with_id(app, MENU_EXIT, "退出", true, None::<&str>)?;
    let menu = MenuBuilder::new(app)
        .items(&[&show, &open_admin])
        .separator()
        .item(&service_action)
        .separator()
        .item(&exit)
        .build()?;

    app.manage(TrayMenuState {
        show,
        open_admin,
        service_action,
        exit,
        service_busy: AtomicBool::new(false),
        english: AtomicBool::new(false),
    });

    let mut builder = TrayIconBuilder::with_id("main-tray")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .tooltip("Mad Library Server 控制中心");
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder
        .on_tray_icon_event(|tray, event| {
            if matches!(
                event,
                TrayIconEvent::DoubleClick {
                    button: MouseButton::Left,
                    ..
                } | TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                }
            ) {
                show_main_window(tray.app_handle());
            }
        })
        .build(app)?;
    Ok(())
}

pub fn begin_service_action(app: &AppHandle) -> bool {
    let Some(menu) = app.try_state::<TrayMenuState>() else {
        return false;
    };
    if menu
        .service_busy
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return false;
    }
    let _ = menu.service_action.set_enabled(false);
    true
}

pub fn finish_service_action(app: &AppHandle, status: &ServiceStatus) {
    if let Some(menu) = app.try_state::<TrayMenuState>() {
        menu.service_busy.store(false, Ordering::Release);
    }
    sync_service_action(app, status);
}

pub fn sync_service_action(app: &AppHandle, status: &ServiceStatus) {
    let Some(menu) = app.try_state::<TrayMenuState>() else {
        return;
    };
    let english = menu.english.load(Ordering::Acquire);
    let _ = menu
        .service_action
        .set_text(service_action_label(status, english));
    let busy = menu.service_busy.load(Ordering::Acquire);
    let _ = menu
        .service_action
        .set_enabled(!busy && status.state != "conflict");
}

pub fn set_language(app: &AppHandle, language: &str, status: &ServiceStatus) -> Result<(), String> {
    let english = match language {
        "zh" => false,
        "en" => true,
        _ => return Err("unsupported control center language".to_string()),
    };
    let Some(menu) = app.try_state::<TrayMenuState>() else {
        return Err("tray menu is not available".to_string());
    };
    menu.english.store(english, Ordering::Release);
    menu.show
        .set_text(if english { "Open" } else { "打开" })
        .map_err(|error| error.to_string())?;
    menu.open_admin
        .set_text(if english { "Admin" } else { "后台管理" })
        .map_err(|error| error.to_string())?;
    menu.exit
        .set_text(if english { "Exit" } else { "退出" })
        .map_err(|error| error.to_string())?;
    sync_service_action(app, status);
    if let Some(tray) = app.tray_by_id("main-tray") {
        tray.set_tooltip(Some(if english {
            "Mad Library Server Control Center"
        } else {
            "Mad Library Server 控制中心"
        }))
        .map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn service_action_label(status: &ServiceStatus, english: bool) -> &'static str {
    match (status.state == "running" && status.managed, english) {
        (true, true) => "Stop Service",
        (true, false) => "停止服务",
        (false, true) => "Start Service",
        (false, false) => "启动服务",
    }
}

pub fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}
