use serde::{Deserialize, Serialize};
use std::{
    fs,
    fs::OpenOptions,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;
#[cfg(windows)]
const RUN_KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";
#[cfg(windows)]
const RUN_VALUE_NAME: &str = "Mad Library Server";

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct DesktopSettings {
    version: u32,
    pub(super) log_directory: PathBuf,
    #[serde(default = "default_launch_at_login")]
    pub(super) launch_at_login: bool,
}

impl DesktopSettings {
    pub(super) fn load_or_create(data_home: &Path) -> Result<Self, String> {
        let path = Self::path(data_home);
        if path.is_file() {
            let bytes = fs::read(&path).map_err(|error| error.to_string())?;
            let raw: serde_json::Value =
                serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
            let launch_at_login_missing = raw.get("launchAtLogin").is_none();
            let mut settings: Self =
                serde_json::from_value(raw).map_err(|error| error.to_string())?;
            if settings.version != 1 {
                return Err(format!("desktop settings file is invalid: {}", path.display()));
            }
            settings.log_directory =
                Self::normalize_log_directory(data_home, settings.log_directory)?;
            if launch_at_login_missing {
                settings.launch_at_login = true;
                apply_launch_at_login(true)?;
                settings.write(data_home)?;
            }
            return Ok(settings);
        }

        let settings = Self {
            version: 1,
            log_directory: data_home.join("logs"),
            launch_at_login: true,
        };
        apply_launch_at_login(true)?;
        settings.write(data_home)?;
        Ok(settings)
    }

    pub(super) fn configured_log_directory(data_home: &Path) -> PathBuf {
        Self::load_or_create(data_home)
            .map(|settings| settings.log_directory)
            .unwrap_or_else(|_| data_home.join("logs"))
    }

    pub(super) fn update_log_directory(
        &mut self,
        data_home: &Path,
        directory: PathBuf,
    ) -> Result<(), String> {
        self.log_directory = Self::normalize_log_directory(data_home, directory)?;
        self.write(data_home)
    }

    pub(super) fn update_launch_at_login(
        &mut self,
        data_home: &Path,
        enabled: bool,
    ) -> Result<(), String> {
        apply_launch_at_login(enabled)?;
        self.launch_at_login = enabled;
        self.write(data_home)
    }

    fn normalize_log_directory(data_home: &Path, directory: PathBuf) -> Result<PathBuf, String> {
        if directory.as_os_str().is_empty() {
            return Err("日志目录不能为空。".to_string());
        }

        let directory = if directory.is_absolute() {
            directory
        } else {
            data_home.join(directory)
        };
        fs::create_dir_all(&directory).map_err(|error| error.to_string())?;

        let probe = directory.join(".madlibrary-write-test");
        OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&probe)
            .map_err(|error| error.to_string())?;
        let _ = fs::remove_file(probe);
        Ok(directory)
    }

    fn path(data_home: &Path) -> PathBuf {
        data_home
            .join("data")
            .join("config")
            .join("desktop-settings.json")
    }

    fn write(&self, data_home: &Path) -> Result<(), String> {
        let path = Self::path(data_home);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        fs::write(
            path,
            serde_json::to_vec_pretty(self).map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string())
    }
}

#[cfg(windows)]
fn apply_launch_at_login(enabled: bool) -> Result<(), String> {
    let mut command = Command::new("reg");
    command.stdout(Stdio::null()).stderr(Stdio::null());
    command.creation_flags(CREATE_NO_WINDOW);
    if enabled {
        let executable = std::env::current_exe().map_err(|error| error.to_string())?;
        let executable = format!("\"{}\"", executable.display());
        command
            .args(["add", RUN_KEY, "/v", RUN_VALUE_NAME, "/t", "REG_SZ", "/d"])
            .arg(executable)
            .args(["/f"]);
        if command.status().map_err(|error| error.to_string())?.success() {
            Ok(())
        } else {
            Err("无法写入开机启动项。".to_string())
        }
    } else {
        command.args(["delete", RUN_KEY, "/v", RUN_VALUE_NAME, "/f"]);
        let _ = command.status().map_err(|error| error.to_string())?;
        Ok(())
    }
}

#[cfg(not(windows))]
fn apply_launch_at_login(_enabled: bool) -> Result<(), String> {
    Err("当前系统暂不支持开机启动设置。".to_string())
}

fn default_launch_at_login() -> bool {
    true
}
