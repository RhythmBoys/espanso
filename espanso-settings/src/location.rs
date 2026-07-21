use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context, Result};
use atomicwrites::{AllowOverwrite, AtomicFile};
use serde::{Deserialize, Serialize};

const LOCATION_FILE_VERSION: u8 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigPathSource {
    Cli,
    Environment,
    Persisted,
    PlatformDefault,
}

impl ConfigPathSource {
    pub const fn display_name(self) -> &'static str {
        match self {
            Self::Cli => "命令行覆盖",
            Self::Environment => "环境变量覆盖",
            Self::Persisted => "自定义目录",
            Self::PlatformDefault => "系统默认",
        }
    }

    pub const fn allows_persistent_change(self) -> bool {
        !matches!(self, Self::Cli | Self::Environment)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedConfigLocation {
    pub path: PathBuf,
    pub source: ConfigPathSource,
}

#[derive(Debug, Serialize, Deserialize)]
struct StoredLocation {
    version: u8,
    config_dir: PathBuf,
}

#[derive(Debug, Clone)]
pub struct ConfigLocationStore {
    path: PathBuf,
}

impl ConfigLocationStore {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn load(&self) -> Result<Option<PathBuf>> {
        if !self.path.exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(&self.path)
            .with_context(|| "unable to read the persisted configuration location")?;
        let stored: StoredLocation = serde_json::from_str(&content)
            .with_context(|| "invalid persisted configuration location")?;
        if stored.version != LOCATION_FILE_VERSION {
            bail!("unsupported configuration location version");
        }

        Ok(Some(validate_directory(&stored.config_dir)?))
    }

    pub fn save_atomic(&self, path: &Path) -> Result<()> {
        let config_dir = validate_directory(path)?;
        let parent = self
            .path
            .parent()
            .context("configuration location file has no parent")?;
        fs::create_dir_all(parent)
            .with_context(|| "unable to create configuration location directory")?;

        let payload = serde_json::to_vec_pretty(&StoredLocation {
            version: LOCATION_FILE_VERSION,
            config_dir,
        })?;
        AtomicFile::new(&self.path, AllowOverwrite)
            .write(|file| {
                file.write_all(&payload)?;
                file.sync_all()
            })
            .with_context(|| "unable to persist configuration location atomically")?;
        Ok(())
    }

    pub fn clear(&self) -> Result<()> {
        if self.path.exists() {
            fs::remove_file(&self.path)
                .with_context(|| "unable to clear persisted configuration location")?;
        }
        Ok(())
    }
}

pub fn resolve_config_location(
    cli: Option<&Path>,
    environment: Option<&Path>,
    store: &ConfigLocationStore,
    platform_default: &Path,
) -> Result<ResolvedConfigLocation> {
    if let Some(path) = cli {
        return resolved(path, ConfigPathSource::Cli);
    }
    if let Some(path) = environment {
        return resolved(path, ConfigPathSource::Environment);
    }
    if let Some(path) = store.load()? {
        return Ok(ResolvedConfigLocation {
            path,
            source: ConfigPathSource::Persisted,
        });
    }
    resolved(platform_default, ConfigPathSource::PlatformDefault)
}

fn resolved(path: &Path, source: ConfigPathSource) -> Result<ResolvedConfigLocation> {
    Ok(ResolvedConfigLocation {
        path: validate_directory(path)?,
        source,
    })
}

fn validate_directory(path: &Path) -> Result<PathBuf> {
    if path.as_os_str().is_empty() || !path.is_absolute() {
        bail!("configuration directory must be an absolute path");
    }
    if !path.is_dir() {
        bail!("configuration directory does not exist or is not a directory");
    }
    path.canonicalize()
        .with_context(|| "unable to canonicalize configuration directory")
}
