use crate::models::StorageRootKind;
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateLibraryRequest {
    #[serde(alias = "name")]
    pub display_name: String,
    pub description: Option<String>,
    pub icon_url: Option<String>,
    pub default_storage_root: Option<CreateDefaultStorageRootRequest>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateDefaultStorageRootRequest {
    pub name: Option<String>,
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
pub struct UpdateLibraryRequest {
    #[serde(alias = "name")]
    pub display_name: String,
    pub description: Option<String>,
    pub icon_url: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateLibraryEnabledRequest {
    pub enabled: bool,
}
