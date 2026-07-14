use std::{fs, net::TcpListener, path::Path};

pub(super) const DEFAULT_SERVER_PORT: u16 = 3789;

pub(super) fn configured_server_port(data_home: &Path) -> Option<u16> {
    let bytes = fs::read(data_home.join("data").join("config").join("runtime.json")).ok()?;
    let value: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
    value
        .get("serverPort")?
        .as_u64()
        .and_then(|port| u16::try_from(port).ok())
}

pub(super) fn update_server_port(
    data_home: &Path,
    port: u16,
) -> Result<(), Box<dyn std::error::Error>> {
    let config_dir = data_home.join("data").join("config");
    fs::create_dir_all(&config_dir)?;
    let path = config_dir.join("runtime.json");
    let mut value = fs::read(&path)
        .ok()
        .and_then(|contents| serde_json::from_slice::<serde_json::Value>(&contents).ok())
        .unwrap_or_else(|| {
            serde_json::json!({
                "version": 1,
                "postgresPort": 54329,
                "databasePassword": nanoid::nanoid!(48),
                "jwtSecret": nanoid::nanoid!(64),
            })
        });
    let object = value
        .as_object_mut()
        .ok_or("runtime configuration must be a JSON object")?;
    object.insert("serverPort".to_string(), serde_json::json!(port));
    fs::write(path, serde_json::to_vec_pretty(&value)?)?;
    Ok(())
}

pub(super) fn is_port_available(port: u16) -> bool {
    TcpListener::bind(("0.0.0.0", port)).is_ok()
}
