use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderRecord {
    pub id: String,
    pub parent_id: Option<String>,
    pub name: String,
    pub description: String,
    pub icon: String,
    pub color: String,
    pub asset_count: i64,
    pub cover_asset_id: Option<Uuid>,
    pub cover_asset: Option<serde_json::Value>,
    pub smart_import_id: Option<String>,
    pub sort_order: i64,
    pub created_by_user_id: Option<Uuid>,
    pub updated_by_user_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TagRecord {
    pub id: String,
    pub group_id: Option<String>,
    pub name: String,
    pub color: Option<String>,
    pub starred: bool,
    pub asset_count: i64,
    pub sort_order: i64,
    pub created_by_user_id: Option<Uuid>,
    pub updated_by_user_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TagGroupRecord {
    pub id: String,
    pub name: String,
    pub color: String,
    pub tag_count: i64,
    pub used_tag_count: i64,
    pub sort_order: i64,
    pub created_by_user_id: Option<Uuid>,
    pub updated_by_user_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
