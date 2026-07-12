use crate::models::Role;
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserLibraryMembershipInput {
    pub library_id: String,
    pub role: Role,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateUserRequest {
    pub email: String,
    pub password: String,
    pub display_name: Option<String>,
    #[serde(default)]
    pub role: Option<Role>,
    pub library_memberships: Option<Vec<UserLibraryMembershipInput>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateUserRequest {
    pub display_name: Option<String>,
    #[serde(default)]
    pub role: Option<Role>,
    pub is_active: Option<bool>,
    pub password: Option<String>,
    pub library_memberships: Option<Vec<UserLibraryMembershipInput>>,
}
