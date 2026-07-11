use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListStorageRootsQuery {
    pub library_id: String,
}
