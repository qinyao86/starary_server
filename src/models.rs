use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::{fmt, str::FromStr};
use uuid::Uuid;

mod library_structure;
mod presets;

pub use library_structure::*;
pub use presets::*;

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

    pub fn can_import_assets(self) -> bool {
        matches!(
            self,
            Role::Owner | Role::Admin | Role::LibraryManager | Role::Editor
        )
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

#[cfg(test)]
mod role_tests {
    use super::Role;

    #[test]
    fn import_assets_requires_editor_or_higher() {
        assert!(Role::Owner.can_import_assets());
        assert!(Role::Admin.can_import_assets());
        assert!(Role::LibraryManager.can_import_assets());
        assert!(Role::Editor.can_import_assets());
        assert!(!Role::Viewer.can_import_assets());
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StorageRootKind {
    ServerFilesystem,
    Smb,
    S3,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LibraryAccessMode {
    Public,
    Invite,
}

impl LibraryAccessMode {
    pub fn as_str(self) -> &'static str {
        match self {
            LibraryAccessMode::Public => "public",
            LibraryAccessMode::Invite => "invite",
        }
    }
}

impl fmt::Display for LibraryAccessMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for LibraryAccessMode {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "public" => Ok(LibraryAccessMode::Public),
            "invite" => Ok(LibraryAccessMode::Invite),
            other => Err(format!("unknown library access mode: {other}")),
        }
    }
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
            other => Err(format!("unknown workspace kind: {other}")),
        }
    }
}

#[derive(Debug, FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserRecord {
    pub id: Uuid,
    pub email: String,
    pub display_name: String,
    pub avatar_key: Option<String>,
    pub global_role: String,
    pub is_active: bool,
    #[sqlx(default)]
    pub last_login_at: Option<DateTime<Utc>>,
    #[sqlx(default)]
    pub last_seen_at: Option<DateTime<Utc>>,
    #[sqlx(default)]
    pub last_seen_library_id: Option<String>,
    #[sqlx(default)]
    pub last_seen_library_name: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, FromRow)]
pub struct UserWithPassword {
    pub id: Uuid,
    pub email: String,
    pub display_name: String,
    pub avatar_key: Option<String>,
    pub password_hash: String,
    pub global_role: String,
    pub is_active: bool,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryRecord {
    pub id: String,
    pub display_name: String,
    pub icon_url: Option<String>,
    pub enabled: bool,
    pub access_mode: String,
    pub created_by_user_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryWithRole {
    pub id: String,
    pub display_name: String,
    pub icon_url: Option<String>,
    pub enabled: bool,
    pub access_mode: String,
    pub storage_locked_at: Option<DateTime<Utc>>,
    pub current_user_role: Option<String>,
    pub is_member: bool,
    pub library_manager_names: Vec<String>,
    pub library_manager_user_ids: Vec<Uuid>,
    pub library_manager_avatar_keys: Vec<Option<String>>,
    pub member_names: Vec<String>,
    pub asset_count: i64,
    pub trash_asset_count: i64,
    pub untagged_asset_count: i64,
    pub uncategorized_asset_count: i64,
    pub favorite_asset_count: i64,
    pub folder_count: i64,
    pub tag_count: i64,
    pub total_size_bytes: i64,
    pub storage_root_count: i64,
    pub enabled_storage_root_count: i64,
    pub primary_storage_kind: Option<String>,
    pub primary_storage_connection_id: Option<Uuid>,
    pub primary_storage_connection_name: Option<String>,
    pub primary_storage_namespace: Option<String>,
    pub primary_storage_uri: Option<String>,
    pub primary_storage_windows_path: Option<String>,
    pub primary_storage_macos_path: Option<String>,
    pub created_by_user_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryStatusRecord {
    pub id: String,
    pub enabled: bool,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserLibraryMembershipRecord {
    pub library_id: String,
    pub library_name: String,
    pub role: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserWithMemberships {
    #[serde(flatten)]
    pub user: UserRecord,
    pub library_memberships: Vec<UserLibraryMembershipRecord>,
}

#[derive(Debug, FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageConnectionRecord {
    pub id: Uuid,
    pub name: String,
    pub kind: String,
    pub canonical_uri: String,
    pub windows_unc_path: Option<String>,
    pub windows_mapped_drive_aliases: serde_json::Value,
    pub macos_smb_url: Option<String>,
    pub macos_mount_aliases: serde_json::Value,
    pub enabled: bool,
    pub is_default: bool,
    pub library_count: i64,
    pub library_names: Vec<String>,
    pub asset_count: i64,
    pub total_size_bytes: i64,
    pub created_by_user_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageRootRecord {
    pub id: Uuid,
    pub library_id: String,
    pub storage_connection_id: Uuid,
    pub storage_connection_name: String,
    pub namespace: String,
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
    pub library_id: String,
    pub user_id: Uuid,
    pub email: String,
    pub display_name: String,
    pub avatar_key: Option<String>,
    pub avatar_updated_at: DateTime<Utc>,
    pub role: String,
    pub imported_asset_count: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryContributorRecord {
    pub user_id: Uuid,
    pub email: String,
    pub display_name: String,
    pub avatar_key: Option<String>,
}

#[derive(Debug, FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetRecord {
    pub id: String,
    pub library_id: String,
    pub name: String,
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
    pub library_id: Option<String>,
    pub actor_user_id: Option<Uuid>,
    pub actor_display_name: Option<String>,
    pub actor_email: Option<String>,
    pub actor_avatar_key: Option<String>,
    pub action: String,
    pub target_type: String,
    pub target_id: Option<String>,
    pub target_name: Option<String>,
    pub details: serde_json::Value,
    pub created_at: DateTime<Utc>,
}
