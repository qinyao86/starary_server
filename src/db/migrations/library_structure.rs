use super::MigrationTx;
use sqlx::Executor;

pub(super) async fn create_library_structure_schema(
    tx: &mut MigrationTx<'_>,
) -> anyhow::Result<()> {
    tx.execute(
        r#"
        CREATE TABLE IF NOT EXISTS folders (
            id TEXT PRIMARY KEY,
            library_id UUID NOT NULL REFERENCES libraries(id) ON DELETE CASCADE,
            parent_id TEXT REFERENCES folders(id) ON DELETE CASCADE,
            name TEXT NOT NULL,
            description TEXT NOT NULL DEFAULT '',
            icon TEXT NOT NULL DEFAULT 'folder',
            color TEXT NOT NULL DEFAULT 'default',
            cover_asset_id UUID REFERENCES assets(id) ON DELETE SET NULL,
            smart_import_id TEXT,
            sort_order BIGINT NOT NULL DEFAULT 0,
            created_by_user_id UUID REFERENCES users(id),
            updated_by_user_id UUID REFERENCES users(id),
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );
        "#,
    )
    .await?;

    tx.execute(
        r#"
        CREATE TABLE IF NOT EXISTS tag_groups (
            id TEXT PRIMARY KEY,
            library_id UUID NOT NULL REFERENCES libraries(id) ON DELETE CASCADE,
            name TEXT NOT NULL,
            color TEXT NOT NULL DEFAULT 'default',
            sort_order BIGINT NOT NULL DEFAULT 0,
            created_by_user_id UUID REFERENCES users(id),
            updated_by_user_id UUID REFERENCES users(id),
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );
        "#,
    )
    .await?;

    tx.execute(
        r#"
        CREATE TABLE IF NOT EXISTS tags (
            id TEXT PRIMARY KEY,
            library_id UUID NOT NULL REFERENCES libraries(id) ON DELETE CASCADE,
            group_id TEXT REFERENCES tag_groups(id) ON DELETE SET NULL,
            name TEXT NOT NULL,
            color TEXT,
            starred BOOLEAN NOT NULL DEFAULT FALSE,
            sort_order BIGINT NOT NULL DEFAULT 0,
            created_by_user_id UUID REFERENCES users(id),
            updated_by_user_id UUID REFERENCES users(id),
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );
        "#,
    )
    .await?;

    tx.execute(
        r#"
        CREATE TABLE IF NOT EXISTS asset_folders (
            asset_id UUID NOT NULL REFERENCES assets(id) ON DELETE CASCADE,
            folder_id TEXT NOT NULL REFERENCES folders(id) ON DELETE CASCADE,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            PRIMARY KEY (asset_id, folder_id)
        );
        "#,
    )
    .await?;

    tx.execute(
        r#"
        CREATE TABLE IF NOT EXISTS asset_tags (
            asset_id UUID NOT NULL REFERENCES assets(id) ON DELETE CASCADE,
            tag_id TEXT NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            PRIMARY KEY (asset_id, tag_id)
        );
        "#,
    )
    .await?;

    Ok(())
}
