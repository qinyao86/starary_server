mod auth;
mod backup;
mod config;
mod db;
mod error;
mod ids;
mod models;
mod path_resolver;
mod portable;
mod routes;
mod state;
mod system_avatars;

use anyhow::Context;
use config::ServerConfig;
use sqlx::postgres::PgPoolOptions;
use state::{AppState, ServiceControl};
use std::{net::SocketAddr, time::Duration};
use tokio::net::TcpListener;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "starary_server=info,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let mut portable_runtime = portable::PortableRuntime::prepare()?;
    let config = ServerConfig::from_env(&portable_runtime.app_home);
    std::fs::create_dir_all(&config.storage_dir).with_context(|| {
        format!(
            "failed to create storage directory {}",
            config.storage_dir.display()
        )
    })?;

    let backup_service = backup::BackupService::new(
        &portable_runtime.app_home,
        &config.storage_dir,
        &config.database_url,
    )?;
    if backup_service.apply_pending_restore()? {
        tracing::info!("Pending database restore completed");
    }

    let pool = PgPoolOptions::new()
        .max_connections(config.database_max_connections)
        .acquire_timeout(Duration::from_secs(5))
        .connect(&config.database_url)
        .await
        .context("failed to connect to PostgreSQL")?;

    db::run_migrations(&pool).await?;

    let bind_addr = config.bind_addr();
    let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
    let scheduler_stop_tx = shutdown_tx.clone();
    let service_control = ServiceControl::new(
        shutdown_tx,
        portable_runtime.runtime_config_path(),
        config.port,
        portable_runtime.server_port_managed(),
    );
    let state = AppState::new(config, pool, service_control, backup_service.clone());
    let app = routes::router(state)
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive());

    let listener = TcpListener::bind(&bind_addr)
        .await
        .with_context(|| format!("failed to bind {bind_addr}"))?;

    tracing::info!("Starary Server listening on http://{bind_addr}");

    let scheduler_shutdown = shutdown_rx.clone();
    let scheduler = tokio::spawn(async move {
        backup_service.run_scheduler(scheduler_shutdown).await;
    });

    let server_result = axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal(shutdown_rx))
    .await;

    let _ = scheduler_stop_tx.send(true);
    let _ = scheduler.await;

    portable_runtime.stop();
    server_result?;

    Ok(())
}

async fn shutdown_signal(mut service_shutdown: tokio::sync::watch::Receiver<bool>) {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    let managed_shutdown = async move {
        while !*service_shutdown.borrow() {
            if service_shutdown.changed().await.is_err() {
                return;
            }
        }
    };

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
        _ = managed_shutdown => {},
    }
}
