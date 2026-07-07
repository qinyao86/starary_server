use crate::models::Role;
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateUserRequest {
    pub email: String,
    pub password: String,
    pub display_name: Option<String>,
    #[serde(default)]
    pub role: Option<Role>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateUserRequest {
    pub display_name: Option<String>,
    #[serde(default)]
    pub role: Option<Role>,
    pub is_active: Option<bool>,
    pub password: Option<String>,
}
