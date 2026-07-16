use crate::{backup::BackupService, config::ServerConfig};
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{
        atomic::{AtomicU16, Ordering},
        Arc,
    },
};
use tokio::sync::watch;
use uuid::Uuid;

#[derive(Clone, Default)]
pub struct BrowserHandoffStore {
    codes: Arc<std::sync::Mutex<HashMap<String, BrowserHandoff>>>,
}

#[derive(Clone)]
pub struct BrowserHandoff {
    pub user_id: Uuid,
    pub expires_at: DateTime<Utc>,
}

impl BrowserHandoffStore {
    pub fn issue(&self, code: String, handoff: BrowserHandoff) {
        if let Ok(mut codes) = self.codes.lock() {
            let now = Utc::now();
            codes.retain(|_, value| value.expires_at > now);
            codes.insert(code, handoff);
        }
    }

    pub fn redeem(&self, code: &str) -> Option<BrowserHandoff> {
        let mut codes = self.codes.lock().ok()?;
        let now = Utc::now();
        codes.retain(|_, value| value.expires_at > now);
        let handoff = codes.remove(code)?;
        (handoff.expires_at > now).then_some(handoff)
    }

    pub fn revoke_user(&self, user_id: Uuid) {
        if let Ok(mut codes) = self.codes.lock() {
            codes.retain(|_, value| value.user_id != user_id);
        }
    }
}

#[derive(Clone)]
pub struct ServiceControl {
    shutdown_tx: watch::Sender<bool>,
    runtime_config_path: Option<PathBuf>,
    configured_port: Arc<AtomicU16>,
    available: bool,
}

impl ServiceControl {
    pub fn new(
        shutdown_tx: watch::Sender<bool>,
        runtime_config_path: Option<PathBuf>,
        current_port: u16,
        available: bool,
    ) -> Self {
        Self {
            shutdown_tx,
            runtime_config_path,
            configured_port: Arc::new(AtomicU16::new(current_port)),
            available,
        }
    }

    pub fn available(&self) -> bool {
        self.available
    }

    pub fn configured_port(&self) -> u16 {
        self.configured_port.load(Ordering::Relaxed)
    }

    pub fn update_port(&self, port: u16) -> anyhow::Result<()> {
        let path = self.runtime_config_path.as_deref().ok_or_else(|| {
            anyhow::anyhow!("server port is managed by the deployment environment")
        })?;
        crate::portable::update_server_port(path, port)?;
        self.configured_port.store(port, Ordering::Relaxed);
        Ok(())
    }

    pub fn request_shutdown(&self) -> anyhow::Result<()> {
        self.shutdown_tx
            .send(true)
            .map_err(|_| anyhow::anyhow!("service shutdown channel is unavailable"))
    }
}

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<ServerConfig>,
    pub pool: PgPool,
    pub service_control: ServiceControl,
    pub backup_service: BackupService,
    pub browser_handoffs: BrowserHandoffStore,
}

impl AppState {
    pub fn new(
        config: ServerConfig,
        pool: PgPool,
        service_control: ServiceControl,
        backup_service: BackupService,
    ) -> Self {
        Self {
            config: Arc::new(config),
            pool,
            service_control,
            backup_service,
            browser_handoffs: BrowserHandoffStore::default(),
        }
    }
}
