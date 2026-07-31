use super::MigrationTx;
use sqlx::Executor;

pub(super) async fn create_task_schema(tx: &mut MigrationTx<'_>) -> anyhow::Result<()> {
    tx.execute(
        r#"
        CREATE TABLE IF NOT EXISTS server_tasks (
            id TEXT PRIMARY KEY,
            library_id TEXT REFERENCES libraries(id) ON DELETE SET NULL,
            user_id UUID REFERENCES users(id) ON DELETE SET NULL,
            client_id TEXT NOT NULL DEFAULT '',
            job_type TEXT NOT NULL,
            title TEXT NOT NULL,
            status TEXT NOT NULL,
            total_count BIGINT NOT NULL DEFAULT 0,
            processed_count BIGINT NOT NULL DEFAULT 0,
            failed_count BIGINT NOT NULL DEFAULT 0,
            progress INTEGER NOT NULL DEFAULT 0,
            message TEXT,
            delete_requested_at TIMESTAMPTZ,
            deleted_at TIMESTAMPTZ,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            last_heartbeat_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );
        "#,
    )
    .await?;

    tx.execute(
        r#"
        CREATE INDEX IF NOT EXISTS idx_server_tasks_updated_at
        ON server_tasks (updated_at DESC);
        "#,
    )
    .await?;

    tx.execute(
        r#"
        CREATE INDEX IF NOT EXISTS idx_server_tasks_visible
        ON server_tasks (deleted_at, updated_at DESC);
        "#,
    )
    .await?;

    Ok(())
}
