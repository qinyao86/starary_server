use super::MigrationTx;
use crate::ids::{generate_id, is_prefixed_id};
use sqlx::{Executor, Row};
use std::collections::HashSet;

pub(super) async fn upgrade_existing_schema(tx: &mut MigrationTx<'_>) -> anyhow::Result<()> {
    convert_domain_ids_to_text(tx).await?;

    tx.execute(
        "ALTER TABLE users ADD COLUMN IF NOT EXISTS is_active BOOLEAN NOT NULL DEFAULT TRUE;",
    )
    .await?;
    tx.execute("ALTER TABLE users ADD COLUMN IF NOT EXISTS avatar_key TEXT;")
        .await?;
    tx.execute(
        "ALTER TABLE users ADD COLUMN IF NOT EXISTS updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW();",
    )
    .await?;
    tx.execute("ALTER TABLE libraries ADD COLUMN IF NOT EXISTS updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW();")
        .await?;
    tx.execute("ALTER TABLE libraries ADD COLUMN IF NOT EXISTS icon_url TEXT;")
        .await?;
    tx.execute("ALTER TABLE libraries DROP COLUMN IF EXISTS description;")
        .await?;
    tx.execute(
        "ALTER TABLE libraries ADD COLUMN IF NOT EXISTS enabled BOOLEAN NOT NULL DEFAULT TRUE;",
    )
    .await?;
    tx.execute(
        "ALTER TABLE libraries ADD COLUMN IF NOT EXISTS access_mode TEXT NOT NULL DEFAULT 'invite';",
    )
    .await?;
    tx.execute(
        "UPDATE libraries SET access_mode = 'invite' WHERE access_mode NOT IN ('public', 'invite');",
    )
    .await?;
    tx.execute("ALTER TABLE libraries ADD COLUMN IF NOT EXISTS deleted_at TIMESTAMPTZ;")
        .await?;
    tx.execute("ALTER TABLE libraries ADD COLUMN IF NOT EXISTS storage_locked_at TIMESTAMPTZ;")
        .await?;
    tx.execute("ALTER TABLE users ADD COLUMN IF NOT EXISTS last_login_at TIMESTAMPTZ;")
        .await?;
    tx.execute("ALTER TABLE users ADD COLUMN IF NOT EXISTS last_seen_at TIMESTAMPTZ;")
        .await?;
    tx.execute("ALTER TABLE users ADD COLUMN IF NOT EXISTS last_seen_library_id TEXT;")
        .await?;
    tx.execute("ALTER TABLE library_memberships ADD COLUMN IF NOT EXISTS updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW();")
        .await?;
    tx.execute(
        "ALTER TABLE storage_connections ADD COLUMN IF NOT EXISTS is_default BOOLEAN NOT NULL DEFAULT FALSE;",
    )
    .await?;
    tx.execute(
        "UPDATE storage_connections SET is_default = FALSE WHERE is_default AND NOT enabled;",
    )
    .await?;
    tx.execute(
        r#"
        WITH ranked AS (
            SELECT id, ROW_NUMBER() OVER (ORDER BY created_at, id) AS position
            FROM storage_connections
            WHERE is_default AND enabled
        )
        UPDATE storage_connections sc
        SET is_default = FALSE
        FROM ranked
        WHERE sc.id = ranked.id AND ranked.position > 1;

        UPDATE storage_connections
        SET is_default = TRUE
        WHERE id = (
            SELECT id
            FROM storage_connections
            WHERE enabled
            ORDER BY created_at, id
            LIMIT 1
        )
        AND NOT EXISTS (
            SELECT 1 FROM storage_connections WHERE is_default AND enabled
        );
        "#,
    )
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
    tx.execute("ALTER TABLE storage_roots ADD COLUMN IF NOT EXISTS storage_connection_id UUID;")
        .await?;
    tx.execute(
        "ALTER TABLE storage_roots ADD COLUMN IF NOT EXISTS namespace TEXT NOT NULL DEFAULT '';",
    )
    .await?;
    tx.execute("ALTER TABLE storage_roots ADD COLUMN IF NOT EXISTS storage_identity TEXT;")
        .await?;
    tx.execute(
        r#"
        INSERT INTO storage_connections (
            id, name, kind, canonical_uri, windows_unc_path,
            windows_mapped_drive_aliases, macos_smb_url, macos_mount_aliases,
            enabled, created_by_user_id, created_at, updated_at
        )
        SELECT
            sr.id,
            CASE
                WHEN COUNT(*) OVER (PARTITION BY lower(sr.name)) > 1
                    THEN sr.name || ' - ' || l.display_name
                ELSE sr.name
            END,
            sr.kind,
            sr.canonical_uri,
            sr.windows_unc_path,
            sr.windows_mapped_drive_aliases,
            sr.macos_smb_url,
            sr.macos_mount_aliases,
            sr.enabled,
            sr.created_by_user_id,
            sr.created_at,
            sr.updated_at
        FROM storage_roots sr
        JOIN libraries l ON l.id = sr.library_id
        WHERE sr.storage_connection_id IS NULL
        ON CONFLICT (id) DO NOTHING;

        UPDATE storage_roots
        SET storage_connection_id = id,
            namespace = ''
        WHERE storage_connection_id IS NULL;
        "#,
    )
    .await?;
    tx.execute(
        r#"
        DO $$
        BEGIN
            IF NOT EXISTS (
                SELECT 1 FROM pg_constraint
                WHERE conname = 'storage_roots_storage_connection_id_fkey'
            ) THEN
                ALTER TABLE storage_roots
                    ADD CONSTRAINT storage_roots_storage_connection_id_fkey
                    FOREIGN KEY (storage_connection_id) REFERENCES storage_connections(id);
            END IF;
        END $$;
        "#,
    )
    .await?;
    tx.execute("ALTER TABLE storage_roots ALTER COLUMN storage_connection_id SET NOT NULL;")
        .await?;
    tx.execute(
        r#"
        UPDATE storage_roots
        SET storage_identity = lower(regexp_replace(replace(btrim(canonical_uri), '\', '/'), '/+$', ''))
        WHERE storage_identity IS NULL OR btrim(storage_identity) = ''
        "#,
    )
    .await?;
    tx.execute("ALTER TABLE storage_roots ALTER COLUMN storage_identity SET NOT NULL;")
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
    tx.execute(
        r#"
        UPDATE libraries l
        SET storage_locked_at = COALESCE(l.storage_locked_at, first_asset.created_at)
        FROM (
            SELECT library_id, MIN(created_at) AS created_at
            FROM assets
            WHERE storage_root_id IS NOT NULL
            GROUP BY library_id
        ) first_asset
        WHERE first_asset.library_id = l.id
          AND l.storage_locked_at IS NULL
        "#,
    )
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
    tx.execute("ALTER TABLE folders ADD COLUMN IF NOT EXISTS cover_asset_id TEXT;")
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

    tx.execute("ALTER TABLE activity_log ADD COLUMN IF NOT EXISTS target_id TEXT;")
        .await?;
    tx.execute("ALTER TABLE activity_log ADD COLUMN IF NOT EXISTS details JSONB NOT NULL DEFAULT '{}'::jsonb;")
        .await?;

    // Library deletion has no restore workflow. Remove legacy soft-deleted rows so
    // their cascading records no longer reserve storage connections.
    tx.execute("DELETE FROM libraries WHERE deleted_at IS NOT NULL;")
        .await?;

    Ok(())
}

async fn convert_domain_ids_to_text(tx: &mut MigrationTx<'_>) -> anyhow::Result<()> {
    tx.execute("ALTER TABLE users ADD COLUMN IF NOT EXISTS last_seen_library_id TEXT;")
        .await?;
    tx.execute("ALTER TABLE folders ADD COLUMN IF NOT EXISTS cover_asset_id TEXT;")
        .await?;
    tx.execute("ALTER TABLE activity_log ADD COLUMN IF NOT EXISTS target_id TEXT;")
        .await?;

    tx.execute(
        r#"
        DO $$
        DECLARE
            item RECORD;
        BEGIN
            FOR item IN
                SELECT DISTINCT c.conrelid::regclass::text AS table_name, c.conname AS constraint_name
                FROM pg_constraint c
                WHERE c.contype = 'f'
                  AND c.confrelid IN (
                    'libraries'::regclass,
                    'assets'::regclass,
                    'folders'::regclass,
                    'tag_groups'::regclass,
                    'tags'::regclass
                  )
            LOOP
                EXECUTE format('ALTER TABLE %s DROP CONSTRAINT %I', item.table_name, item.constraint_name);
            END LOOP;
        END $$;
        "#,
    )
    .await?;

    normalize_domain_id_shapes(tx).await?;

    tx.execute(
        r#"
        DO $$
        DECLARE
            item RECORD;
        BEGIN
            FOR item IN
                SELECT *
                FROM (VALUES
                    ('libraries', 'id'),
                    ('library_memberships', 'library_id'),
                    ('storage_roots', 'library_id'),
                    ('assets', 'id'),
                    ('assets', 'library_id'),
                    ('folders', 'library_id'),
                    ('folders', 'cover_asset_id'),
                    ('tag_groups', 'library_id'),
                    ('tags', 'library_id'),
                    ('asset_folders', 'asset_id'),
                    ('asset_tags', 'asset_id'),
                    ('presets', 'library_id'),
                    ('activity_log', 'library_id'),
                    ('activity_log', 'target_id'),
                    ('users', 'last_seen_library_id')
                ) AS columns(table_name, column_name)
            LOOP
                IF EXISTS (
                    SELECT 1
                    FROM information_schema.columns
                    WHERE table_schema = 'public'
                      AND table_name = item.table_name
                      AND column_name = item.column_name
                      AND udt_name = 'uuid'
                ) THEN
                    EXECUTE format(
                        'ALTER TABLE %I ALTER COLUMN %I TYPE TEXT USING %I::text',
                        item.table_name,
                        item.column_name,
                        item.column_name
                    );
                END IF;
            END LOOP;
        END $$;
        "#,
    )
    .await?;

    tx.execute(
        r#"
        DO $$
        BEGIN
            IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'library_memberships_library_id_fkey') THEN
                ALTER TABLE library_memberships
                    ADD CONSTRAINT library_memberships_library_id_fkey
                    FOREIGN KEY (library_id) REFERENCES libraries(id) ON DELETE CASCADE;
            END IF;

            IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'storage_roots_library_id_fkey') THEN
                ALTER TABLE storage_roots
                    ADD CONSTRAINT storage_roots_library_id_fkey
                    FOREIGN KEY (library_id) REFERENCES libraries(id) ON DELETE CASCADE;
            END IF;

            IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'assets_library_id_fkey') THEN
                ALTER TABLE assets
                    ADD CONSTRAINT assets_library_id_fkey
                    FOREIGN KEY (library_id) REFERENCES libraries(id) ON DELETE CASCADE;
            END IF;

            IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'folders_library_id_fkey') THEN
                ALTER TABLE folders
                    ADD CONSTRAINT folders_library_id_fkey
                    FOREIGN KEY (library_id) REFERENCES libraries(id) ON DELETE CASCADE;
            END IF;

            IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'folders_cover_asset_id_fkey') THEN
                ALTER TABLE folders
                    ADD CONSTRAINT folders_cover_asset_id_fkey
                    FOREIGN KEY (cover_asset_id) REFERENCES assets(id) ON DELETE SET NULL;
            END IF;

            IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'folders_parent_id_fkey') THEN
                ALTER TABLE folders
                    ADD CONSTRAINT folders_parent_id_fkey
                    FOREIGN KEY (parent_id) REFERENCES folders(id) ON DELETE CASCADE;
            END IF;

            IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'tag_groups_library_id_fkey') THEN
                ALTER TABLE tag_groups
                    ADD CONSTRAINT tag_groups_library_id_fkey
                    FOREIGN KEY (library_id) REFERENCES libraries(id) ON DELETE CASCADE;
            END IF;

            IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'tags_library_id_fkey') THEN
                ALTER TABLE tags
                    ADD CONSTRAINT tags_library_id_fkey
                    FOREIGN KEY (library_id) REFERENCES libraries(id) ON DELETE CASCADE;
            END IF;

            IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'tags_group_id_fkey') THEN
                ALTER TABLE tags
                    ADD CONSTRAINT tags_group_id_fkey
                    FOREIGN KEY (group_id) REFERENCES tag_groups(id) ON DELETE SET NULL;
            END IF;

            IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'asset_folders_asset_id_fkey') THEN
                ALTER TABLE asset_folders
                    ADD CONSTRAINT asset_folders_asset_id_fkey
                    FOREIGN KEY (asset_id) REFERENCES assets(id) ON DELETE CASCADE;
            END IF;

            IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'asset_folders_folder_id_fkey') THEN
                ALTER TABLE asset_folders
                    ADD CONSTRAINT asset_folders_folder_id_fkey
                    FOREIGN KEY (folder_id) REFERENCES folders(id) ON DELETE CASCADE;
            END IF;

            IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'asset_tags_asset_id_fkey') THEN
                ALTER TABLE asset_tags
                    ADD CONSTRAINT asset_tags_asset_id_fkey
                    FOREIGN KEY (asset_id) REFERENCES assets(id) ON DELETE CASCADE;
            END IF;

            IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'asset_tags_tag_id_fkey') THEN
                ALTER TABLE asset_tags
                    ADD CONSTRAINT asset_tags_tag_id_fkey
                    FOREIGN KEY (tag_id) REFERENCES tags(id) ON DELETE CASCADE;
            END IF;

            IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'presets_library_id_fkey') THEN
                ALTER TABLE presets
                    ADD CONSTRAINT presets_library_id_fkey
                    FOREIGN KEY (library_id) REFERENCES libraries(id) ON DELETE CASCADE;
            END IF;

            IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'activity_log_library_id_fkey') THEN
                ALTER TABLE activity_log
                    ADD CONSTRAINT activity_log_library_id_fkey
                    FOREIGN KEY (library_id) REFERENCES libraries(id) ON DELETE CASCADE;
            END IF;

            IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'users_last_seen_library_id_fkey') THEN
                ALTER TABLE users
                    ADD CONSTRAINT users_last_seen_library_id_fkey
                    FOREIGN KEY (last_seen_library_id) REFERENCES libraries(id) ON DELETE SET NULL;
            END IF;
        END $$;
        "#,
    )
    .await?;

    Ok(())
}

struct IdRewrite {
    old_id: String,
    new_id: String,
}

async fn normalize_domain_id_shapes(tx: &mut MigrationTx<'_>) -> anyhow::Result<()> {
    let library_rewrites = rewrite_primary_ids(tx, "libraries", "lib_").await?;
    rewrite_storage_namespace_paths(tx, &library_rewrites).await?;
    rewrite_reference_column(tx, "library_memberships", "library_id", &library_rewrites).await?;
    rewrite_reference_column(tx, "storage_roots", "library_id", &library_rewrites).await?;
    rewrite_reference_column(tx, "assets", "library_id", &library_rewrites).await?;
    rewrite_reference_column(tx, "folders", "library_id", &library_rewrites).await?;
    rewrite_reference_column(tx, "tag_groups", "library_id", &library_rewrites).await?;
    rewrite_reference_column(tx, "tags", "library_id", &library_rewrites).await?;
    rewrite_reference_column(tx, "presets", "library_id", &library_rewrites).await?;
    rewrite_reference_column(tx, "activity_log", "library_id", &library_rewrites).await?;
    rewrite_reference_column(tx, "users", "last_seen_library_id", &library_rewrites).await?;
    rewrite_activity_targets(tx, &library_rewrites).await?;
    rewrite_json_id_mentions(tx, &library_rewrites).await?;

    let asset_rewrites = rewrite_primary_ids(tx, "assets", "asset_").await?;
    rewrite_reference_column(tx, "folders", "cover_asset_id", &asset_rewrites).await?;
    rewrite_reference_column(tx, "asset_folders", "asset_id", &asset_rewrites).await?;
    rewrite_reference_column(tx, "asset_tags", "asset_id", &asset_rewrites).await?;
    rewrite_activity_targets(tx, &asset_rewrites).await?;
    rewrite_json_id_mentions(tx, &asset_rewrites).await?;

    let folder_rewrites = rewrite_primary_ids(tx, "folders", "folder_").await?;
    rewrite_reference_column(tx, "folders", "parent_id", &folder_rewrites).await?;
    rewrite_reference_column(tx, "asset_folders", "folder_id", &folder_rewrites).await?;
    rewrite_activity_targets(tx, &folder_rewrites).await?;
    rewrite_json_id_mentions(tx, &folder_rewrites).await?;

    let tag_group_rewrites = rewrite_primary_ids(tx, "tag_groups", "tag_group_").await?;
    rewrite_reference_column(tx, "tags", "group_id", &tag_group_rewrites).await?;
    rewrite_activity_targets(tx, &tag_group_rewrites).await?;
    rewrite_json_id_mentions(tx, &tag_group_rewrites).await?;

    let tag_rewrites = rewrite_primary_ids(tx, "tags", "tag_").await?;
    rewrite_reference_column(tx, "asset_tags", "tag_id", &tag_rewrites).await?;
    rewrite_activity_targets(tx, &tag_rewrites).await?;
    rewrite_json_id_mentions(tx, &tag_rewrites).await?;

    let preset_rewrites = rewrite_primary_ids(tx, "presets", "preset_").await?;
    rewrite_reference_column(tx, "folders", "smart_import_id", &preset_rewrites).await?;
    rewrite_activity_targets(tx, &preset_rewrites).await?;
    rewrite_json_id_mentions(tx, &preset_rewrites).await?;

    Ok(())
}

async fn rewrite_primary_ids(
    tx: &mut MigrationTx<'_>,
    table_name: &str,
    prefix: &str,
) -> anyhow::Result<Vec<IdRewrite>> {
    let select_sql = format!("SELECT id FROM {table_name}");
    let rows = sqlx::query(&select_sql).fetch_all(&mut **tx).await?;
    let mut existing_ids = rows
        .iter()
        .map(|row| row.try_get::<String, _>("id"))
        .collect::<Result<HashSet<_>, _>>()?;
    let mut rewrites = Vec::new();

    for row in rows {
        let old_id: String = row.try_get("id")?;
        if is_prefixed_id(&old_id, prefix) {
            continue;
        }

        let mut new_id = generate_id(prefix);
        while existing_ids.contains(&new_id) {
            new_id = generate_id(prefix);
        }
        existing_ids.insert(new_id.clone());
        rewrites.push(IdRewrite { old_id, new_id });
    }

    let update_sql = format!("UPDATE {table_name} SET id = $2 WHERE id = $1");
    for rewrite in &rewrites {
        sqlx::query(&update_sql)
            .bind(&rewrite.old_id)
            .bind(&rewrite.new_id)
            .execute(&mut **tx)
            .await?;
    }

    Ok(rewrites)
}

async fn rewrite_reference_column(
    tx: &mut MigrationTx<'_>,
    table_name: &str,
    column_name: &str,
    rewrites: &[IdRewrite],
) -> anyhow::Result<()> {
    if rewrites.is_empty() {
        return Ok(());
    }

    let update_sql = format!("UPDATE {table_name} SET {column_name} = $2 WHERE {column_name} = $1");
    for rewrite in rewrites {
        sqlx::query(&update_sql)
            .bind(&rewrite.old_id)
            .bind(&rewrite.new_id)
            .execute(&mut **tx)
            .await?;
    }

    Ok(())
}

async fn rewrite_activity_targets(
    tx: &mut MigrationTx<'_>,
    rewrites: &[IdRewrite],
) -> anyhow::Result<()> {
    rewrite_reference_column(tx, "activity_log", "target_id", rewrites).await
}

async fn rewrite_json_id_mentions(
    tx: &mut MigrationTx<'_>,
    rewrites: &[IdRewrite],
) -> anyhow::Result<()> {
    for rewrite in rewrites {
        sqlx::query(
            r#"
            UPDATE activity_log
            SET details = replace(details::text, $1, $2)::jsonb
            WHERE details::text LIKE '%' || $1 || '%'
            "#,
        )
        .bind(&rewrite.old_id)
        .bind(&rewrite.new_id)
        .execute(&mut **tx)
        .await?;

        sqlx::query(
            r#"
            UPDATE presets
            SET value_json = replace(value_json::text, $1, $2)::jsonb
            WHERE value_json::text LIKE '%' || $1 || '%'
            "#,
        )
        .bind(&rewrite.old_id)
        .bind(&rewrite.new_id)
        .execute(&mut **tx)
        .await?;
    }

    Ok(())
}

async fn rewrite_storage_namespace_paths(
    tx: &mut MigrationTx<'_>,
    rewrites: &[IdRewrite],
) -> anyhow::Result<()> {
    for rewrite in rewrites {
        // Existing libraries with assets may already have files under the old namespace.
        // In that case we keep path text stable and only normalize IDs/references.
        let asset_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM assets WHERE library_id = $1")
                .bind(&rewrite.old_id)
                .fetch_one(&mut **tx)
                .await?;
        if asset_count > 0 {
            continue;
        }

        sqlx::query(
            r#"
            UPDATE storage_roots
            SET canonical_uri = CASE
                    WHEN canonical_uri LIKE '%' || $1
                    THEN left(canonical_uri, char_length(canonical_uri) - char_length($1)) || $2
                    ELSE canonical_uri
                END,
                windows_unc_path = CASE
                    WHEN windows_unc_path LIKE '%' || $1
                    THEN left(windows_unc_path, char_length(windows_unc_path) - char_length($1)) || $2
                    ELSE windows_unc_path
                END,
                macos_smb_url = CASE
                    WHEN macos_smb_url LIKE '%' || $1
                    THEN left(macos_smb_url, char_length(macos_smb_url) - char_length($1)) || $2
                    ELSE macos_smb_url
                END
            WHERE library_id = $1
            "#,
        )
        .bind(&rewrite.old_id)
        .bind(&rewrite.new_id)
        .execute(&mut **tx)
        .await?;
    }

    Ok(())
}
