use crate::models::StorageRootKind;
use serde::Deserialize;
use uuid::Uuid;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListStorageRootsQuery {
    pub library_id: Uuid,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateStorageRootRequest {
    pub library_id: Uuid,
    pub name: String,
    pub kind: StorageRootKind,
    pub canonical_uri: String,
    pub windows_unc_path: Option<String>,
    #[serde(default)]
    pub windows_mapped_drive_aliases: Vec<String>,
    pub macos_smb_url: Option<String>,
    #[serde(default)]
    pub macos_mount_aliases: Vec<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateStorageRootRequest {
    pub name: String,
    pub kind: StorageRootKind,
    pub canonical_uri: String,
    pub windows_unc_path: Option<String>,
    #[serde(default)]
    pub windows_mapped_drive_aliases: Vec<String>,
    pub macos_smb_url: Option<String>,
    #[serde(default)]
    pub macos_mount_aliases: Vec<String>,
    pub enabled: bool,
}
