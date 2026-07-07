use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateLibraryRequest {
    #[serde(alias = "name")]
    pub display_name: String,
    pub description: Option<String>,
    pub icon_url: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateLibraryRequest {
    #[serde(alias = "name")]
    pub display_name: String,
    pub description: Option<String>,
    pub icon_url: Option<String>,
}
