use super::MigrationTx;
use sqlx::Executor;

pub(super) async fn create_presets_schema(tx: &mut MigrationTx<'_>) -> anyhow::Result<()> {
    tx.execute(
        r#"
        CREATE TABLE IF NOT EXISTS presets (
            id TEXT PRIMARY KEY,
            library_id TEXT NOT NULL REFERENCES libraries(id) ON DELETE CASCADE,
            "type" TEXT NOT NULL,
            name TEXT NOT NULL,
            value_json JSONB NOT NULL DEFAULT '{}'::jsonb,
            item_count BIGINT NOT NULL DEFAULT 0,
            sort_order BIGINT NOT NULL DEFAULT 0,
            created_by_user_id UUID REFERENCES users(id),
            updated_by_user_id UUID REFERENCES users(id),
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );
        "#,
    )
    .await?;

    Ok(())
}
