use clap::Parser;
use std::path::PathBuf;

#[derive(Clone, Debug, Parser)]
#[command(name = "starary-server")]
pub struct ServerConfig {
    #[arg(long, env = "STARARY_SERVER_HOST", default_value = "0.0.0.0")]
    pub host: String,

    #[arg(long, env = "STARARY_SERVER_PORT", default_value_t = 3789)]
    pub port: u16,

    #[arg(long, env = "STARARY_DATABASE_URL")]
    pub database_url: String,

    #[arg(long, env = "STARARY_DATABASE_MAX_CONNECTIONS", default_value_t = 10)]
    pub database_max_connections: u32,

    #[arg(long, env = "STARARY_STORAGE_DIR")]
    pub storage_dir: PathBuf,

    #[arg(
        long,
        env = "STARARY_ALLOW_PERSONAL_STORAGE_PATHS",
        default_value_t = false
    )]
    pub allow_personal_storage_paths: bool,

    #[arg(long, env = "STARARY_ADMIN_ASSETS_DIR")]
    pub admin_assets_dir: Option<PathBuf>,

    #[arg(long, env = "STARARY_DEPLOYMENT_MODE", default_value = "local")]
    pub deployment_mode: String,

    #[arg(long, env = "STARARY_JWT_SECRET")]
    pub jwt_secret: String,

    #[arg(long, env = "STARARY_DESKTOP_CONTROL_TOKEN")]
    pub desktop_control_token: Option<String>,

    #[arg(long, env = "STARARY_DESKTOP_INSTANCE_ID")]
    pub desktop_instance_id: Option<String>,

    #[arg(long, env = "STARARY_TOKEN_TTL_HOURS", default_value_t = 168)]
    pub token_ttl_hours: i64,
}

impl ServerConfig {
    pub fn from_env(app_home: &std::path::Path) -> Self {
        Self::parse().normalize_paths(app_home)
    }

    pub fn bind_addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    pub fn resolved_admin_assets_dir(&self) -> PathBuf {
        self.admin_assets_dir
            .clone()
            .expect("admin assets directory is normalized during startup")
    }

    fn normalize_paths(mut self, app_home: &std::path::Path) -> Self {
        if self.storage_dir.is_relative() {
            self.storage_dir = app_home.join(&self.storage_dir);
        }
        if let Some(admin_assets_dir) = &self.admin_assets_dir {
            if admin_assets_dir.is_relative() {
                self.admin_assets_dir = Some(app_home.join(admin_assets_dir));
            }
        } else {
            self.admin_assets_dir = Some(app_home.join("admin-ui"));
        }
        self
    }
}
