use super::MigrationTx;
use crate::path_resolver::storage_locations_overlap;
use sqlx::Executor;

pub(super) async fn create_indexes(tx: &mut MigrationTx<'_>) -> anyhow::Result<()> {
    let duplicate_library: Option<String> = sqlx::query_scalar(
        r#"
        SELECT library_id
        FROM storage_roots
        GROUP BY library_id
        HAVING COUNT(*) > 1
        LIMIT 1
        "#,
    )
    .fetch_optional(&mut **tx)
    .await?;
    if let Some(library_id) = duplicate_library {
        anyhow::bail!(
            "library {library_id} has multiple storage bindings; migrate it to one exclusive storage location before upgrading"
        );
    }

    let storage_locations: Vec<(String, String, String)> = sqlx::query_as(
        r#"
        SELECT library_id, kind, canonical_uri
        FROM storage_roots
        ORDER BY library_id
        "#,
    )
    .fetch_all(&mut **tx)
    .await?;
    for (index, (library_id, kind, canonical_uri)) in storage_locations.iter().enumerate() {
        if let Some((other_library_id, _, other_uri)) =
            storage_locations[index + 1..]
                .iter()
                .find(|(_, other_kind, other_uri)| {
                    other_kind == kind && storage_locations_overlap(canonical_uri, other_uri)
                })
        {
            anyhow::bail!(
                "storage paths for libraries {library_id} and {other_library_id} overlap ({canonical_uri} and {other_uri}); migrate one library before upgrading"
            );
        }
    }

    tx.execute("CREATE INDEX IF NOT EXISTS idx_assets_library_id ON assets(library_id);")
        .await?;
    tx.execute(
        "CREATE INDEX IF NOT EXISTS idx_assets_library_deleted_at ON assets(library_id, deleted_at);",
    )
    .await?;
    tx.execute("CREATE INDEX IF NOT EXISTS idx_assets_storage_root_id ON assets(storage_root_id);")
        .await?;
    tx.execute("CREATE UNIQUE INDEX IF NOT EXISTS idx_storage_connections_name ON storage_connections(lower(name));")
        .await?;
    tx.execute(
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_storage_connections_single_default ON storage_connections(is_default) WHERE is_default;",
    )
    .await?;
    tx.execute("CREATE INDEX IF NOT EXISTS idx_storage_roots_connection_id ON storage_roots(storage_connection_id);")
        .await?;
    tx.execute(
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_storage_roots_library_unique ON storage_roots(library_id);",
    )
    .await?;
    tx.execute(
        r#"
        DROP INDEX IF EXISTS idx_storage_roots_location_unique;
        CREATE UNIQUE INDEX idx_storage_roots_location_unique
            ON storage_roots(storage_identity);
        "#,
    )
    .await?;
    tx.execute(
        "CREATE INDEX IF NOT EXISTS idx_folders_library_parent ON folders(library_id, parent_id);",
    )
    .await?;
    tx.execute("CREATE INDEX IF NOT EXISTS idx_folders_cover_asset_id ON folders(cover_asset_id);")
        .await?;
    tx.execute(
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_tags_library_name ON tags(library_id, lower(name));",
    )
    .await?;
    tx.execute("CREATE INDEX IF NOT EXISTS idx_tags_library_group ON tags(library_id, group_id);")
        .await?;
    tx.execute("CREATE UNIQUE INDEX IF NOT EXISTS idx_tag_groups_library_name ON tag_groups(library_id, lower(name));")
        .await?;
    tx.execute(
        "CREATE INDEX IF NOT EXISTS idx_asset_folders_folder_id ON asset_folders(folder_id);",
    )
    .await?;
    tx.execute("CREATE INDEX IF NOT EXISTS idx_asset_folders_folder_asset ON asset_folders(folder_id, asset_id);")
        .await?;
    tx.execute("CREATE INDEX IF NOT EXISTS idx_asset_tags_tag_id ON asset_tags(tag_id);")
        .await?;
    tx.execute(
        "CREATE INDEX IF NOT EXISTS idx_asset_tags_tag_asset ON asset_tags(tag_id, asset_id);",
    )
    .await?;
    tx.execute(
        "CREATE INDEX IF NOT EXISTS idx_asset_favorites_user_library ON asset_favorites(user_id, library_id);",
    )
    .await?;
    tx.execute(
        r#"CREATE INDEX IF NOT EXISTS idx_presets_library_type_order ON presets(library_id, "type", sort_order, created_at);"#,
    )
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

    Ok(())
}
