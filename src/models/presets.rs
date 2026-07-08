use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::FromRow;

#[derive(Debug, FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetRecord {
    pub id: String,
    pub r#type: String,
    pub name: String,
    pub value: serde_json::Value,
    pub sort_order: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub asset_count: Option<i64>,
}
