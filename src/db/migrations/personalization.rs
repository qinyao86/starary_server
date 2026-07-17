use super::MigrationTx;
use sqlx::Executor;

pub(super) async fn create_personalization_schema(tx: &mut MigrationTx<'_>) -> anyhow::Result<()> {
    tx.execute(
        r#"
        ALTER TABLE users
        ADD COLUMN IF NOT EXISTS preferences JSONB NOT NULL DEFAULT '{}'::jsonb;

        CREATE TABLE IF NOT EXISTS asset_favorites (
            library_id TEXT NOT NULL REFERENCES libraries(id) ON DELETE CASCADE,
            asset_id TEXT NOT NULL REFERENCES assets(id) ON DELETE CASCADE,
            user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            PRIMARY KEY (library_id, asset_id, user_id)
        );

        CREATE INDEX IF NOT EXISTS idx_asset_favorites_user_library
        ON asset_favorites (user_id, library_id, created_at DESC);

        CREATE INDEX IF NOT EXISTS idx_asset_favorites_library_asset
        ON asset_favorites (library_id, asset_id);
        "#,
    )
    .await?;

    tx.execute(
        r#"
        INSERT INTO asset_favorites (library_id, asset_id, user_id)
        SELECT
            library_id,
            id,
            COALESCE(updated_by_user_id, imported_by_user_id, created_by_user_id)
        FROM assets
        WHERE metadata->'starred' = 'true'::jsonb
          AND COALESCE(updated_by_user_id, imported_by_user_id, created_by_user_id) IS NOT NULL
        ON CONFLICT DO NOTHING;

        UPDATE assets
        SET metadata = metadata - 'starred'
        WHERE metadata ? 'starred';
        "#,
    )
    .await?;

    Ok(())
}
