use crate::models::{LibraryAccessMode, StorageRootKind};
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateLibraryRequest {
    #[serde(alias = "name")]
    pub display_name: String,
    pub icon_url: Option<String>,
    #[serde(default)]
    pub access_mode: Option<LibraryAccessMode>,
    pub default_storage_root: Option<CreateDefaultStorageRootRequest>,
    pub storage_binding: Option<StorageBindingRequest>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageBindingRequest {
    pub connection_id: uuid::Uuid,
    pub namespace: Option<String>,
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
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdateLibraryRequest {
    #[serde(alias = "name")]
    pub display_name: String,
    #[serde(default)]
    pub access_mode: Option<LibraryAccessMode>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UploadLibraryIconRequest {
    pub content_base64: String,
    pub size_bytes: Option<u64>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SetLibraryIconFromAssetRequest {
    pub asset_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateLibraryEnabledRequest {
    pub enabled: bool,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteLibraryRequest {
    #[serde(default)]
    pub delete_files: bool,
}

#[cfg(test)]
mod tests {
    use super::{CreateLibraryRequest, UpdateLibraryRequest};
    use crate::models::LibraryAccessMode;

    #[test]
    fn update_library_request_rejects_storage_binding() {
        let result = serde_json::from_str::<UpdateLibraryRequest>(
            r#"{"displayName":"Design","storageBinding":{"connectionId":"00000000-0000-0000-0000-000000000000"}}"#,
        );

        assert!(result.is_err());
    }

    #[test]
    fn update_library_request_rejects_icon_url() {
        let result = serde_json::from_str::<UpdateLibraryRequest>(
            r#"{"displayName":"Design","iconUrl":"https://example.com/icon.png"}"#,
        );

        assert!(result.is_err());
    }

    #[test]
    fn create_library_request_accepts_access_mode() {
        let result = serde_json::from_str::<CreateLibraryRequest>(
            r#"{"displayName":"Design","accessMode":"public"}"#,
        )
        .expect("request should deserialize");

        assert!(matches!(
            result.access_mode,
            Some(LibraryAccessMode::Public)
        ));
    }
}
