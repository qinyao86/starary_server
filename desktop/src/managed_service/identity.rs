use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ControlIdentity {
    pub(super) version: u32,
    pub(super) instance_id: String,
    pub(super) control_token: String,
    #[serde(default = "default_desired_running")]
    pub(super) desired_running: bool,
}

impl ControlIdentity {
    pub(super) fn load_or_create(data_home: &Path) -> Result<Self, String> {
        let config_dir = data_home.join("data").join("config");
        fs::create_dir_all(&config_dir).map_err(|error| error.to_string())?;
        let path = Self::path(data_home);
        if path.is_file() {
            let bytes = fs::read(&path).map_err(|error| error.to_string())?;
            let identity: Self =
                serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
            if identity.version == 1
                && !identity.instance_id.trim().is_empty()
                && identity.control_token.len() >= 32
            {
                return Ok(identity);
            }
            return Err(format!("控制身份文件无效：{}", path.display()));
        }

        let identity = Self {
            version: 1,
            instance_id: Uuid::new_v4().to_string(),
            control_token: format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple()),
            desired_running: true,
        };
        identity.write(&path)?;
        Ok(identity)
    }

    pub(super) fn persist_desired_running(
        &self,
        data_home: &Path,
        desired_running: bool,
    ) -> Result<(), String> {
        let mut identity = self.clone();
        identity.desired_running = desired_running;
        identity.write(&Self::path(data_home))
    }

    fn path(data_home: &Path) -> PathBuf {
        data_home
            .join("data")
            .join("config")
            .join("desktop-control.json")
    }

    fn write(&self, path: &Path) -> Result<(), String> {
        fs::write(
            path,
            serde_json::to_vec_pretty(self).map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string())
    }
}

fn default_desired_running() -> bool {
    true
}
