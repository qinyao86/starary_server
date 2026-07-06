use crate::config::ServerConfig;
use sqlx::PgPool;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<ServerConfig>,
    pub pool: PgPool,
}

impl AppState {
    pub fn new(config: ServerConfig, pool: PgPool) -> Self {
        Self {
            config: Arc::new(config),
            pool,
        }
    }
}
