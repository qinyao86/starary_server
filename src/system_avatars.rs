use serde::Serialize;
use uuid::Uuid;

pub struct SystemAvatar {
    pub key: &'static str,
    pub gender: &'static str,
    pub bytes: &'static [u8],
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemAvatarOption {
    pub key: &'static str,
    pub gender: &'static str,
    pub url: String,
}

macro_rules! avatar {
    ($key:literal, $gender:literal) => {
        SystemAvatar {
            key: $key,
            gender: $gender,
            bytes: include_bytes!(concat!("system_avatars/", $key, ".svg")),
        }
    };
}

pub const AVATARS: &[SystemAvatar] = &[
    avatar!("male-01", "male"),
    avatar!("male-02", "male"),
    avatar!("male-03", "male"),
    avatar!("male-04", "male"),
    avatar!("male-05", "male"),
    avatar!("male-06", "male"),
    avatar!("male-07", "male"),
    avatar!("male-08", "male"),
    avatar!("male-09", "male"),
    avatar!("male-10", "male"),
    avatar!("male-11", "male"),
    avatar!("male-12", "male"),
    avatar!("male-13", "male"),
    avatar!("male-14", "male"),
    avatar!("male-15", "male"),
    avatar!("male-16", "male"),
    avatar!("male-17", "male"),
    avatar!("male-18", "male"),
    avatar!("male-19", "male"),
    avatar!("male-20", "male"),
    avatar!("female-01", "female"),
    avatar!("female-02", "female"),
    avatar!("female-03", "female"),
    avatar!("female-04", "female"),
    avatar!("female-05", "female"),
    avatar!("female-06", "female"),
    avatar!("female-07", "female"),
    avatar!("female-08", "female"),
    avatar!("female-09", "female"),
    avatar!("female-10", "female"),
    avatar!("female-11", "female"),
    avatar!("female-12", "female"),
    avatar!("female-13", "female"),
    avatar!("female-14", "female"),
    avatar!("female-15", "female"),
    avatar!("female-16", "female"),
    avatar!("female-17", "female"),
    avatar!("female-18", "female"),
    avatar!("female-19", "female"),
    avatar!("female-20", "female"),
];

pub fn get(key: &str) -> Option<&'static SystemAvatar> {
    AVATARS.iter().find(|avatar| avatar.key == key)
}

pub fn is_valid_key(key: &str) -> bool {
    get(key).is_some()
}

pub fn default_key_for_user(user_id: Uuid) -> &'static str {
    let index = user_id.as_bytes().iter().fold(0usize, |acc, byte| {
        acc.wrapping_mul(31).wrapping_add(*byte as usize)
    }) % AVATARS.len();
    AVATARS[index].key
}

pub fn url_for_key(key: &str) -> String {
    format!("/api/v1/avatars/system/{key}")
}

pub fn options() -> Vec<SystemAvatarOption> {
    AVATARS
        .iter()
        .map(|avatar| SystemAvatarOption {
            key: avatar.key,
            gender: avatar.gender,
            url: url_for_key(avatar.key),
        })
        .collect()
}
