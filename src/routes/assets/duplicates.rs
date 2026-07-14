use super::{build_asset_responses, query_assets_by_ids, AssetResponse};
use crate::{
    auth::AuthUser,
    error::{AppError, AppResult},
    routes::access::ensure_library_write_access,
    state::AppState,
};
use axum::{
    extract::{Path, State},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::{Postgres, Row, Transaction};
use std::collections::{HashMap, HashSet};

const MAX_MERGE_ASSETS: usize = 500;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MergeDuplicateAssetsRequest {
    decisions: Vec<DuplicateMergeDecision>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DuplicateMergeDecision {
    primary_asset_id: String,
    duplicate_asset_ids: Vec<String>,
    name_asset_id: Option<String>,
    selected_tag_ids: Vec<String>,
    selected_folder_ids: Vec<String>,
    description_asset_id: Option<String>,
    url_asset_id: Option<String>,
    rating: Option<i64>,
    starred: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MergeDuplicateAssetsResponse {
    primary_asset_ids: Vec<String>,
    trashed_asset_ids: Vec<String>,
    affected_count: usize,
    items: Vec<AssetResponse>,
    trashed_items: Vec<AssetResponse>,
    total: i64,
}

#[derive(Debug)]
struct MergeAssetSnapshot {
    asset_kind: String,
    hash: String,
    id: String,
    metadata: Value,
    name: String,
    size_bytes: i64,
}

pub async fn merge_duplicate_assets(
    State(state): State<AppState>,
    user: AuthUser,
    Path(library_id): Path<String>,
    Json(request): Json<MergeDuplicateAssetsRequest>,
) -> AppResult<Json<MergeDuplicateAssetsResponse>> {
    ensure_library_write_access(&state, &user, &library_id).await?;
    if request.decisions.is_empty() {
        return Err(AppError::BadRequest(
            "at least one merge decision is required".to_string(),
        ));
    }

    let mut tx = state.pool.begin().await?;
    let mut primary_asset_ids = Vec::new();
    let mut trashed_asset_ids = Vec::new();
    let mut touched_asset_ids = HashSet::new();

    for decision in request.decisions {
        let group_asset_ids = normalize_group_asset_ids(&decision)?;
        for asset_id in &group_asset_ids {
            if !touched_asset_ids.insert(asset_id.clone()) {
                return Err(AppError::BadRequest(
                    "an asset cannot appear in more than one merge decision".to_string(),
                ));
            }
        }
        if touched_asset_ids.len() > MAX_MERGE_ASSETS {
            return Err(AppError::BadRequest(format!(
                "cannot merge more than {MAX_MERGE_ASSETS} assets at once"
            )));
        }

        let snapshots = load_group_snapshots(&mut tx, &library_id, &group_asset_ids).await?;
        verify_duplicate_group(&snapshots, &decision.primary_asset_id)?;
        let snapshot_by_id = snapshots
            .iter()
            .map(|snapshot| (snapshot.id.as_str(), snapshot))
            .collect::<HashMap<_, _>>();
        let primary = snapshot_by_id
            .get(decision.primary_asset_id.as_str())
            .ok_or_else(|| {
                AppError::BadRequest("primary asset is not in the duplicate group".to_string())
            })?;

        validate_rating(decision.rating)?;
        ensure_relation_targets(&mut tx, "tags", &library_id, &decision.selected_tag_ids).await?;
        ensure_relation_targets(
            &mut tx,
            "folders",
            &library_id,
            &decision.selected_folder_ids,
        )
        .await?;

        let name = selected_name(&snapshot_by_id, decision.name_asset_id.as_deref())
            .unwrap_or_else(|| primary.name.clone());
        let description = selected_metadata_text(
            &snapshot_by_id,
            decision.description_asset_id.as_deref(),
            "description",
        )
        .or_else(|| metadata_text(&primary.metadata, "description"))
        .unwrap_or_default();
        let url = selected_metadata_text(&snapshot_by_id, decision.url_asset_id.as_deref(), "url")
            .or_else(|| metadata_text(&primary.metadata, "url"))
            .unwrap_or_default();

        sqlx::query(
            r#"
            UPDATE assets
            SET name = $3,
                metadata = metadata || jsonb_build_object(
                    'description', $4::text,
                    'url', $5::text,
                    'rating', $6::bigint,
                    'starred', $7::boolean
                ),
                updated_by_user_id = $8,
                updated_at = NOW()
            WHERE library_id = $1 AND id = $2 AND deleted_at IS NULL
            "#,
        )
        .bind(&library_id)
        .bind(&decision.primary_asset_id)
        .bind(name)
        .bind(description)
        .bind(url)
        .bind(decision.rating)
        .bind(decision.starred)
        .bind(user.id)
        .execute(&mut *tx)
        .await?;

        replace_relations(
            &mut tx,
            "asset_tags",
            "tag_id",
            &decision.primary_asset_id,
            &decision.selected_tag_ids,
        )
        .await?;
        replace_relations(
            &mut tx,
            "asset_folders",
            "folder_id",
            &decision.primary_asset_id,
            &decision.selected_folder_ids,
        )
        .await?;

        let duplicate_ids = group_asset_ids
            .into_iter()
            .filter(|asset_id| asset_id != &decision.primary_asset_id)
            .collect::<Vec<_>>();
        let deleted_ids: Vec<String> = sqlx::query_scalar(
            r#"
            UPDATE assets
            SET deleted_at = NOW(),
                deleted_by_user_id = $3,
                restored_at = NULL,
                restored_by_user_id = NULL,
                updated_by_user_id = $3,
                updated_at = NOW()
            WHERE library_id = $1 AND id = ANY($2) AND deleted_at IS NULL
            RETURNING id
            "#,
        )
        .bind(&library_id)
        .bind(&duplicate_ids)
        .bind(user.id)
        .fetch_all(&mut *tx)
        .await?;
        if deleted_ids.len() != duplicate_ids.len() {
            return Err(AppError::BadRequest(
                "one or more duplicate assets could not be moved to trash".to_string(),
            ));
        }

        primary_asset_ids.push(decision.primary_asset_id);
        trashed_asset_ids.extend(deleted_ids);
    }

    sqlx::query(
        "INSERT INTO activity_log (id, library_id, actor_user_id, action, target_type, details) VALUES (gen_random_uuid(), $1, $2, 'assets.duplicates_merged', 'asset', $3::jsonb)",
    )
    .bind(&library_id)
    .bind(user.id)
    .bind(json!({
        "primaryAssetIds": primary_asset_ids,
        "trashedAssetIds": trashed_asset_ids,
    }))
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    let primary_records = query_assets_by_ids(&state, &library_id, &primary_asset_ids).await?;
    let trashed_records = query_assets_by_ids(&state, &library_id, &trashed_asset_ids).await?;
    let items = build_asset_responses(&state, &library_id, primary_records).await?;
    let trashed_items = build_asset_responses(&state, &library_id, trashed_records).await?;
    let total = sqlx::query_scalar(
        "SELECT COUNT(*) FROM assets WHERE library_id = $1 AND deleted_at IS NULL",
    )
    .bind(&library_id)
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(MergeDuplicateAssetsResponse {
        affected_count: primary_asset_ids.len() + trashed_asset_ids.len(),
        primary_asset_ids,
        trashed_asset_ids,
        items,
        trashed_items,
        total,
    }))
}

fn normalize_group_asset_ids(decision: &DuplicateMergeDecision) -> AppResult<Vec<String>> {
    let mut seen = HashSet::new();
    let mut ids = Vec::new();
    for value in
        std::iter::once(&decision.primary_asset_id).chain(decision.duplicate_asset_ids.iter())
    {
        let value = value.trim();
        if !value.is_empty() && seen.insert(value.to_string()) {
            ids.push(value.to_string());
        }
    }
    if ids.len() < 2 {
        return Err(AppError::BadRequest(
            "a duplicate group must contain at least two assets".to_string(),
        ));
    }
    Ok(ids)
}

async fn load_group_snapshots(
    tx: &mut Transaction<'_, Postgres>,
    library_id: &str,
    asset_ids: &[String],
) -> AppResult<Vec<MergeAssetSnapshot>> {
    let rows = sqlx::query(
        r#"
        SELECT id, name, asset_kind, metadata,
               COALESCE(metadata->>'hash', '') AS hash,
               CASE WHEN (metadata->>'sizeBytes') ~ '^[0-9]+$' THEN (metadata->>'sizeBytes')::BIGINT ELSE 0 END AS size_bytes
        FROM assets
        WHERE library_id = $1 AND id = ANY($2) AND deleted_at IS NULL
        FOR UPDATE
        "#,
    )
    .bind(library_id)
    .bind(asset_ids)
    .fetch_all(&mut **tx)
    .await?;
    if rows.len() != asset_ids.len() {
        return Err(AppError::BadRequest(
            "one or more duplicate assets were not found".to_string(),
        ));
    }
    rows.into_iter()
        .map(|row| {
            Ok(MergeAssetSnapshot {
                asset_kind: row.try_get("asset_kind")?,
                hash: row.try_get("hash")?,
                id: row.try_get("id")?,
                metadata: row.try_get("metadata")?,
                name: row.try_get("name")?,
                size_bytes: row.try_get("size_bytes")?,
            })
        })
        .collect()
}

fn verify_duplicate_group(
    snapshots: &[MergeAssetSnapshot],
    primary_asset_id: &str,
) -> AppResult<()> {
    let primary = snapshots
        .iter()
        .find(|asset| asset.id == primary_asset_id)
        .ok_or_else(|| {
            AppError::BadRequest("primary asset is not in the duplicate group".to_string())
        })?;
    if primary.hash.is_empty() || (primary.asset_kind != "link" && primary.size_bytes <= 0) {
        return Err(AppError::BadRequest(
            "duplicate assets must have a valid hash and size".to_string(),
        ));
    }
    let valid = snapshots.iter().all(|asset| {
        asset.hash == primary.hash
            && if primary.asset_kind == "link" {
                asset.asset_kind == "link"
            } else {
                asset.asset_kind != "link" && asset.size_bytes == primary.size_bytes
            }
    });
    if !valid {
        return Err(AppError::BadRequest(
            "assets in a merge decision are not duplicates".to_string(),
        ));
    }
    Ok(())
}

fn selected_name(
    snapshots: &HashMap<&str, &MergeAssetSnapshot>,
    asset_id: Option<&str>,
) -> Option<String> {
    asset_id
        .and_then(|asset_id| snapshots.get(asset_id))
        .map(|asset| asset.name.clone())
}

fn selected_metadata_text(
    snapshots: &HashMap<&str, &MergeAssetSnapshot>,
    asset_id: Option<&str>,
    field: &str,
) -> Option<String> {
    asset_id
        .and_then(|asset_id| snapshots.get(asset_id))
        .and_then(|asset| metadata_text(&asset.metadata, field))
}

fn metadata_text(metadata: &Value, field: &str) -> Option<String> {
    metadata
        .get(field)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}

fn validate_rating(rating: Option<i64>) -> AppResult<()> {
    if rating.is_some_and(|rating| !(0..=5).contains(&rating)) {
        return Err(AppError::BadRequest(
            "asset rating must be between 0 and 5".to_string(),
        ));
    }
    Ok(())
}

async fn ensure_relation_targets(
    tx: &mut Transaction<'_, Postgres>,
    table: &str,
    library_id: &str,
    ids: &[String],
) -> AppResult<()> {
    if ids.is_empty() {
        return Ok(());
    }
    let ids = ids
        .iter()
        .map(|id| id.trim().to_string())
        .filter(|id| !id.is_empty())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let query = match table {
        "tags" => "SELECT COUNT(*) FROM tags WHERE library_id = $1 AND id = ANY($2)",
        "folders" => "SELECT COUNT(*) FROM folders WHERE library_id = $1 AND id = ANY($2)",
        _ => {
            return Err(AppError::Internal(anyhow::anyhow!(
                "invalid relation table"
            )))
        }
    };
    let count: i64 = sqlx::query_scalar(query)
        .bind(library_id)
        .bind(&ids)
        .fetch_one(&mut **tx)
        .await?;
    if count != ids.len() as i64 {
        return Err(AppError::BadRequest(format!(
            "one or more {table} were not found"
        )));
    }
    Ok(())
}

async fn replace_relations(
    tx: &mut Transaction<'_, Postgres>,
    table: &str,
    relation_column: &str,
    asset_id: &str,
    relation_ids: &[String],
) -> AppResult<()> {
    let (delete_query, insert_query) = match (table, relation_column) {
        ("asset_tags", "tag_id") => (
            "DELETE FROM asset_tags WHERE asset_id = $1",
            "INSERT INTO asset_tags (asset_id, tag_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
        ),
        ("asset_folders", "folder_id") => (
            "DELETE FROM asset_folders WHERE asset_id = $1",
            "INSERT INTO asset_folders (asset_id, folder_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
        ),
        _ => return Err(AppError::Internal(anyhow::anyhow!("invalid asset relation"))),
    };
    sqlx::query(delete_query)
        .bind(asset_id)
        .execute(&mut **tx)
        .await?;
    let mut seen = HashSet::new();
    for relation_id in relation_ids
        .iter()
        .map(|id| id.trim())
        .filter(|id| !id.is_empty())
    {
        if seen.insert(relation_id) {
            sqlx::query(insert_query)
                .bind(asset_id)
                .bind(relation_id)
                .execute(&mut **tx)
                .await?;
        }
    }
    Ok(())
}
