use super::MigrationTx;
use sqlx::Executor;

pub(super) async fn create_indexes(tx: &mut MigrationTx<'_>) -> anyhow::Result<()> {
    tx.execute("CREATE INDEX IF NOT EXISTS idx_assets_library_id ON assets(library_id);")
        .await?;
    tx.execute("CREATE INDEX IF NOT EXISTS idx_assets_storage_root_id ON assets(storage_root_id);")
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
