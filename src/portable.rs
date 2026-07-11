use anyhow::{bail, Context};
use nanoid::nanoid;
use serde::{Deserialize, Serialize};
use std::{
    env, fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

const DATABASE_NAME: &str = "madlibrary_team";
const DATABASE_USER: &str = "madlibrary";
const DEFAULT_POSTGRES_PORT: u16 = 54329;

pub struct PortableRuntime {
    pub app_home: PathBuf,
    managed_postgres: Option<ManagedPostgres>,
}

impl PortableRuntime {
    pub fn prepare() -> anyhow::Result<Self> {
        let app_home = resolve_app_home()?;
        dotenvy::from_path(app_home.join(".env")).ok();

        let postgres_mode = PostgresMode::from_env()?;
        let database_url_is_set = env::var("MADLIBRARY_DATABASE_URL")
            .map(|value| !value.trim().is_empty())
            .unwrap_or(false);
        let managed_postgres = match postgres_mode {
            PostgresMode::Auto if database_url_is_set => None,
            PostgresMode::Auto | PostgresMode::Bundled => {
                Some(prepare_bundled_postgres(&app_home)?)
            }
            PostgresMode::External if database_url_is_set => None,
            PostgresMode::External => {
                bail!("MADLIBRARY_POSTGRES_MODE=external requires MADLIBRARY_DATABASE_URL")
            }
        };

        Ok(Self {
            app_home,
            managed_postgres,
        })
    }

    pub fn stop(&mut self) {
        if let Some(postgres) = self.managed_postgres.as_mut() {
            postgres.stop();
        }
    }
}

enum PostgresMode {
    Auto,
    Bundled,
    External,
}

impl PostgresMode {
    fn from_env() -> anyhow::Result<Self> {
        let value = env::var("MADLIBRARY_POSTGRES_MODE").unwrap_or_else(|_| "auto".to_string());
        match value.trim().to_ascii_lowercase().as_str() {
            "auto" => Ok(Self::Auto),
            "bundled" => Ok(Self::Bundled),
            "external" => Ok(Self::External),
            _ => bail!("MADLIBRARY_POSTGRES_MODE must be auto, bundled, or external"),
        }
    }
}

fn resolve_app_home() -> anyhow::Result<PathBuf> {
    if let Some(value) = env::var_os("MADLIBRARY_HOME") {
        return absolute_path(PathBuf::from(value));
    }

    if cfg!(debug_assertions) {
        return Ok(PathBuf::from(env!("CARGO_MANIFEST_DIR")));
    }

    let executable = env::current_exe().context("failed to locate the server executable")?;
    executable
        .parent()
        .map(Path::to_path_buf)
        .context("server executable has no parent directory")
}

fn absolute_path(path: PathBuf) -> anyhow::Result<PathBuf> {
    if path.is_absolute() {
        Ok(path)
    } else {
        Ok(env::current_dir()
            .context("failed to read the current directory")?
            .join(path))
    }
}

fn prepare_bundled_postgres(app_home: &Path) -> anyhow::Result<ManagedPostgres> {
    let postgres_home = app_home.join("postgresql");
    let postgres_executable = postgres_home.join("bin").join(executable_name("postgres"));
    if !postgres_executable.is_file() {
        bail!(
            "bundled PostgreSQL was requested but {} is missing; set MADLIBRARY_POSTGRES_MODE=external and MADLIBRARY_DATABASE_URL to use an external database",
            postgres_executable.display()
        );
    }

    let data_home = app_home.join("data");
    let config_dir = data_home.join("config");
    let logs_dir = data_home.join("logs");
    let storage_dir = data_home.join("storage");
    let postgres_data_dir = data_home.join("postgresql");
    for directory in [&config_dir, &logs_dir, &storage_dir] {
        fs::create_dir_all(directory)
            .with_context(|| format!("failed to create {}", directory.display()))?;
    }

    let runtime_config = PortableConfig::load_or_create(&config_dir.join("runtime.json"))?;
    let database_url = format!(
        "postgresql://{DATABASE_USER}:{}@127.0.0.1:{}/{DATABASE_NAME}",
        runtime_config.database_password, runtime_config.postgres_port
    );

    env::set_var("MADLIBRARY_DATABASE_URL", database_url);
    env::set_var("MADLIBRARY_STORAGE_DIR", &storage_dir);
    env::set_var("MADLIBRARY_ADMIN_ASSETS_DIR", app_home.join("admin-ui"));
    env::set_var("MADLIBRARY_DEPLOYMENT_MODE", "portable");
    env::set_var("MADLIBRARY_JWT_SECRET", &runtime_config.jwt_secret);

    tracing::info!("Starting bundled PostgreSQL...");
    ManagedPostgres::start(
        postgres_home,
        postgres_data_dir,
        logs_dir.join("postgresql.log"),
        config_dir,
        runtime_config,
    )
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PortableConfig {
    version: u32,
    postgres_port: u16,
    database_password: String,
    jwt_secret: String,
}

impl PortableConfig {
    fn load_or_create(path: &Path) -> anyhow::Result<Self> {
        if path.is_file() {
            let contents =
                fs::read(path).with_context(|| format!("failed to read {}", path.display()))?;
            let config: Self = serde_json::from_slice(&contents)
                .with_context(|| format!("failed to parse {}", path.display()))?;
            config.validate(path)?;
            return Ok(config);
        }

        let config = Self {
            version: 1,
            postgres_port: DEFAULT_POSTGRES_PORT,
            database_password: nanoid!(48),
            jwt_secret: nanoid!(64),
        };
        let contents = serde_json::to_vec_pretty(&config)?;
        fs::write(path, contents).with_context(|| format!("failed to write {}", path.display()))?;
        Ok(config)
    }

    fn validate(&self, path: &Path) -> anyhow::Result<()> {
        if self.version != 1
            || self.postgres_port == 0
            || self.database_password.len() < 32
            || self.jwt_secret.len() < 32
        {
            bail!(
                "portable runtime configuration is invalid: {}",
                path.display()
            );
        }
        Ok(())
    }
}

struct ManagedPostgres {
    pg_ctl: PathBuf,
    data_dir: PathBuf,
    should_stop: bool,
}

impl ManagedPostgres {
    fn start(
        postgres_home: PathBuf,
        data_dir: PathBuf,
        log_path: PathBuf,
        config_dir: PathBuf,
        config: PortableConfig,
    ) -> anyhow::Result<Self> {
        let bin_dir = postgres_home.join("bin");
        let pg_ctl = require_executable(&bin_dir, "pg_ctl")?;
        let initdb = require_executable(&bin_dir, "initdb")?;
        let createdb = require_executable(&bin_dir, "createdb")?;
        require_executable(&bin_dir, "postgres")?;

        let mut managed = Self {
            pg_ctl,
            data_dir,
            should_stop: false,
        };

        let new_cluster = !managed.data_dir.join("PG_VERSION").is_file();
        if new_cluster {
            managed.initialize(&initdb, &config_dir, &config.database_password)?;
        }

        if !managed.is_running()? {
            managed.start_server(&log_path, config.postgres_port)?;
        }
        managed.should_stop = true;

        managed.ensure_database(&createdb, config.postgres_port, &config.database_password)?;

        tracing::info!(
            port = config.postgres_port,
            data_dir = %managed.data_dir.display(),
            "Bundled PostgreSQL is ready"
        );
        Ok(managed)
    }

    fn initialize(
        &self,
        initdb: &Path,
        config_dir: &Path,
        database_password: &str,
    ) -> anyhow::Result<()> {
        if self.data_dir.exists() && self.data_dir.read_dir()?.next().is_some() {
            bail!(
                "PostgreSQL data directory is incomplete: {} (PG_VERSION is missing)",
                self.data_dir.display()
            );
        }

        let staging_dir = self.data_dir.with_extension("initializing");
        if staging_dir.exists() {
            fs::remove_dir_all(&staging_dir).with_context(|| {
                format!(
                    "failed to clean incomplete PostgreSQL initialization at {}",
                    staging_dir.display()
                )
            })?;
        }
        if self.data_dir.exists() {
            fs::remove_dir(&self.data_dir).with_context(|| {
                format!(
                    "failed to remove empty directory {}",
                    self.data_dir.display()
                )
            })?;
        }

        tracing::info!(data_dir = %self.data_dir.display(), "Initializing PostgreSQL data directory");
        let password_file = config_dir.join(".postgres-password.tmp");
        fs::write(&password_file, database_password)
            .with_context(|| format!("failed to write {}", password_file.display()))?;

        let result = run_checked(
            Command::new(initdb)
                .arg("-D")
                .arg(&staging_dir)
                .arg("-U")
                .arg(DATABASE_USER)
                .arg("--encoding=UTF8")
                .arg("--locale=C")
                .arg("--auth-host=scram-sha-256")
                .arg("--auth-local=trust")
                .arg("--pwfile")
                .arg(&password_file),
            "initialize bundled PostgreSQL",
        );
        let _ = fs::remove_file(password_file);
        if let Err(error) = result {
            let _ = fs::remove_dir_all(staging_dir);
            return Err(error);
        }

        fs::rename(&staging_dir, &self.data_dir).with_context(|| {
            format!(
                "failed to activate PostgreSQL data directory {}",
                self.data_dir.display()
            )
        })?;
        Ok(())
    }

    fn is_running(&self) -> anyhow::Result<bool> {
        let output = Command::new(&self.pg_ctl)
            .arg("status")
            .arg("-D")
            .arg(&self.data_dir)
            .output()
            .context("failed to check bundled PostgreSQL status")?;
        Ok(output.status.success())
    }

    fn start_server(&self, log_path: &Path, port: u16) -> anyhow::Result<()> {
        let server_options = format!("-h 127.0.0.1 -p {port}");
        let status = Command::new(&self.pg_ctl)
            .arg("start")
            .arg("-D")
            .arg(&self.data_dir)
            .arg("-l")
            .arg(log_path)
            .arg("-w")
            .arg("-t")
            .arg("30")
            .arg("-o")
            .arg(server_options)
            .status()
            .context("failed to start bundled PostgreSQL")?;
        if !status.success() {
            bail!(
                "failed to start bundled PostgreSQL; see {}",
                log_path.display()
            );
        }
        Ok(())
    }

    fn ensure_database(
        &self,
        createdb: &Path,
        port: u16,
        database_password: &str,
    ) -> anyhow::Result<()> {
        let output = Command::new(createdb)
            .arg("-h")
            .arg("127.0.0.1")
            .arg("-p")
            .arg(port.to_string())
            .arg("-U")
            .arg(DATABASE_USER)
            .arg("-O")
            .arg(DATABASE_USER)
            .arg(DATABASE_NAME)
            .env("PGPASSWORD", database_password)
            .output()
            .context("failed to create the Mad Library database")?;
        if output.status.success() {
            return Ok(());
        }

        let error = command_error(&output);
        if error.to_ascii_lowercase().contains("already exists") {
            return Ok(());
        }
        bail!("failed to create the Mad Library database: {error}")
    }

    fn stop(&mut self) {
        if !self.should_stop {
            return;
        }
        tracing::info!("Stopping bundled PostgreSQL...");
        match Command::new(&self.pg_ctl)
            .arg("stop")
            .arg("-D")
            .arg(&self.data_dir)
            .arg("-m")
            .arg("fast")
            .arg("-w")
            .arg("-t")
            .arg("30")
            .output()
        {
            Ok(output) if output.status.success() => {
                tracing::info!("Bundled PostgreSQL stopped");
            }
            Ok(output) => {
                tracing::error!(error = %command_error(&output), "Failed to stop bundled PostgreSQL");
            }
            Err(error) => {
                tracing::error!(%error, "Failed to stop bundled PostgreSQL");
            }
        }
        self.should_stop = false;
    }
}

impl Drop for ManagedPostgres {
    fn drop(&mut self) {
        self.stop();
    }
}

fn require_executable(bin_dir: &Path, name: &str) -> anyhow::Result<PathBuf> {
    let path = bin_dir.join(executable_name(name));
    if !path.is_file() {
        bail!(
            "bundled PostgreSQL is incomplete; missing {}",
            path.display()
        );
    }
    Ok(path)
}

fn executable_name(name: &str) -> String {
    if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_string()
    }
}

fn run_checked(command: &mut Command, action: &str) -> anyhow::Result<()> {
    let output = command
        .output()
        .with_context(|| format!("failed to {action}"))?;
    if !output.status.success() {
        bail!("failed to {action}: {}", command_error(&output));
    }
    Ok(())
}

fn command_error(output: &Output) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if !stderr.is_empty() {
        return stderr;
    }
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if !stdout.is_empty() {
        return stdout;
    }
    format!("process exited with {}", output.status)
}
