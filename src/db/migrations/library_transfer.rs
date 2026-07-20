use super::MigrationTx;
use sqlx::Executor;

pub(super) async fn create_library_transfer_schema(tx: &mut MigrationTx<'_>) -> anyhow::Result<()> {
    tx.execute(
        r#"
        CREATE TABLE IF NOT EXISTS library_transfer_operations (
            operation_id TEXT PRIMARY KEY,
            source_library_id TEXT NOT NULL REFERENCES libraries(id) ON DELETE CASCADE,
            target_library_id TEXT NOT NULL REFERENCES libraries(id) ON DELETE CASCADE,
            user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            item_kind TEXT NOT NULL CHECK (item_kind IN ('asset', 'folder')),
            source_item_id TEXT NOT NULL,
            response JSONB NOT NULL,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );
        "#,
    )
    .await?;

    Ok(())
}
