use crate::models::Role;
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpsertMemberRequest {
    pub role: Role,
}
