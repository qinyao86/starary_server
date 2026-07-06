use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::{fmt, str::FromStr};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Owner,
    Admin,
    LibraryManager,
    Editor,
    Viewer,
}

impl Role {
    pub fn as_str(self) -> &'static str {
        match self {
            Role::Owner => "owner",
            Role::Admin => "admin",
            Role::LibraryManager => "library_manager",
            Role::Editor => "editor",
            Role::Viewer => "viewer",
        }
    }

    pub fn can_manage_server(self) -> bool {
        matches!(self, Role::Owner | Role::Admin)
    }

    pub fn can_create_library(self) -> bool {
        matches!(self, Role::Owner | Role::Admin)
    }

    pub fn can_manage_library(self) -> bool {
        matches!(self, Role::Owner | Role::Admin | Role::LibraryManager)
    }
}

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Role {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "owner" => Ok(Role::Owner),
            "admin" => Ok(Role::Admin),
            "library_manager" => Ok(Role::LibraryManager),
            "editor" => Ok(Role::Editor),
            "viewer" => Ok(Role::Viewer),
            other => Err(format!("unknown role: {other}")),
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StorageRootKind {
    ServerFilesystem,
    Smb,
    S3,
}

impl StorageRootKind {
    pub fn as_str(self) -> &'static str {
        match self {
            StorageRootKind::ServerFilesystem => "server_filesystem",
            StorageRootKind::Smb => "smb",
            StorageRootKind::S3 => "s3",
        }
    }
}

impl fmt::Display for StorageRootKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for StorageRootKind {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "server_filesystem" => Ok(StorageRootKind::ServerFilesystem),
            "smb" => Ok(StorageRootKind::Smb),
            "s3" => Ok(StorageRootKind::S3),
            other => Err(format!("unknown storage root kind: {other}")),
        }
    }
}

#[derive(Debug, FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserRecord {
    pub id: Uuid,
    pub email: String,
    pub display_name: String,
    pub global_role: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, FromRow)]
pub struct UserWithPassword {
    pub id: Uuid,
    pub email: String,
    pub display_name: String,
    pub password_hash: String,
    pub global_role: String,
    pub is_active: bool,
}

#[derive(Debug, FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamLibraryRecord {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub created_by_user_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryWithRole {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub current_user_role: String,
    pub creator_name: String,
    pub member_names: Vec<String>,
    pub asset_count: i64,
    pub folder_count: i64,
    pub tag_count: i64,
    pub total_size_bytes: i64,
    pub created_by_user_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageRootRecord {
    pub id: Uuid,
    pub library_id: Uuid,
    pub name: String,
    pub kind: String,
    pub canonical_uri: String,
    pub windows_unc_path: Option<String>,
    pub windows_mapped_drive_aliases: serde_json::Value,
    pub macos_smb_url: Option<String>,
    pub macos_mount_aliases: serde_json::Value,
    pub enabled: bool,
    pub created_by_user_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryMemberRecord {
    pub library_id: Uuid,
    pub user_id: Uuid,
    pub email: String,
    pub display_name: String,
    pub role: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetRecord {
    pub id: Uuid,
    pub library_id: Uuid,
    pub title: String,
    pub asset_kind: String,
    pub import_mode: String,
    pub storage_key: Option<String>,
    pub storage_root_id: Option<Uuid>,
    pub relative_path: Option<String>,
    pub metadata: serde_json::Value,
    pub created_by_user_id: Uuid,
    pub imported_by_user_id: Option<Uuid>,
    pub updated_by_user_id: Option<Uuid>,
    pub deleted_by_user_id: Option<Uuid>,
    pub restored_by_user_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub imported_at: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub restored_at: Option<DateTime<Utc>>,
}

#[derive(Debug, FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivityLogRecord {
    pub id: Uuid,
    pub library_id: Option<Uuid>,
    pub actor_user_id: Option<Uuid>,
    pub action: String,
    pub target_type: String,
    pub target_id: Option<Uuid>,
    pub details: serde_json::Value,
    pub created_at: DateTime<Utc>,
}
