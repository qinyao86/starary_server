use crate::{
    error::{AppError, AppResult},
    models::StorageRootKind,
};

pub fn validate_storage_root(kind: StorageRootKind, canonical_uri: &str) -> AppResult<()> {
    let value = canonical_uri.trim();
    if value.is_empty() {
        return Err(AppError::BadRequest("canonicalUri is required".to_string()));
    }

    if looks_like_personal_path(value) {
        return Err(AppError::BadRequest(
            "team indexed storage roots cannot point to a personal user directory".to_string(),
        ));
    }

    match kind {
        StorageRootKind::ServerFilesystem => Ok(()),
        StorageRootKind::Smb => {
            if value.starts_with("smb://") || value.starts_with("\\\\") {
                Ok(())
            } else {
                Err(AppError::BadRequest(
                    "smb storage roots must use smb:// or UNC canonicalUri".to_string(),
                ))
            }
        }
        StorageRootKind::S3 => {
            if value.starts_with("s3://") {
                Ok(())
            } else {
                Err(AppError::BadRequest(
                    "s3 storage roots must use s3:// canonicalUri".to_string(),
                ))
            }
        }
    }
}

pub fn validate_aliases(values: &[String]) -> AppResult<()> {
    for value in values {
        if looks_like_personal_path(value) {
            return Err(AppError::BadRequest(
                "team storage aliases cannot point to a personal user directory".to_string(),
            ));
        }
    }

    Ok(())
}

pub fn looks_like_personal_path(value: &str) -> bool {
    let normalized = value.replace('/', "\\").to_ascii_lowercase();
    normalized.contains("\\users\\")
        || normalized.starts_with("c:\\users\\")
        || normalized.contains("\\documents\\")
        || value.to_ascii_lowercase().starts_with("/users/")
        || value.to_ascii_lowercase().starts_with("/home/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_personal_windows_paths() {
        assert!(looks_like_personal_path(r"C:\Users\Alice\Assets"));
        assert!(looks_like_personal_path(r"P:\Users\Alice\Assets"));
    }

    #[test]
    fn accepts_shared_paths() {
        assert!(!looks_like_personal_path(r"\\nas\projects\Assets"));
        assert!(!looks_like_personal_path("smb://nas/projects/Assets"));
    }
}
