use anyhow::{bail, Context};
use chrono::{DateTime, Local, NaiveTime, Utc};
use serde::{Deserialize, Serialize};
use std::{
    env, fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::Arc,
    time::Duration,
};
use tokio::sync::{watch, Mutex};
use url::Url;

const DEFAULT_BACKUP_TIME: &str = "02:00";
const DEFAULT_RETENTION_COUNT: usize = 30;

#[derive(Clone)]
pub struct BackupService {
    inner: Arc<BackupServiceInner>,
}

struct BackupServiceInner {
    backup_dir: PathBuf,
    config_path: PathBuf,
    pending_restore_path: PathBuf,
    pg_dump: PathBuf,
    database: DatabaseCommandEnv,
    operation_lock: Mutex<()>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupSettings {
    pub automatic_enabled: bool,
    pub automatic_time: String,
    pub retention_count: usize,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupStatus {
    pub available: bool,
    pub backup_dir: String,
    pub settings: BackupSettings,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupRecord {
    pub id: String,
    pub kind: BackupKind,
    pub size_bytes: u64,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BackupKind {
    Automatic,
    Manual,
    PreRestore,
    PreInitialize,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct PendingRestore {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    backup_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    source_path: Option<PathBuf>,
    requested_at: DateTime<Utc>,
}

#[derive(Clone)]
struct DatabaseCommandEnv {
    host: String,
    port: String,
    user: String,
    password: String,
    database: String,
}

impl BackupService {
    pub fn new(app_home: &Path, storage_dir: &Path, database_url: &str) -> anyhow::Result<Self> {
        let data_home = storage_dir
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| app_home.join("data"));
        let backup_dir = data_home.join("backups");
        let config_dir = data_home.join("config");
        fs::create_dir_all(&backup_dir)
            .with_context(|| format!("failed to create {}", backup_dir.display()))?;
        fs::create_dir_all(&config_dir)
            .with_context(|| format!("failed to create {}", config_dir.display()))?;

        let bin_dir = backup_bin_dir(app_home);
        Ok(Self {
            inner: Arc::new(BackupServiceInner {
                backup_dir,
                config_path: config_dir.join("backup.json"),
                pending_restore_path: config_dir.join("pending-restore.json"),
                pg_dump: bin_dir.join(executable_name("pg_dump")),
                database: DatabaseCommandEnv::parse(database_url)?,
                operation_lock: Mutex::new(()),
            }),
        })
    }

    pub fn status(&self) -> anyhow::Result<BackupStatus> {
        Ok(BackupStatus {
            available: self.available(),
            backup_dir: self.inner.backup_dir.display().to_string(),
            settings: self.load_settings()?,
        })
    }

    pub fn available(&self) -> bool {
        self.inner.pg_dump.is_file()
    }

    pub fn load_settings(&self) -> anyhow::Result<BackupSettings> {
        BackupSettings::load_or_create(&self.inner.config_path)
    }

    pub async fn update_settings(&self, settings: BackupSettings) -> anyhow::Result<BackupStatus> {
        settings.validate()?;
        write_json_atomic(&self.inner.config_path, &settings)?;
        let _guard = self.inner.operation_lock.lock().await;
        self.prune_automatic(settings.retention_count)?;
        self.status()
    }

    pub fn list(&self) -> anyhow::Result<Vec<BackupRecord>> {
        let mut records = Vec::new();
        for entry in fs::read_dir(&self.inner.backup_dir)? {
            let entry = entry?;
            let path = entry.path();
            let Some((id, kind)) = backup_file_identity(&path) else {
                continue;
            };
            let metadata = entry.metadata()?;
            let modified = metadata
                .modified()
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
            records.push(BackupRecord {
                id,
                kind,
                size_bytes: metadata.len(),
                created_at: DateTime::<Utc>::from(modified),
            });
        }
        records.sort_by(|left, right| right.created_at.cmp(&left.created_at));
        Ok(records)
    }

    pub async fn create_manual(&self) -> anyhow::Result<BackupRecord> {
        self.create(BackupKind::Manual).await
    }

    pub async fn create_pre_initialize(&self) -> anyhow::Result<BackupRecord> {
        self.create(BackupKind::PreInitialize).await
    }

    pub async fn delete(&self, backup_id: &str) -> anyhow::Result<()> {
        let _guard = self.inner.operation_lock.lock().await;
        let path = self.backup_path(backup_id)?;
        fs::remove_file(&path).with_context(|| format!("failed to delete {}", path.display()))
    }

    pub fn backup_path(&self, backup_id: &str) -> anyhow::Result<PathBuf> {
        validate_backup_id(backup_id)?;
        let path = self.inner.backup_dir.join(backup_id);
        if !path.is_file() || backup_file_identity(&path).is_none() {
            bail!("backup not found")
        }
        Ok(path)
    }

    pub async fn queue_restore(&self, backup_id: &str) -> anyhow::Result<()> {
        let _guard = self.inner.operation_lock.lock().await;
        self.backup_path(backup_id)?;
        write_json_atomic(
            &self.inner.pending_restore_path,
            &PendingRestore {
                backup_id: Some(backup_id.to_string()),
                source_path: None,
                requested_at: Utc::now(),
            },
        )
    }

    pub fn uploaded_restore_paths(&self) -> anyhow::Result<(PathBuf, PathBuf)> {
        fs::create_dir_all(&self.inner.backup_dir)?;
        let id = format!(
            "starary-upload-restore-{}.dump",
            Local::now().format("%Y%m%d-%H%M%S-%3f")
        );
        let destination = self.inner.backup_dir.join(id);
        let partial = destination.with_extension("dump.partial");
        Ok((partial, destination))
    }

    pub async fn queue_uploaded_restore(&self, source_path: &Path) -> anyhow::Result<()> {
        let _guard = self.inner.operation_lock.lock().await;
        validate_uploaded_restore_source(source_path, &self.inner.backup_dir)?;
        write_json_atomic(
            &self.inner.pending_restore_path,
            &PendingRestore {
                backup_id: None,
                source_path: Some(source_path.to_path_buf()),
                requested_at: Utc::now(),
            },
        )
    }

    pub async fn run_scheduler(&self, mut shutdown: watch::Receiver<bool>) {
        loop {
            if *shutdown.borrow() {
                return;
            }
            if let Err(error) = self.run_due_automatic().await {
                tracing::error!(%error, "Automatic database backup failed");
            }
            tokio::select! {
                changed = shutdown.changed() => {
                    if changed.is_err() || *shutdown.borrow() {
                        return;
                    }
                }
                _ = tokio::time::sleep(Duration::from_secs(60)) => {}
            }
        }
    }

    pub fn apply_pending_restore(&self) -> anyhow::Result<bool> {
        if !self.inner.pending_restore_path.is_file() {
            return Ok(false);
        }
        let pending: PendingRestore =
            serde_json::from_slice(&fs::read(&self.inner.pending_restore_path)?)?;
        let (source, cleanup_source) = self.pending_restore_source(&pending)?;
        let recovery_id = format!(
            "starary-pre-restore-{}.dump",
            Local::now().format("%Y%m%d-%H%M%S-%3f")
        );
        let recovery = self.inner.backup_dir.join(&recovery_id);

        self.dump_sync(&recovery)
            .context("failed to create the pre-restore safety backup")?;
        tracing::info!(source = %source.display(), "Applying pending database restore");
        if let Err(error) = self.restore_sync(&source) {
            tracing::error!(%error, "Database restore failed; rolling back to pre-restore backup");
            self.restore_sync(&recovery)
                .context("database restore and automatic rollback both failed")?;
            fs::remove_file(&self.inner.pending_restore_path)?;
            if cleanup_source {
                let _ = fs::remove_file(&source);
            }
            return Err(error.context("database restore failed and was rolled back"));
        }
        fs::remove_file(&self.inner.pending_restore_path)?;
        if cleanup_source {
            let _ = fs::remove_file(&source);
        }
        Ok(true)
    }

    async fn run_due_automatic(&self) -> anyhow::Result<()> {
        if !self.available() {
            return Ok(());
        }
        let settings = self.load_settings()?;
        if !settings.automatic_enabled {
            return Ok(());
        }
        let scheduled_time = NaiveTime::parse_from_str(&settings.automatic_time, "%H:%M")?;
        let now = Local::now();
        if now.time() < scheduled_time
            || self.has_automatic_for_date(&now.format("%Y%m%d").to_string())?
        {
            return Ok(());
        }
        self.create(BackupKind::Automatic).await?;
        Ok(())
    }

    async fn create(&self, kind: BackupKind) -> anyhow::Result<BackupRecord> {
        if !self.available() {
            bail!("PostgreSQL backup tools are not available")
        }
        let _guard = self.inner.operation_lock.lock().await;
        let prefix = match kind {
            BackupKind::Automatic => "auto",
            BackupKind::Manual => "manual",
            BackupKind::PreRestore => "pre-restore",
            BackupKind::PreInitialize => "pre-initialize",
        };
        let id = format!(
            "starary-{prefix}-{}.dump",
            Local::now().format("%Y%m%d-%H%M%S-%3f")
        );
        let destination = self.inner.backup_dir.join(&id);
        let partial = destination.with_extension("dump.partial");
        let pg_dump = self.inner.pg_dump.clone();
        let database = self.inner.database.clone();
        let partial_for_command = partial.clone();
        let output = tokio::task::spawn_blocking(move || {
            let mut command = Command::new(pg_dump);
            database.apply(&mut command);
            command
                .arg("--format=custom")
                .arg("--compress=6")
                .arg("--no-owner")
                .arg("--no-privileges")
                .arg("--file")
                .arg(partial_for_command)
                .output()
        })
        .await??;
        if !output.status.success() {
            let _ = fs::remove_file(&partial);
            bail!("pg_dump failed: {}", command_error(&output))
        }
        fs::rename(&partial, &destination)
            .with_context(|| format!("failed to activate backup {}", destination.display()))?;
        if matches!(kind, BackupKind::Automatic) {
            self.prune_automatic(self.load_settings()?.retention_count)?;
        }
        self.list()?
            .into_iter()
            .find(|record| record.id == id)
            .context("created backup was not found")
    }

    fn dump_sync(&self, destination: &Path) -> anyhow::Result<()> {
        let partial = destination.with_extension("dump.partial");
        let mut command = Command::new(&self.inner.pg_dump);
        self.inner.database.apply(&mut command);
        let output = command
            .arg("--format=custom")
            .arg("--compress=6")
            .arg("--no-owner")
            .arg("--no-privileges")
            .arg("--file")
            .arg(&partial)
            .output()?;
        if !output.status.success() {
            let _ = fs::remove_file(&partial);
            bail!("pg_dump failed: {}", command_error(&output))
        }
        fs::rename(partial, destination)?;
        Ok(())
    }

    fn restore_sync(&self, source: &Path) -> anyhow::Result<()> {
        let bin_dir = self
            .inner
            .pg_dump
            .parent()
            .context("invalid pg_dump path")?;
        let dropdb = bin_dir.join(executable_name("dropdb"));
        let createdb = bin_dir.join(executable_name("createdb"));
        let pg_restore = bin_dir.join(executable_name("pg_restore"));
        for tool in [&dropdb, &createdb, &pg_restore] {
            if !tool.is_file() {
                bail!("PostgreSQL restore tool is missing: {}", tool.display())
            }
        }

        let mut drop_command = Command::new(dropdb);
        self.inner.database.apply(&mut drop_command);
        let output = drop_command
            .arg("--force")
            .arg("--maintenance-db=postgres")
            .arg(&self.inner.database.database)
            .output()?;
        ensure_command_success("dropdb", &output)?;

        let mut create_command = Command::new(createdb);
        self.inner.database.apply(&mut create_command);
        let output = create_command
            .arg("--maintenance-db=postgres")
            .arg(&self.inner.database.database)
            .output()?;
        ensure_command_success("createdb", &output)?;

        let mut restore_command = Command::new(pg_restore);
        self.inner.database.apply(&mut restore_command);
        let output = restore_command
            .arg("--exit-on-error")
            .arg("--no-owner")
            .arg("--no-privileges")
            .arg("--dbname")
            .arg(&self.inner.database.database)
            .arg(source)
            .output()?;
        ensure_command_success("pg_restore", &output)
    }

    fn has_automatic_for_date(&self, date: &str) -> anyhow::Result<bool> {
        Ok(self.list()?.iter().any(|record| {
            matches!(record.kind, BackupKind::Automatic)
                && record.id.starts_with(&format!("starary-auto-{date}-"))
        }))
    }

    fn prune_automatic(&self, retention_count: usize) -> anyhow::Result<()> {
        let automatic = self
            .list()?
            .into_iter()
            .filter(|record| matches!(record.kind, BackupKind::Automatic))
            .collect::<Vec<_>>();
        for record in automatic.into_iter().skip(retention_count) {
            fs::remove_file(self.inner.backup_dir.join(record.id))?;
        }
        Ok(())
    }

    fn pending_restore_source(&self, pending: &PendingRestore) -> anyhow::Result<(PathBuf, bool)> {
        if let Some(backup_id) = pending.backup_id.as_deref() {
            return Ok((self.backup_path(backup_id)?, false));
        }
        if let Some(source_path) = pending.source_path.as_deref() {
            validate_uploaded_restore_source(source_path, &self.inner.backup_dir)?;
            return Ok((source_path.to_path_buf(), true));
        }
        bail!("pending restore source is missing")
    }
}

impl BackupSettings {
    fn load_or_create(path: &Path) -> anyhow::Result<Self> {
        if path.is_file() {
            let contents = fs::read(path)?;
            let settings: Self = serde_json::from_slice(&contents)?;
            settings.validate()?;
            return Ok(settings);
        }
        let settings = Self {
            automatic_enabled: true,
            automatic_time: DEFAULT_BACKUP_TIME.to_string(),
            retention_count: DEFAULT_RETENTION_COUNT,
        };
        write_json_atomic(path, &settings)?;
        Ok(settings)
    }

    fn validate(&self) -> anyhow::Result<()> {
        if !(1..=365).contains(&self.retention_count) {
            bail!("automatic backup retention must be between 1 and 365")
        }
        NaiveTime::parse_from_str(&self.automatic_time, "%H:%M")
            .context("automatic backup time must use HH:MM")?;
        Ok(())
    }
}

impl DatabaseCommandEnv {
    fn parse(database_url: &str) -> anyhow::Result<Self> {
        let url = Url::parse(database_url).context("invalid PostgreSQL database URL")?;
        let database = url.path().trim_start_matches('/');
        if database.is_empty() {
            bail!("PostgreSQL database URL is missing a database name")
        }
        Ok(Self {
            host: url.host_str().unwrap_or("127.0.0.1").to_string(),
            port: url.port().unwrap_or(5432).to_string(),
            user: url.username().to_string(),
            password: url.password().unwrap_or_default().to_string(),
            database: database.to_string(),
        })
    }

    fn apply(&self, command: &mut Command) {
        command
            .env("PGHOST", &self.host)
            .env("PGPORT", &self.port)
            .env("PGUSER", &self.user)
            .env("PGPASSWORD", &self.password)
            .env("PGDATABASE", &self.database);
    }
}

fn backup_bin_dir(app_home: &Path) -> PathBuf {
    if let Some(value) = env::var_os("STARARY_POSTGRES_BIN_DIR") {
        return PathBuf::from(value);
    }
    let portable = app_home.join("postgresql").join("bin");
    if portable.is_dir() {
        portable
    } else {
        app_home
            .join("binaries")
            .join("windows-x64")
            .join("postgresql")
            .join("bin")
    }
}

fn backup_file_identity(path: &Path) -> Option<(String, BackupKind)> {
    let id = path.file_name()?.to_str()?.to_string();
    if !id.ends_with(".dump") {
        return None;
    }
    let kind = if id.starts_with("starary-auto-") {
        BackupKind::Automatic
    } else if id.starts_with("starary-manual-") {
        BackupKind::Manual
    } else if id.starts_with("starary-pre-restore-") {
        BackupKind::PreRestore
    } else if id.starts_with("starary-pre-initialize-") {
        BackupKind::PreInitialize
    } else {
        return None;
    };
    Some((id, kind))
}

fn validate_backup_id(value: &str) -> anyhow::Result<()> {
    if value.is_empty()
        || value.contains('/')
        || value.contains('\\')
        || value.contains("..")
        || Path::new(value).file_name().and_then(|name| name.to_str()) != Some(value)
    {
        bail!("invalid backup id")
    }
    Ok(())
}

fn validate_uploaded_restore_source(path: &Path, backup_dir: &Path) -> anyhow::Result<()> {
    if path.extension().and_then(|value| value.to_str()) != Some("dump") {
        bail!("restore source must be a .dump file")
    }
    if !path.is_file() {
        bail!("restore source was not found")
    }
    let backup_dir = backup_dir.canonicalize()?;
    let source = path.canonicalize()?;
    if !source.starts_with(&backup_dir) {
        bail!("restore source must be in the backup directory")
    }
    Ok(())
}

fn write_json_atomic(path: &Path, value: &impl Serialize) -> anyhow::Result<()> {
    let temporary = path.with_extension("json.tmp");
    fs::write(&temporary, serde_json::to_vec_pretty(value)?)?;
    if path.exists() {
        fs::remove_file(path)?;
    }
    fs::rename(temporary, path)?;
    Ok(())
}

fn command_error(output: &Output) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if stderr.is_empty() {
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    } else {
        stderr
    }
}

fn ensure_command_success(name: &str, output: &Output) -> anyhow::Result<()> {
    if output.status.success() {
        Ok(())
    } else {
        bail!("{name} failed: {}", command_error(output))
    }
}

fn executable_name(name: &str) -> String {
    if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_service(test_name: &str) -> (BackupService, PathBuf) {
        let root =
            env::temp_dir().join(format!("starary-backup-{test_name}-{}", nanoid::nanoid!()));
        let storage = root.join("data").join("storage");
        fs::create_dir_all(&storage).unwrap();
        let service = BackupService::new(
            &root,
            &storage,
            "postgresql://starary:password@127.0.0.1:5432/starary_team",
        )
        .unwrap();
        (service, root)
    }

    #[test]
    fn retention_prunes_only_old_automatic_backups() {
        let (service, root) = test_service("retention");
        for name in [
            "starary-auto-20260710-020000.dump",
            "starary-auto-20260711-020000.dump",
            "starary-auto-20260712-020000.dump",
            "starary-manual-20260701-120000.dump",
        ] {
            fs::write(service.inner.backup_dir.join(name), name).unwrap();
            std::thread::sleep(Duration::from_millis(5));
        }

        service.prune_automatic(2).unwrap();

        let remaining = service
            .list()
            .unwrap()
            .into_iter()
            .map(|record| record.id)
            .collect::<Vec<_>>();
        assert_eq!(remaining.len(), 3);
        assert!(remaining.iter().any(|id| id.starts_with("starary-manual-")));
        assert_eq!(
            remaining
                .iter()
                .filter(|id| id.starts_with("starary-auto-"))
                .count(),
            2
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn backup_ids_cannot_escape_the_backup_directory() {
        assert!(validate_backup_id("../runtime.json").is_err());
        assert!(validate_backup_id("folder\\backup.dump").is_err());
        assert!(validate_backup_id("starary-manual-20260712-120000.dump").is_ok());
    }
}
