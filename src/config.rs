use clap::Parser;
use std::path::PathBuf;

#[derive(Clone, Debug, Parser)]
#[command(name = "madlibrary-server")]
pub struct ServerConfig {
    #[arg(long, env = "MADLIBRARY_SERVER_HOST", default_value = "127.0.0.1")]
    pub host: String,

    #[arg(long, env = "MADLIBRARY_SERVER_PORT", default_value_t = 3789)]
    pub port: u16,

    #[arg(long, env = "MADLIBRARY_DATABASE_URL")]
    pub database_url: String,

    #[arg(
        long,
        env = "MADLIBRARY_DATABASE_MAX_CONNECTIONS",
        default_value_t = 10
    )]
    pub database_max_connections: u32,

    #[arg(long, env = "MADLIBRARY_STORAGE_DIR")]
    pub storage_dir: PathBuf,

    #[arg(long, env = "MADLIBRARY_ADMIN_ASSETS_DIR")]
    pub admin_assets_dir: Option<PathBuf>,

    #[arg(long, env = "MADLIBRARY_DEPLOYMENT_MODE", default_value = "local")]
    pub deployment_mode: String,

    #[arg(long, env = "MADLIBRARY_JWT_SECRET")]
    pub jwt_secret: String,

    #[arg(long, env = "MADLIBRARY_TOKEN_TTL_HOURS", default_value_t = 168)]
    pub token_ttl_hours: i64,
}

impl ServerConfig {
    pub fn from_env() -> Self {
        Self::parse().normalize_paths()
    }

    pub fn bind_addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    pub fn resolved_admin_assets_dir(&self) -> PathBuf {
        self.admin_assets_dir.clone().unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("admin-ui")
                .join("dist")
        })
    }

    fn normalize_paths(mut self) -> Self {
        let server_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        if self.storage_dir.is_relative() {
            self.storage_dir = server_dir.join(&self.storage_dir);
        }
        if let Some(admin_assets_dir) = &self.admin_assets_dir {
            if admin_assets_dir.is_relative() {
                self.admin_assets_dir = Some(server_dir.join(admin_assets_dir));
            }
        }
        self
    }
}
