use super::MigrationTx;
use sqlx::Executor;

pub(super) async fn upgrade_existing_schema(tx: &mut MigrationTx<'_>) -> anyhow::Result<()> {
    tx.execute(
        "ALTER TABLE users ADD COLUMN IF NOT EXISTS is_active BOOLEAN NOT NULL DEFAULT TRUE;",
    )
    .await?;
    tx.execute(
        "ALTER TABLE users ADD COLUMN IF NOT EXISTS updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW();",
    )
    .await?;
    tx.execute("ALTER TABLE libraries ADD COLUMN IF NOT EXISTS updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW();")
        .await?;
    tx.execute("ALTER TABLE libraries ADD COLUMN IF NOT EXISTS icon_url TEXT;")
        .await?;
    tx.execute("ALTER TABLE libraries ADD COLUMN IF NOT EXISTS deleted_at TIMESTAMPTZ;")
        .await?;
    tx.execute("ALTER TABLE users ADD COLUMN IF NOT EXISTS last_login_at TIMESTAMPTZ;")
        .await?;
    tx.execute("ALTER TABLE users ADD COLUMN IF NOT EXISTS last_seen_at TIMESTAMPTZ;")
        .await?;
    tx.execute(
        "ALTER TABLE users ADD COLUMN IF NOT EXISTS last_seen_library_id UUID REFERENCES libraries(id) ON DELETE SET NULL;",
    )
    .await?;
    tx.execute("ALTER TABLE library_memberships ADD COLUMN IF NOT EXISTS updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW();")
        .await?;
    tx.execute("ALTER TABLE storage_roots ADD COLUMN IF NOT EXISTS windows_mapped_drive_aliases JSONB NOT NULL DEFAULT '[]'::jsonb;")
        .await?;
    tx.execute("ALTER TABLE storage_roots ADD COLUMN IF NOT EXISTS macos_mount_aliases JSONB NOT NULL DEFAULT '[]'::jsonb;")
        .await?;
    tx.execute(
        "ALTER TABLE storage_roots ADD COLUMN IF NOT EXISTS enabled BOOLEAN NOT NULL DEFAULT TRUE;",
    )
    .await?;
    tx.execute("ALTER TABLE storage_roots ADD COLUMN IF NOT EXISTS updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW();")
        .await?;
    tx.execute("ALTER TABLE assets ADD COLUMN IF NOT EXISTS storage_key TEXT;")
        .await?;
    tx.execute("ALTER TABLE assets ADD COLUMN IF NOT EXISTS storage_root_id UUID REFERENCES storage_roots(id);")
        .await?;
    tx.execute("ALTER TABLE assets ADD COLUMN IF NOT EXISTS relative_path TEXT;")
        .await?;
    tx.execute(
        "ALTER TABLE assets ADD COLUMN IF NOT EXISTS metadata JSONB NOT NULL DEFAULT '{}'::jsonb;",
    )
    .await?;
    tx.execute("ALTER TABLE assets ADD COLUMN IF NOT EXISTS imported_by_user_id UUID REFERENCES users(id);")
        .await?;
    tx.execute(
        "ALTER TABLE assets ADD COLUMN IF NOT EXISTS updated_by_user_id UUID REFERENCES users(id);",
    )
    .await?;
    tx.execute(
        "ALTER TABLE assets ADD COLUMN IF NOT EXISTS deleted_by_user_id UUID REFERENCES users(id);",
    )
    .await?;
    tx.execute("ALTER TABLE assets ADD COLUMN IF NOT EXISTS restored_by_user_id UUID REFERENCES users(id);")
        .await?;
    tx.execute("ALTER TABLE assets ADD COLUMN IF NOT EXISTS imported_at TIMESTAMPTZ;")
        .await?;
    tx.execute("ALTER TABLE assets ADD COLUMN IF NOT EXISTS updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW();")
        .await?;
    tx.execute("ALTER TABLE assets ADD COLUMN IF NOT EXISTS deleted_at TIMESTAMPTZ;")
        .await?;
    tx.execute("ALTER TABLE assets ADD COLUMN IF NOT EXISTS restored_at TIMESTAMPTZ;")
        .await?;

    tx.execute(
        "ALTER TABLE folders ADD COLUMN IF NOT EXISTS description TEXT NOT NULL DEFAULT '';",
    )
    .await?;
    tx.execute("ALTER TABLE folders ADD COLUMN IF NOT EXISTS icon TEXT NOT NULL DEFAULT 'folder';")
        .await?;
    tx.execute(
        "ALTER TABLE folders ADD COLUMN IF NOT EXISTS color TEXT NOT NULL DEFAULT 'default';",
    )
    .await?;
    tx.execute("ALTER TABLE folders ADD COLUMN IF NOT EXISTS cover_asset_id UUID REFERENCES assets(id) ON DELETE SET NULL;")
        .await?;
    tx.execute("ALTER TABLE folders ADD COLUMN IF NOT EXISTS smart_import_id TEXT;")
        .await?;
    tx.execute(
        "ALTER TABLE folders ADD COLUMN IF NOT EXISTS sort_order BIGINT NOT NULL DEFAULT 0;",
    )
    .await?;
    tx.execute("ALTER TABLE folders ADD COLUMN IF NOT EXISTS created_by_user_id UUID REFERENCES users(id);")
        .await?;
    tx.execute("ALTER TABLE folders ADD COLUMN IF NOT EXISTS updated_by_user_id UUID REFERENCES users(id);")
        .await?;
    tx.execute("ALTER TABLE folders ADD COLUMN IF NOT EXISTS updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW();")
        .await?;
    tx.execute("ALTER TABLE folders ADD COLUMN IF NOT EXISTS created_at TIMESTAMPTZ NOT NULL DEFAULT NOW();")
        .await?;

    tx.execute(
        "ALTER TABLE tag_groups ADD COLUMN IF NOT EXISTS color TEXT NOT NULL DEFAULT 'default';",
    )
    .await?;
    tx.execute(
        "ALTER TABLE tag_groups ADD COLUMN IF NOT EXISTS sort_order BIGINT NOT NULL DEFAULT 0;",
    )
    .await?;
    tx.execute("ALTER TABLE tag_groups ADD COLUMN IF NOT EXISTS created_by_user_id UUID REFERENCES users(id);")
        .await?;
    tx.execute("ALTER TABLE tag_groups ADD COLUMN IF NOT EXISTS updated_by_user_id UUID REFERENCES users(id);")
        .await?;
    tx.execute("ALTER TABLE tag_groups ADD COLUMN IF NOT EXISTS updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW();")
        .await?;
    tx.execute("ALTER TABLE tag_groups ADD COLUMN IF NOT EXISTS created_at TIMESTAMPTZ NOT NULL DEFAULT NOW();")
        .await?;

    tx.execute("ALTER TABLE tags ADD COLUMN IF NOT EXISTS color TEXT;")
        .await?;
    tx.execute("ALTER TABLE tags ADD COLUMN IF NOT EXISTS starred BOOLEAN NOT NULL DEFAULT FALSE;")
        .await?;
    tx.execute("ALTER TABLE tags ADD COLUMN IF NOT EXISTS sort_order BIGINT NOT NULL DEFAULT 0;")
        .await?;
    tx.execute(
        "ALTER TABLE tags ADD COLUMN IF NOT EXISTS created_by_user_id UUID REFERENCES users(id);",
    )
    .await?;
    tx.execute(
        "ALTER TABLE tags ADD COLUMN IF NOT EXISTS updated_by_user_id UUID REFERENCES users(id);",
    )
    .await?;
    tx.execute(
        "ALTER TABLE tags ADD COLUMN IF NOT EXISTS updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW();",
    )
    .await?;
    tx.execute(
        "ALTER TABLE tags ADD COLUMN IF NOT EXISTS created_at TIMESTAMPTZ NOT NULL DEFAULT NOW();",
    )
    .await?;

    tx.execute("ALTER TABLE activity_log ADD COLUMN IF NOT EXISTS target_id UUID;")
        .await?;
    tx.execute("ALTER TABLE activity_log ADD COLUMN IF NOT EXISTS details JSONB NOT NULL DEFAULT '{}'::jsonb;")
        .await?;

    Ok(())
}
