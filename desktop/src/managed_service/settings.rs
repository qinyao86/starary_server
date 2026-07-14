use serde::{Deserialize, Serialize};
use std::{
    fs,
    fs::OpenOptions,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct DesktopSettings {
    version: u32,
    pub(super) log_directory: PathBuf,
}

impl DesktopSettings {
    pub(super) fn load_or_create(data_home: &Path) -> Result<Self, String> {
        let path = Self::path(data_home);
        if path.is_file() {
            let bytes = fs::read(&path).map_err(|error| error.to_string())?;
            let mut settings: Self =
                serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
            if settings.version != 1 {
                return Err(format!("desktop settings file is invalid: {}", path.display()));
            }
            settings.log_directory =
                Self::normalize_log_directory(data_home, settings.log_directory)?;
            return Ok(settings);
        }

        let settings = Self {
            version: 1,
            log_directory: data_home.join("logs"),
        };
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
