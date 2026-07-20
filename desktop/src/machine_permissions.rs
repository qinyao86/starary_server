use std::{
    fs,
    path::Path,
    process::{Command, Stdio},
};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;
const PERMISSIONS_MARKER: &str = ".machine-permissions-v1";

pub fn ensure_machine_data_permissions(data_home: &Path) -> Result<(), String> {
    fs::create_dir_all(data_home).map_err(|error| error.to_string())?;
    let marker = data_home.join(PERMISSIONS_MARKER);
    if marker.is_file() {
        return Ok(());
    }

    apply_machine_data_permissions(data_home)?;
    fs::write(&marker, b"1\n").map_err(|error| {
        format!(
            "machine data permissions were updated, but the marker could not be written at {}: {error}",
            marker.display(),
        )
    })
}

#[cfg(windows)]
fn apply_machine_data_permissions(data_home: &Path) -> Result<(), String> {
    let mut command = Command::new("icacls.exe");
    command
        .arg(data_home)
        .args(["/grant", "*S-1-5-32-545:(OI)(CI)M", "/T", "/C", "/Q"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    command.creation_flags(CREATE_NO_WINDOW);
    let output = command.output().map_err(|error| {
        format!(
            "failed to update shared data permissions for {}: {error}",
            data_home.display(),
        )
    })?;
    if output.status.success() {
        return Ok(());
    }

    let details = String::from_utf8_lossy(&output.stderr).trim().to_string();
    Err(format!(
        "shared server data is not writable by all local users; run the control center as administrator once to repair {}{}",
        data_home.display(),
        if details.is_empty() {
            String::new()
        } else {
            format!(": {details}")
        },
    ))
}

#[cfg(not(windows))]
fn apply_machine_data_permissions(_data_home: &Path) -> Result<(), String> {
    Ok(())
}
