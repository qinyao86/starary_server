use crate::{
    error::{AppError, AppResult},
    models::StorageRootKind,
};
use std::{
    fs,
    net::{IpAddr, ToSocketAddrs},
};

#[derive(Clone, Debug)]
pub struct ResolvedStorageLocation {
    pub canonical_uri: String,
    pub windows_unc_path: Option<String>,
    pub macos_smb_url: Option<String>,
}

#[derive(Clone, Debug)]
struct UncPathParts {
    host: String,
    share: String,
    segments: Vec<String>,
}

#[cfg(test)]
pub fn validate_storage_root(kind: StorageRootKind, canonical_uri: &str) -> AppResult<()> {
    validate_storage_root_with_policy(kind, canonical_uri, false)
}

pub fn validate_storage_root_with_policy(
    kind: StorageRootKind,
    canonical_uri: &str,
    allow_personal_paths: bool,
) -> AppResult<()> {
    let value = canonical_uri.trim();
    if value.is_empty() {
        return Err(AppError::BadRequest("canonicalUri is required".to_string()));
    }

    if !allow_personal_paths && looks_like_personal_path(value) {
        return Err(AppError::BadRequest(
            "team workspaces cannot point to a personal user directory".to_string(),
        ));
    }

    match kind {
        StorageRootKind::ServerFilesystem => Ok(()),
        StorageRootKind::Smb => {
            if value.starts_with("smb://")
                || value.starts_with("\\\\")
                || split_windows_drive_path(value).is_some()
            {
                Ok(())
            } else {
                Err(AppError::BadRequest(
                    "shared folder workspaces must use smb://, UNC, or a mapped Windows drive path"
                        .to_string(),
                ))
            }
        }
        StorageRootKind::S3 => {
            if value.starts_with("s3://") {
                Ok(())
            } else {
                Err(AppError::BadRequest(
                    "object storage workspaces must use s3:// standard location".to_string(),
                ))
            }
        }
    }
}

#[cfg(test)]
pub fn resolve_storage_location(
    kind: StorageRootKind,
    canonical_uri: &str,
    windows_unc_path: Option<String>,
    macos_smb_url: Option<String>,
) -> AppResult<ResolvedStorageLocation> {
    resolve_storage_location_with_policy(
        kind,
        canonical_uri,
        windows_unc_path,
        macos_smb_url,
        false,
    )
}

pub fn resolve_storage_location_with_policy(
    kind: StorageRootKind,
    canonical_uri: &str,
    windows_unc_path: Option<String>,
    macos_smb_url: Option<String>,
    allow_personal_paths: bool,
) -> AppResult<ResolvedStorageLocation> {
    let canonical_uri = trim_storage_location(canonical_uri);
    validate_storage_root_with_policy(kind, &canonical_uri, allow_personal_paths)?;

    if kind == StorageRootKind::Smb {
        return resolve_smb_storage_location(
            &canonical_uri,
            windows_unc_path,
            macos_smb_url,
            allow_personal_paths,
        );
    }

    let windows_unc_path = normalize_optional_storage_location(windows_unc_path)
        .or_else(|| canonical_uri_to_windows_path(kind, &canonical_uri));
    let macos_smb_url = normalize_optional_storage_location(macos_smb_url)
        .or_else(|| canonical_uri_to_macos_path(kind, &canonical_uri));

    if let Some(value) = &windows_unc_path {
        validate_aliases_with_policy(std::slice::from_ref(value), allow_personal_paths)?;
    }
    if let Some(value) = &macos_smb_url {
        validate_aliases_with_policy(std::slice::from_ref(value), allow_personal_paths)?;
    }

    Ok(ResolvedStorageLocation {
        canonical_uri,
        windows_unc_path,
        macos_smb_url,
    })
}

#[cfg(test)]
pub fn resolve_library_storage_namespace(
    kind: StorageRootKind,
    root_uri: &str,
    library_storage_id: &str,
    windows_unc_root: Option<String>,
    macos_smb_root: Option<String>,
) -> AppResult<ResolvedStorageLocation> {
    resolve_library_storage_namespace_with_policy(
        kind,
        root_uri,
        library_storage_id,
        windows_unc_root,
        macos_smb_root,
        false,
    )
}

#[cfg(test)]
pub fn resolve_library_storage_namespace_with_policy(
    kind: StorageRootKind,
    root_uri: &str,
    library_storage_id: &str,
    windows_unc_root: Option<String>,
    macos_smb_root: Option<String>,
    allow_personal_paths: bool,
) -> AppResult<ResolvedStorageLocation> {
    validate_library_storage_id(library_storage_id)?;
    resolve_storage_namespace_with_policy(
        kind,
        root_uri,
        library_storage_id,
        windows_unc_root,
        macos_smb_root,
        allow_personal_paths,
    )
}

pub fn resolve_storage_namespace_with_policy(
    kind: StorageRootKind,
    root_uri: &str,
    namespace: &str,
    windows_unc_root: Option<String>,
    macos_smb_root: Option<String>,
    allow_personal_paths: bool,
) -> AppResult<ResolvedStorageLocation> {
    if namespace.is_empty() {
        return resolve_storage_location_with_policy(
            kind,
            root_uri,
            windows_unc_root,
            macos_smb_root,
            allow_personal_paths,
        );
    }
    let canonical_uri = append_storage_segment(root_uri, namespace);
    let windows_unc_path = normalize_optional_storage_location(windows_unc_root)
        .map(|value| append_storage_segment(&value, namespace));
    let macos_smb_url = normalize_optional_storage_location(macos_smb_root)
        .map(|value| append_storage_segment(&value, namespace));

    resolve_storage_location_with_policy(
        kind,
        &canonical_uri,
        windows_unc_path,
        macos_smb_url,
        allow_personal_paths,
    )
}

pub fn normalize_storage_namespace(value: Option<&str>, library_id: &str) -> AppResult<String> {
    let value = value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(library_id);
    let normalized = value.replace('\\', "/").trim_matches('/').to_string();
    if normalized.is_empty()
        || normalized.starts_with('.')
        || normalized
            .split('/')
            .any(|segment| segment.is_empty() || segment == "." || segment == "..")
        || normalized.contains(':')
    {
        return Err(AppError::BadRequest(
            "storage namespace must be a relative folder path without dot segments".to_string(),
        ));
    }
    Ok(normalized)
}

pub fn normalize_existing_storage_namespace(value: &str) -> AppResult<String> {
    if value.trim().is_empty() {
        return Ok(String::new());
    }
    normalize_storage_namespace(Some(value), "unused")
}

pub fn storage_locations_overlap(first: &str, second: &str) -> bool {
    let first = storage_identity(first);
    let second = storage_identity(second);
    first == second
        || first
            .strip_prefix(&second)
            .is_some_and(|suffix| suffix.starts_with('/'))
        || second
            .strip_prefix(&first)
            .is_some_and(|suffix| suffix.starts_with('/'))
}

pub fn storage_identity(value: &str) -> String {
    value
        .trim()
        .replace('\\', "/")
        .trim_end_matches('/')
        .to_lowercase()
}

#[cfg(test)]
fn validate_library_storage_id(value: &str) -> AppResult<()> {
    use crate::ids::is_prefixed_id;
    if !is_prefixed_id(value, "lib_") {
        return Err(AppError::BadRequest(
            "library storage folder must use the lib_ plus 12 character format".to_string(),
        ));
    }

    Ok(())
}

pub fn ensure_storage_location_exists(
    kind: StorageRootKind,
    location: &ResolvedStorageLocation,
) -> AppResult<()> {
    match kind {
        StorageRootKind::S3 => Ok(()),
        StorageRootKind::ServerFilesystem => ensure_directory_exists(&location.canonical_uri),
        StorageRootKind::Smb => {
            if cfg!(windows) {
                let path = location.windows_unc_path.as_deref().ok_or_else(|| {
                    AppError::BadRequest(
                        "shared folder location cannot be resolved to a Windows path".to_string(),
                    )
                })?;
                ensure_directory_exists(path)
            } else {
                Err(AppError::BadRequest(
                    "shared folder workspaces require a Windows server or a mounted server filesystem path for existence checks".to_string(),
                ))
            }
        }
    }
}

pub fn ensure_storage_namespace_exists(
    kind: StorageRootKind,
    location: &ResolvedStorageLocation,
) -> AppResult<()> {
    match kind {
        StorageRootKind::S3 => Ok(()),
        StorageRootKind::ServerFilesystem => create_directory_namespace(&location.canonical_uri),
        StorageRootKind::Smb => {
            if cfg!(windows) {
                let path = location.windows_unc_path.as_deref().ok_or_else(|| {
                    AppError::BadRequest(
                        "shared folder location cannot be resolved to a Windows path".to_string(),
                    )
                })?;
                create_directory_namespace(path)
            } else {
                Err(AppError::BadRequest(
                    "shared folder namespace creation requires a server-side mounted filesystem path on this operating system".to_string(),
                ))
            }
        }
    }
}

pub fn validate_aliases_with_policy(
    values: &[String],
    allow_personal_paths: bool,
) -> AppResult<()> {
    for value in values {
        if !allow_personal_paths && looks_like_personal_path(value) {
            return Err(AppError::BadRequest(
                "team workspace aliases cannot point to a personal user directory".to_string(),
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

fn resolve_smb_storage_location(
    canonical_uri: &str,
    windows_unc_path: Option<String>,
    macos_smb_url: Option<String>,
    allow_personal_paths: bool,
) -> AppResult<ResolvedStorageLocation> {
    let standard_unc_path = resolve_smb_input_to_standard_unc(canonical_uri)?;
    let standard_smb_url = unc_to_smb_url(&standard_unc_path).ok_or_else(|| {
        AppError::BadRequest("shared folder location cannot be converted to smb://".to_string())
    })?;

    let windows_unc_path = match normalize_optional_storage_location(windows_unc_path) {
        Some(value) => Some(resolve_smb_input_to_standard_unc(&value)?),
        None => Some(standard_unc_path),
    };

    let macos_smb_url = match normalize_optional_storage_location(macos_smb_url) {
        Some(value) => {
            let unc_path = resolve_smb_input_to_standard_unc(&value)?;
            Some(unc_to_smb_url(&unc_path).ok_or_else(|| {
                AppError::BadRequest(
                    "macOS shared folder location cannot be converted to smb://".to_string(),
                )
            })?)
        }
        None => Some(standard_smb_url.clone()),
    };

    if let Some(value) = &windows_unc_path {
        validate_aliases_with_policy(std::slice::from_ref(value), allow_personal_paths)?;
    }
    if let Some(value) = &macos_smb_url {
        validate_aliases_with_policy(std::slice::from_ref(value), allow_personal_paths)?;
    }

    Ok(ResolvedStorageLocation {
        canonical_uri: standard_smb_url,
        windows_unc_path,
        macos_smb_url,
    })
}

fn ensure_directory_exists(path: &str) -> AppResult<()> {
    let metadata = fs::metadata(path).map_err(|error| {
        AppError::BadRequest(format!(
            "workspace path does not exist or cannot be accessed: {path}: {error}"
        ))
    })?;

    if !metadata.is_dir() {
        return Err(AppError::BadRequest(format!(
            "workspace path must be a directory: {path}"
        )));
    }

    Ok(())
}

fn create_directory_namespace(path: &str) -> AppResult<()> {
    fs::create_dir_all(path).map_err(|error| {
        AppError::BadRequest(format!(
            "could not create library storage folder at {path}: {error}"
        ))
    })
}

fn normalize_optional_storage_location(value: Option<String>) -> Option<String> {
    value
        .map(|value| trim_storage_location(&value))
        .filter(|value| !value.is_empty())
}

fn trim_storage_location(value: &str) -> String {
    value
        .trim()
        .trim_end_matches(|character| character == '\\' || character == '/')
        .to_string()
}

fn append_storage_segment(root: &str, segment: &str) -> String {
    let root = trim_storage_location(root);
    if root.starts_with("smb://") || root.starts_with("s3://") {
        format!("{root}/{segment}")
    } else if root.starts_with("\\\\") || root.contains('\\') {
        format!("{root}\\{segment}")
    } else {
        format!("{root}/{segment}")
    }
}

fn canonical_uri_to_windows_path(kind: StorageRootKind, canonical_uri: &str) -> Option<String> {
    match kind {
        StorageRootKind::Smb => {
            if canonical_uri.starts_with("\\\\") {
                Some(canonical_uri.to_string())
            } else if canonical_uri.starts_with("smb://") {
                smb_url_to_unc(canonical_uri)
            } else {
                None
            }
        }
        StorageRootKind::ServerFilesystem => {
            if canonical_uri.contains('\\') || has_windows_drive_prefix(canonical_uri) {
                Some(canonical_uri.to_string())
            } else {
                None
            }
        }
        StorageRootKind::S3 => Some(canonical_uri.to_string()),
    }
}

fn canonical_uri_to_macos_path(kind: StorageRootKind, canonical_uri: &str) -> Option<String> {
    match kind {
        StorageRootKind::Smb => {
            if canonical_uri.starts_with("smb://") {
                Some(canonical_uri.to_string())
            } else if canonical_uri.starts_with("\\\\") {
                unc_to_smb_url(canonical_uri)
            } else {
                None
            }
        }
        StorageRootKind::ServerFilesystem => {
            if canonical_uri.starts_with('/') {
                Some(canonical_uri.to_string())
            } else {
                None
            }
        }
        StorageRootKind::S3 => Some(canonical_uri.to_string()),
    }
}

fn resolve_smb_input_to_standard_unc(value: &str) -> AppResult<String> {
    let unc_path = resolve_smb_input_to_unc(value)?;
    standardize_unc_host_to_ip(&unc_path)
}

fn resolve_smb_input_to_unc(value: &str) -> AppResult<String> {
    let value = trim_storage_location(value);
    if value.starts_with("smb://") {
        let unc_path = smb_url_to_unc(&value).ok_or_else(|| {
            AppError::BadRequest(
                "shared folder smb:// location must include a host and share".to_string(),
            )
        })?;
        return normalize_unc_path(&unc_path);
    }

    if value.starts_with("\\\\") {
        return normalize_unc_path(&value);
    }

    if split_windows_drive_path(&value).is_some() {
        return resolve_windows_mapped_drive_to_unc(&value);
    }

    Err(AppError::BadRequest(
        "shared folder workspaces must use smb://, UNC, or a mapped Windows drive path".to_string(),
    ))
}

fn normalize_unc_path(value: &str) -> AppResult<String> {
    let parts = parse_unc_path(value)?;
    Ok(format_unc_path(&parts.host, &parts.share, &parts.segments))
}

fn standardize_unc_host_to_ip(value: &str) -> AppResult<String> {
    let parts = parse_unc_path(value)?;
    let host = resolve_host_to_ip(&parts.host)?;
    Ok(format_unc_path(&host, &parts.share, &parts.segments))
}

fn parse_unc_path(value: &str) -> AppResult<UncPathParts> {
    let normalized = value.replace('/', "\\");
    let rest = normalized.trim_start_matches('\\').trim_matches('\\');
    let parts = rest
        .split('\\')
        .filter(|part| !part.is_empty())
        .map(ToString::to_string)
        .collect::<Vec<_>>();

    if parts.len() < 2 {
        return Err(AppError::BadRequest(
            "UNC shared folder location must include a host and share".to_string(),
        ));
    }

    Ok(UncPathParts {
        host: parts[0].clone(),
        share: parts[1].clone(),
        segments: parts[2..].to_vec(),
    })
}

fn format_unc_path(host: &str, share: &str, segments: &[String]) -> String {
    let mut path = format!("\\\\{host}\\{share}");
    for segment in segments {
        path.push('\\');
        path.push_str(segment);
    }
    path
}

fn format_smb_url(host: &str, share: &str, segments: &[String]) -> String {
    let mut url = format!("smb://{host}/{share}");
    for segment in segments {
        url.push('/');
        url.push_str(segment);
    }
    url
}

fn smb_url_to_unc(value: &str) -> Option<String> {
    let rest = value.strip_prefix("smb://")?.trim_matches('/');
    if rest.is_empty() {
        return None;
    }

    let mut parts = rest.split('/').filter(|part| !part.is_empty());
    let host = parts.next()?;
    let share = parts.next()?;
    let segments = parts.map(ToString::to_string).collect::<Vec<_>>();

    Some(format_unc_path(host, share, &segments))
}

fn unc_to_smb_url(value: &str) -> Option<String> {
    let parts = parse_unc_path(value).ok()?;
    Some(format_smb_url(&parts.host, &parts.share, &parts.segments))
}

fn resolve_host_to_ip(host: &str) -> AppResult<String> {
    let host = host.trim_matches(|character| character == '[' || character == ']');
    if let Ok(ip_address) = host.parse::<IpAddr>() {
        return Ok(ip_address.to_string());
    }

    let addresses = (host, 445).to_socket_addrs().map_err(|error| {
        AppError::BadRequest(format!(
            "could not resolve shared folder host '{host}' to an IP address: {error}"
        ))
    })?;

    let mut first_ip = None;
    let mut first_ipv4 = None;
    for address in addresses {
        let ip = address.ip();
        if first_ip.is_none() {
            first_ip = Some(ip);
        }
        if ip.is_ipv4() {
            first_ipv4 = Some(ip);
            break;
        }
    }

    let ip = first_ipv4.or(first_ip).ok_or_else(|| {
        AppError::BadRequest(format!(
            "could not resolve shared folder host '{host}' to an IP address"
        ))
    })?;

    if ip.is_ipv6() {
        return Err(AppError::BadRequest(format!(
            "shared folder host '{host}' resolved to IPv6, which is not supported for workspace standard paths yet"
        )));
    }

    Ok(ip.to_string())
}

fn has_windows_drive_prefix(value: &str) -> bool {
    split_windows_drive_path(value).is_some()
}

fn split_windows_drive_path(value: &str) -> Option<(char, String)> {
    let bytes = value.as_bytes();
    if bytes.len() < 2 || bytes[1] != b':' || !bytes[0].is_ascii_alphabetic() {
        return None;
    }

    let drive = bytes[0] as char;
    let remainder = value[2..]
        .trim_start_matches(|character| character == '\\' || character == '/')
        .to_string();

    Some((drive, remainder))
}

#[cfg(windows)]
fn resolve_windows_mapped_drive_to_unc(value: &str) -> AppResult<String> {
    let (drive, remainder) = split_windows_drive_path(value)
        .ok_or_else(|| AppError::BadRequest("Windows mapped drive path is invalid".to_string()))?;
    let remote_root = query_windows_mapped_drive_remote_path(drive)?;

    if remainder.is_empty() {
        return normalize_unc_path(&remote_root);
    }

    append_segments_to_unc(&remote_root, &remainder)
}

#[cfg(not(windows))]
fn resolve_windows_mapped_drive_to_unc(_value: &str) -> AppResult<String> {
    Err(AppError::BadRequest(
        "Windows mapped drive paths can only be resolved by a Windows server".to_string(),
    ))
}

#[cfg(windows)]
fn query_windows_mapped_drive_remote_path(drive: char) -> AppResult<String> {
    use windows_sys::Win32::{
        Foundation::{ERROR_MORE_DATA, ERROR_NOT_CONNECTED, NO_ERROR},
        NetworkManagement::WNet::WNetGetConnectionW,
    };

    let drive_name = format!("{}:", drive.to_ascii_uppercase());
    let drive_name_wide = to_wide_null(&drive_name);
    let mut length = 260u32;
    let mut buffer = vec![0u16; length as usize];

    let mut result =
        unsafe { WNetGetConnectionW(drive_name_wide.as_ptr(), buffer.as_mut_ptr(), &mut length) };

    if result == ERROR_MORE_DATA {
        buffer.resize(length as usize, 0);
        result = unsafe {
            WNetGetConnectionW(drive_name_wide.as_ptr(), buffer.as_mut_ptr(), &mut length)
        };
    }

    if result != NO_ERROR {
        if result == ERROR_NOT_CONNECTED {
            return Err(AppError::BadRequest(format!(
                "Windows drive {drive_name} is not mapped to a network location"
            )));
        }

        return Err(AppError::BadRequest(format!(
            "could not resolve Windows mapped drive {drive_name}; system error {result}"
        )));
    }

    let remote_path = wide_null_to_string(&buffer);
    normalize_unc_path(&remote_path)
}

#[cfg(windows)]
fn append_segments_to_unc(root: &str, remainder: &str) -> AppResult<String> {
    let mut parts = parse_unc_path(root)?;
    parts.segments.extend(
        remainder
            .split(|character| character == '\\' || character == '/')
            .filter(|segment| !segment.is_empty())
            .map(ToString::to_string),
    );

    Ok(format_unc_path(&parts.host, &parts.share, &parts.segments))
}

#[cfg(windows)]
fn to_wide_null(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(windows)]
fn wide_null_to_string(value: &[u16]) -> String {
    let length = value
        .iter()
        .position(|character| *character == 0)
        .unwrap_or(value.len());
    String::from_utf16_lossy(&value[..length])
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
    fn personal_server_paths_require_the_development_policy() {
        let path = r"C:\Users\Alice\Assets";
        assert!(validate_storage_root(StorageRootKind::ServerFilesystem, path).is_err());
        assert!(
            validate_storage_root_with_policy(StorageRootKind::ServerFilesystem, path, true)
                .is_ok()
        );
    }

    #[test]
    fn accepts_shared_paths() {
        assert!(!looks_like_personal_path(r"\\192.168.3.13\projects\Assets"));
        assert!(!looks_like_personal_path(
            "smb://192.168.3.13/projects/Assets"
        ));
    }

    #[test]
    fn recognizes_mapped_drive_inputs() {
        assert_eq!(
            split_windows_drive_path("p:/libraries"),
            Some(('p', "libraries".to_string()))
        );
        assert_eq!(
            split_windows_drive_path(r"P:\libraries\team"),
            Some(('P', r"libraries\team".to_string()))
        );
    }

    #[test]
    fn resolves_unc_library_namespace_for_both_platforms() {
        let resolved = resolve_library_storage_namespace(
            StorageRootKind::Smb,
            r"\\192.168.3.13\libraries",
            "lib_000000000000",
            None,
            None,
        )
        .unwrap();

        assert_eq!(
            resolved.canonical_uri,
            "smb://192.168.3.13/libraries/lib_000000000000"
        );
        assert_eq!(
            resolved.windows_unc_path.as_deref(),
            Some(r"\\192.168.3.13\libraries\lib_000000000000")
        );
        assert_eq!(
            resolved.macos_smb_url.as_deref(),
            Some("smb://192.168.3.13/libraries/lib_000000000000")
        );
    }

    #[test]
    fn resolves_smb_library_namespace_for_both_platforms() {
        let resolved = resolve_library_storage_namespace(
            StorageRootKind::Smb,
            "smb://192.168.3.13/libraries/",
            "lib_000000000000",
            None,
            None,
        )
        .unwrap();

        assert_eq!(
            resolved.canonical_uri,
            "smb://192.168.3.13/libraries/lib_000000000000"
        );
        assert_eq!(
            resolved.windows_unc_path.as_deref(),
            Some(r"\\192.168.3.13\libraries\lib_000000000000")
        );
        assert_eq!(
            resolved.macos_smb_url.as_deref(),
            Some("smb://192.168.3.13/libraries/lib_000000000000")
        );
    }

    #[test]
    fn rejects_shared_folder_without_share() {
        let error =
            resolve_storage_location(StorageRootKind::Smb, "smb://192.168.3.13", None, None)
                .unwrap_err();

        assert!(error.to_string().contains("must include a host and share"));
    }

    #[test]
    fn requires_existing_server_filesystem_location() {
        let missing_path = std::env::temp_dir()
            .join(format!("starary-missing-{}", uuid::Uuid::new_v4()))
            .to_string_lossy()
            .to_string();
        let location = ResolvedStorageLocation {
            canonical_uri: missing_path,
            windows_unc_path: None,
            macos_smb_url: None,
        };

        let error = ensure_storage_location_exists(StorageRootKind::ServerFilesystem, &location)
            .unwrap_err();

        assert!(error.to_string().contains("does not exist"));
    }

    #[test]
    fn detects_overlapping_storage_locations_at_path_boundaries() {
        assert!(storage_locations_overlap(
            r"C:\Starary\LibraryA",
            "c:/starary/librarya"
        ));
        assert!(storage_locations_overlap(
            "smb://nas/libraries/team-a",
            "smb://NAS/libraries/team-a/assets"
        ));
        assert!(storage_locations_overlap(
            "s3://bucket/libraries/team-a/previews",
            "s3://bucket/libraries/team-a"
        ));
        assert!(!storage_locations_overlap(
            "smb://nas/libraries/team-a",
            "smb://nas/libraries/team-a-old"
        ));
        assert!(!storage_locations_overlap(
            r"C:\Starary\LibraryA",
            r"C:\Starary\LibraryB"
        ));
    }
}
