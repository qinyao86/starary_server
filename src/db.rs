use anyhow::Context;
use sqlx::{Executor, PgPool};

pub async fn run_migrations(pool: &PgPool) -> anyhow::Result<()> {
    let mut tx = pool.begin().await.context("failed to begin migration")?;

    tx.execute(
        r#"
        CREATE TABLE IF NOT EXISTS users (
            id UUID PRIMARY KEY,
            email TEXT NOT NULL UNIQUE,
            display_name TEXT NOT NULL,
            password_hash TEXT NOT NULL,
            global_role TEXT NOT NULL CHECK (global_role IN ('owner', 'admin', 'library_manager', 'editor', 'viewer')),
            is_active BOOLEAN NOT NULL DEFAULT TRUE,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );
        "#,
    )
    .await?;

    tx.execute(
        r#"
        DO $$
        BEGIN
            IF to_regclass('public.libraries') IS NULL
               AND to_regclass('public.team_libraries') IS NOT NULL THEN
                ALTER TABLE team_libraries RENAME TO libraries;
            END IF;
        END $$;
        "#,
    )
    .await?;

    tx.execute(
        r#"
        CREATE TABLE IF NOT EXISTS libraries (
            id UUID PRIMARY KEY,
            display_name TEXT NOT NULL,
            description TEXT,
            icon_url TEXT,
            created_by_user_id UUID NOT NULL REFERENCES users(id),
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            deleted_at TIMESTAMPTZ
        );
        "#,
    )
    .await?;

    tx.execute(
        r#"
        DO $$
        BEGIN
            IF EXISTS (
                SELECT 1 FROM information_schema.columns
                WHERE table_schema = 'public'
                  AND table_name = 'libraries'
                  AND column_name = 'name'
            ) AND NOT EXISTS (
                SELECT 1 FROM information_schema.columns
                WHERE table_schema = 'public'
                  AND table_name = 'libraries'
                  AND column_name = 'display_name'
            ) THEN
                ALTER TABLE libraries RENAME COLUMN name TO display_name;
            END IF;
        END $$;
        "#,
    )
    .await?;

    tx.execute("ALTER TABLE libraries ADD COLUMN IF NOT EXISTS display_name TEXT;")
        .await?;
    tx.execute(
        "UPDATE libraries SET display_name = 'Untitled Library' WHERE display_name IS NULL OR btrim(display_name) = '';",
    )
    .await?;
    tx.execute("ALTER TABLE libraries ALTER COLUMN display_name SET NOT NULL;")
        .await?;

    tx.execute(
        r#"
        CREATE TABLE IF NOT EXISTS library_memberships (
            library_id UUID NOT NULL REFERENCES libraries(id) ON DELETE CASCADE,
            user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            role TEXT NOT NULL CHECK (role IN ('owner', 'admin', 'library_manager', 'editor', 'viewer')),
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            PRIMARY KEY (library_id, user_id)
        );
        "#,
    )
    .await?;

    tx.execute(
        r#"
        CREATE TABLE IF NOT EXISTS storage_roots (
            id UUID PRIMARY KEY,
            library_id UUID NOT NULL REFERENCES libraries(id) ON DELETE CASCADE,
            name TEXT NOT NULL,
            kind TEXT NOT NULL CHECK (kind IN ('server_filesystem', 'smb', 's3')),
            canonical_uri TEXT NOT NULL,
            windows_unc_path TEXT,
            windows_mapped_drive_aliases JSONB NOT NULL DEFAULT '[]'::jsonb,
            macos_smb_url TEXT,
            macos_mount_aliases JSONB NOT NULL DEFAULT '[]'::jsonb,
            enabled BOOLEAN NOT NULL DEFAULT TRUE,
            created_by_user_id UUID NOT NULL REFERENCES users(id),
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            UNIQUE (library_id, name)
        );
        "#,
    )
    .await?;

    tx.execute(
        r#"
        CREATE TABLE IF NOT EXISTS assets (
            id UUID PRIMARY KEY,
            library_id UUID NOT NULL REFERENCES libraries(id) ON DELETE CASCADE,
            name TEXT NOT NULL,
            asset_kind TEXT NOT NULL,
            import_mode TEXT NOT NULL CHECK (import_mode IN ('copy', 'reference')),
            storage_key TEXT,
            storage_root_id UUID REFERENCES storage_roots(id),
            relative_path TEXT,
            metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
            created_by_user_id UUID NOT NULL REFERENCES users(id),
            imported_by_user_id UUID REFERENCES users(id),
            updated_by_user_id UUID REFERENCES users(id),
            deleted_by_user_id UUID REFERENCES users(id),
            restored_by_user_id UUID REFERENCES users(id),
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            imported_at TIMESTAMPTZ,
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            deleted_at TIMESTAMPTZ,
            restored_at TIMESTAMPTZ,
            CHECK (
                (import_mode = 'copy' AND storage_key IS NOT NULL)
                OR
                (import_mode = 'reference' AND storage_root_id IS NOT NULL AND relative_path IS NOT NULL)
            )
        );
        "#,
    )
    .await?;

    tx.execute(
        r#"
        DO $$
        BEGIN
            IF EXISTS (
                SELECT 1 FROM information_schema.columns
                WHERE table_schema = 'public'
                  AND table_name = 'assets'
                  AND column_name = 'title'
            ) AND NOT EXISTS (
                SELECT 1 FROM information_schema.columns
                WHERE table_schema = 'public'
                  AND table_name = 'assets'
                  AND column_name = 'name'
            ) THEN
                ALTER TABLE assets RENAME COLUMN title TO name;
            END IF;
        END $$;
        "#,
    )
    .await?;

    tx.execute("ALTER TABLE assets ADD COLUMN IF NOT EXISTS name TEXT;")
        .await?;
    tx.execute(
        "ALTER TABLE assets ADD COLUMN IF NOT EXISTS metadata JSONB NOT NULL DEFAULT '{}'::jsonb;",
    )
    .await?;
    tx.execute(
        r#"
        UPDATE assets
        SET name = COALESCE(
            NULLIF(btrim(name), ''),
            NULLIF(btrim(metadata->>'name'), ''),
            NULLIF(btrim(metadata->>'title'), ''),
            id::text
        )
        WHERE name IS NULL OR btrim(name) = '';
        "#,
    )
    .await?;
    tx.execute("ALTER TABLE assets ALTER COLUMN name SET NOT NULL;")
        .await?;

    tx.execute(
        r#"
        CREATE TABLE IF NOT EXISTS activity_log (
            id UUID PRIMARY KEY,
            library_id UUID REFERENCES libraries(id) ON DELETE CASCADE,
            actor_user_id UUID REFERENCES users(id),
            action TEXT NOT NULL,
            target_type TEXT NOT NULL,
            target_id UUID,
            details JSONB NOT NULL DEFAULT '{}'::jsonb,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );
        "#,
    )
    .await?;

    // Existing installed databases may have been created by an older build. Keep
    // these schema additions idempotent so startup upgrades them in place.
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
    tx.execute("ALTER TABLE activity_log ADD COLUMN IF NOT EXISTS target_id UUID;")
        .await?;
    tx.execute("ALTER TABLE activity_log ADD COLUMN IF NOT EXISTS details JSONB NOT NULL DEFAULT '{}'::jsonb;")
        .await?;

    tx.execute("CREATE INDEX IF NOT EXISTS idx_assets_library_id ON assets(library_id);")
        .await?;
    tx.execute("CREATE INDEX IF NOT EXISTS idx_assets_storage_root_id ON assets(storage_root_id);")
        .await?;
    tx.execute(
        "CREATE INDEX IF NOT EXISTS idx_activity_log_library_id ON activity_log(library_id);",
    )
    .await?;
    tx.execute("CREATE INDEX IF NOT EXISTS idx_users_last_seen_at ON users(last_seen_at);")
        .await?;
    tx.execute(
        "CREATE INDEX IF NOT EXISTS idx_users_last_seen_library_id ON users(last_seen_library_id);",
    )
    .await?;

    tx.commit().await.context("failed to commit migration")?;
    Ok(())
}
