use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateFolderRequest {
    pub name: String,
    pub parent_id: Option<String>,
    pub icon: Option<String>,
    pub color: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateFolderImportPlanRequest {
    pub parent_id: Option<String>,
    pub folders: Vec<CreateFolderImportPlanItem>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateFolderImportPlanItem {
    pub key: String,
    pub parent_key: Option<String>,
    pub name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateFolderRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub cover_asset_id: Option<String>,
    pub clear_cover_asset_id: Option<bool>,
    pub smart_import_id: Option<String>,
    pub clear_smart_import_id: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReorderFoldersRequest {
    pub folder_ids: Vec<String>,
    pub parent_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTagGroupRequest {
    pub name: String,
    pub color: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTagGroupRequest {
    pub name: Option<String>,
    pub color: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTagRequest {
    pub name: String,
    pub group_id: Option<String>,
    pub color: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTagRequest {
    pub name: Option<String>,
    pub group_id: Option<String>,
    pub clear_group_id: Option<bool>,
    pub color: Option<String>,
    pub clear_color: Option<bool>,
    pub starred: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MoveTagsRequest {
    pub tag_ids: Vec<String>,
    pub group_id: Option<String>,
}
