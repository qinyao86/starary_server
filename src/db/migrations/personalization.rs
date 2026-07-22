use super::MigrationTx;
use chrono::{DateTime, Utc};
use serde_json::{json, Map, Value};
use sqlx::{Executor, Row};
use std::collections::{BTreeMap, HashSet};
use uuid::Uuid;

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

    migrate_shared_filter_presets(tx).await?;

    Ok(())
}

async fn migrate_shared_filter_presets(tx: &mut MigrationTx<'_>) -> anyhow::Result<()> {
    let rows = sqlx::query(
        r#"
        SELECT id, library_id, name, value_json, sort_order, created_at, updated_at
        FROM presets
        WHERE "type" = 'filter'
        ORDER BY library_id, sort_order, created_at
        "#,
    )
    .fetch_all(&mut **tx)
    .await?;
    if rows.is_empty() {
        return Ok(());
    }

    let mut presets_by_library = BTreeMap::<String, Vec<Value>>::new();
    for row in rows {
        let library_id: String = row.try_get("library_id")?;
        let created_at: DateTime<Utc> = row.try_get("created_at")?;
        let updated_at: DateTime<Utc> = row.try_get("updated_at")?;
        presets_by_library
            .entry(library_id)
            .or_default()
            .push(json!({
                "id": row.try_get::<String, _>("id")?,
                "name": row.try_get::<String, _>("name")?,
                "value": row.try_get::<Value, _>("value_json")?,
                "sortOrder": row.try_get::<i64, _>("sort_order")?,
                "createdAt": created_at,
                "updatedAt": updated_at,
            }));
    }

    for (library_id, legacy_presets) in presets_by_library {
        let users = sqlx::query(
            r#"
            SELECT DISTINCT u.id, u.preferences
            FROM users u
            LEFT JOIN library_memberships m
              ON m.user_id = u.id AND m.library_id = $1
            WHERE u.is_active = TRUE
              AND (u.global_role IN ('owner', 'admin') OR m.library_id IS NOT NULL)
            "#,
        )
        .bind(&library_id)
        .fetch_all(&mut **tx)
        .await?;

        for user in users {
            let user_id: Uuid = user.try_get("id")?;
            let mut preferences: Value = user.try_get("preferences")?;
            let existing_presets = extract_filter_presets(&preferences, &library_id);
            let mut existing_ids = existing_presets
                .iter()
                .filter_map(|preset| preset.get("id").and_then(Value::as_str))
                .map(ToOwned::to_owned)
                .collect::<HashSet<_>>();
            let mut merged_presets = existing_presets;
            for preset in &legacy_presets {
                let Some(id) = preset.get("id").and_then(Value::as_str) else {
                    continue;
                };
                if existing_ids.insert(id.to_string()) {
                    merged_presets.push(preset.clone());
                }
            }
            set_filter_presets(&mut preferences, &library_id, merged_presets);
            sqlx::query("UPDATE users SET preferences = $2 WHERE id = $1")
                .bind(user_id)
                .bind(preferences)
                .execute(&mut **tx)
                .await?;
        }
    }

    sqlx::query("DELETE FROM presets WHERE \"type\" = 'filter'")
        .execute(&mut **tx)
        .await?;
    Ok(())
}

fn extract_filter_presets(preferences: &Value, library_id: &str) -> Vec<Value> {
    preferences
        .get("libraries")
        .and_then(|libraries| libraries.get(library_id))
        .and_then(|library| library.get("filterPresets"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

fn set_filter_presets(preferences: &mut Value, library_id: &str, presets: Vec<Value>) {
    if !preferences.is_object() {
        *preferences = json!({});
    }
    let root = preferences
        .as_object_mut()
        .expect("preferences must be an object");
    root.insert("version".to_string(), json!(2));
    let libraries = root
        .entry("libraries".to_string())
        .or_insert_with(|| Value::Object(Map::new()));
    if !libraries.is_object() {
        *libraries = Value::Object(Map::new());
    }
    let libraries = libraries
        .as_object_mut()
        .expect("libraries must be an object");
    let library = libraries
        .entry(library_id.to_string())
        .or_insert_with(|| Value::Object(Map::new()));
    if !library.is_object() {
        *library = Value::Object(Map::new());
    }
    library
        .as_object_mut()
        .expect("library preference must be an object")
        .insert("filterPresets".to_string(), Value::Array(presets));
}
