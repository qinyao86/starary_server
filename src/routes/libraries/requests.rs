use crate::models::StorageRootKind;
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateLibraryRequest {
    #[serde(alias = "name")]
    pub display_name: String,
    pub icon_url: Option<String>,
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
    pub icon_url: Option<String>,
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
    use super::UpdateLibraryRequest;

    #[test]
    fn update_library_request_rejects_storage_binding() {
        let result = serde_json::from_str::<UpdateLibraryRequest>(
            r#"{"displayName":"Design","storageBinding":{"connectionId":"00000000-0000-0000-0000-000000000000"}}"#,
        );

        assert!(result.is_err());
    }
}
