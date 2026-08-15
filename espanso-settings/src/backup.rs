use std::{
    fs::{self, File},
    io::{Read, Write},
    path::{Component, Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use walkdir::WalkDir;
use zip::{write::FileOptions, CompressionMethod, ZipArchive, ZipWriter};

use crate::{
    migration::{canonicalize_destination, staging_path},
    AdoptPlan, AdoptService, ConfigValidator,
};

pub const BACKUP_MANIFEST_NAME: &str = "espanso-backup.json";
pub const BACKUP_FORMAT: &str = "espanso-backup";
pub const BACKUP_FORMAT_VERSION: u32 = 1;

/// Written into every archive Settings produces so Import can recognise it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupManifest {
    pub format: String,
    pub format_version: u32,
    pub exported_at: String,
    pub source_path_hint: String,
    pub file_count: usize,
    pub byte_count: u64,
    pub includes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportSummary {
    pub archive: PathBuf,
    pub file_count: usize,
    pub byte_count: u64,
}

/// A portable archive that has been unpacked and checked but not yet published.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportPlan {
    pub archive: PathBuf,
    pub destination: PathBuf,
    /// Temporary directory holding the extracted archive. Always deleted by
    /// [`BackupService::execute_import`] or [`BackupService::discard_import`].
    pub staging_root: PathBuf,
    /// Path under `staging_root` that is the real Espanso configuration root
    /// (may equal `staging_root`, or a single child when the zip had a wrapper).
    pub config_root_in_staging: PathBuf,
    pub adopt: AdoptPlan,
    pub had_manifest: bool,
}

pub struct BackupService;

impl BackupService {
    /// Packs `config_root` into `archive_path` as a portable backup.
    ///
    /// Never modifies the source directory. Symbolic links are refused so a
    /// backup cannot smuggle path escapes, matching [`crate::MigrationService`].
    pub fn export(config_root: &Path, archive_path: &Path) -> Result<ExportSummary> {
        if !config_root.is_dir() {
            bail!("configuration directory does not exist");
        }
        if archive_path.exists() {
            bail!("the backup file already exists; choose a different path");
        }
        if let Some(parent) = archive_path.parent() {
            if !parent.as_os_str().is_empty() && !parent.is_dir() {
                bail!("the backup destination folder does not exist");
            }
        }

        let mut file_count = 0usize;
        let mut byte_count = 0u64;
        let mut entries = Vec::new();

        for entry in WalkDir::new(config_root).min_depth(1).follow_links(false) {
            let entry = entry?;
            let metadata = fs::symlink_metadata(entry.path())?;
            if metadata.file_type().is_symlink() {
                bail!(
                    "symbolic links and reparse points are not supported in backups ({})",
                    entry.path().display()
                );
            }
            let relative = entry
                .path()
                .strip_prefix(config_root)?
                .to_string_lossy()
                .replace('\\', "/");
            if relative.is_empty() {
                continue;
            }
            if metadata.is_file() {
                file_count += 1;
                byte_count = byte_count.saturating_add(metadata.len());
                entries.push((relative, entry.path().to_path_buf(), false));
            } else if metadata.is_dir() {
                let dir_name = if relative.ends_with('/') {
                    relative
                } else {
                    format!("{relative}/")
                };
                entries.push((dir_name, entry.path().to_path_buf(), true));
            }
        }

        let manifest = BackupManifest {
            format: BACKUP_FORMAT.to_string(),
            format_version: BACKUP_FORMAT_VERSION,
            exported_at: unix_timestamp_label(),
            source_path_hint: config_root.to_string_lossy().into_owned(),
            file_count,
            byte_count,
            includes: vec!["config".to_string(), "match".to_string()],
        };
        let manifest_json = serde_json::to_vec_pretty(&manifest)
            .context("unable to serialise the backup manifest")?;

        let file = File::create(archive_path)
            .with_context(|| format!("unable to create {}", archive_path.display()))?;
        let mut zip = ZipWriter::new(file);
        let options = FileOptions::default().compression_method(CompressionMethod::Deflated);

        zip.start_file(BACKUP_MANIFEST_NAME, options)
            .context("unable to write the backup manifest entry")?;
        zip.write_all(&manifest_json)
            .context("unable to write the backup manifest body")?;

        for (name, path, is_dir) in entries {
            if is_dir {
                zip.add_directory(&name, options)
                    .with_context(|| format!("unable to add directory {name} to the backup"))?;
            } else {
                zip.start_file(&name, options)
                    .with_context(|| format!("unable to add {name} to the backup"))?;
                let mut source = File::open(&path)
                    .with_context(|| format!("unable to read {}", path.display()))?;
                let mut buffer = Vec::new();
                source
                    .read_to_end(&mut buffer)
                    .with_context(|| format!("unable to read {}", path.display()))?;
                zip.write_all(&buffer)
                    .with_context(|| format!("unable to compress {name}"))?;
            }
        }

        zip.finish()
            .context("unable to finalise the backup archive")?;

        Ok(ExportSummary {
            archive: archive_path.to_path_buf(),
            file_count,
            byte_count,
        })
    }

    /// Unpacks and validates `archive_path`, planning a restore into `destination`.
    ///
    /// `destination` must be absolute and empty (or not yet exist). The archive is
    /// never applied by this method; call [`Self::execute_import`] after the user
    /// confirms the plan, or [`Self::discard_import`] to cancel.
    pub fn plan_import(archive_path: &Path, destination: &Path) -> Result<ImportPlan> {
        if !archive_path.is_file() {
            bail!("the backup file does not exist");
        }
        if !destination.is_absolute() {
            bail!("the import destination must be an absolute path");
        }

        let destination = canonicalize_destination(destination)?;
        if destination.exists() {
            if !destination.is_dir() {
                bail!("the import destination is not a directory");
            }
            if fs::read_dir(&destination)?.next().is_some() {
                bail!(
                    "the import destination must be empty; pick a new empty folder so the current configuration is never overwritten by default"
                );
            }
        }

        let staging_root = staging_path(&destination, "espanso-import")?;
        if staging_root.exists() {
            bail!("import staging directory already exists");
        }
        fs::create_dir_all(&staging_root).context("unable to create import staging directory")?;

        let extract_result = (|| {
            extract_archive(archive_path, &staging_root)?;
            let (config_root, had_manifest) = locate_config_root(&staging_root)?;
            let adopt = AdoptService::inspect(&config_root)?;
            Ok(ImportPlan {
                archive: archive_path.to_path_buf(),
                destination: destination.clone(),
                staging_root: staging_root.clone(),
                config_root_in_staging: config_root,
                adopt,
                had_manifest,
            })
        })();

        match extract_result {
            Ok(plan) => Ok(plan),
            Err(error) => {
                let _ = fs::remove_dir_all(&staging_root);
                Err(error)
            }
        }
    }

    /// Publishes a previously planned import into its destination.
    pub fn execute_import(plan: &ImportPlan, validator: &dyn ConfigValidator) -> Result<()> {
        if !plan.staging_root.exists() {
            bail!("import staging is missing; plan may have been discarded");
        }

        validator
            .validate(&plan.config_root_in_staging)
            .with_context(|| "restored configuration failed validation")?;

        if plan.destination.exists() {
            fs::remove_dir(&plan.destination)
                .with_context(|| "unable to remove the empty destination directory")?;
        }

        // Publish the real config root (not a wrapper folder) so the destination
        // looks like a normal Espanso configuration directory.
        fs::rename(&plan.config_root_in_staging, &plan.destination)
            .with_context(|| "unable to publish the restored configuration directory")?;

        if plan.staging_root.exists() {
            let _ = fs::remove_dir_all(&plan.staging_root);
        }
        Ok(())
    }

    /// Removes staging left behind by [`Self::plan_import`] when the user cancels.
    pub fn discard_import(plan: &ImportPlan) {
        if plan.staging_root.exists() {
            let _ = fs::remove_dir_all(&plan.staging_root);
        }
    }

    /// Plan + execute against an empty destination in one shot.
    pub fn import(
        archive_path: &Path,
        destination: &Path,
        validator: &dyn ConfigValidator,
    ) -> Result<ImportPlan> {
        let plan = Self::plan_import(archive_path, destination)?;
        match Self::execute_import(&plan, validator) {
            Ok(()) => Ok(plan),
            Err(error) => {
                Self::discard_import(&plan);
                Err(error)
            }
        }
    }

    /// Default file name for a new export.
    pub fn default_export_name() -> String {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        format!("espanso-backup-{stamp}.espanso-backup.zip")
    }
}

fn unix_timestamp_label() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    format!("unix:{secs}")
}

fn extract_archive(archive_path: &Path, staging: &Path) -> Result<()> {
    let file = File::open(archive_path)
        .with_context(|| format!("unable to open {}", archive_path.display()))?;
    let mut archive =
        ZipArchive::new(file).with_context(|| "the selected file is not a valid zip archive")?;

    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .with_context(|| format!("unable to read zip entry {index}"))?;
        let Some(enclosed) = entry.enclosed_name().map(Path::to_path_buf) else {
            bail!("backup entry has an unsafe path and was refused");
        };
        for component in enclosed.components() {
            match component {
                Component::Normal(_) | Component::CurDir => {}
                _ => bail!("backup entry has an unsafe path and was refused"),
            }
        }

        let output = staging.join(&enclosed);
        if entry.is_dir() {
            fs::create_dir_all(&output)
                .with_context(|| format!("unable to create {}", output.display()))?;
            continue;
        }
        if let Some(parent) = output.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("unable to create {}", parent.display()))?;
        }
        let mut out = File::create(&output)
            .with_context(|| format!("unable to create {}", output.display()))?;
        std::io::copy(&mut entry, &mut out)
            .with_context(|| format!("unable to extract {}", output.display()))?;
    }
    Ok(())
}

/// Finds the Espanso configuration root inside an extracted archive.
///
/// Accepts:
/// 1. staging itself contains `config/`;
/// 2. exactly one child directory contains `config/` (common hand-zip wrapper).
fn locate_config_root(staging: &Path) -> Result<(PathBuf, bool)> {
    let had_manifest = staging.join(BACKUP_MANIFEST_NAME).is_file();
    if had_manifest {
        let raw = fs::read_to_string(staging.join(BACKUP_MANIFEST_NAME))
            .context("unable to read the backup manifest")?;
        let manifest: BackupManifest =
            serde_json::from_str(&raw).context("the backup manifest is not valid JSON")?;
        if manifest.format != BACKUP_FORMAT {
            bail!(
                "unsupported backup format '{}'; expected '{BACKUP_FORMAT}'",
                manifest.format
            );
        }
        if manifest.format_version > BACKUP_FORMAT_VERSION {
            bail!(
                "backup format version {} is newer than this Settings build supports ({BACKUP_FORMAT_VERSION})",
                manifest.format_version
            );
        }
    }

    if staging.join("config").is_dir() {
        return Ok((staging.to_path_buf(), had_manifest));
    }

    let mut candidates = Vec::new();
    for entry in fs::read_dir(staging)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() && entry.path().join("config").is_dir() {
            candidates.push(entry.path());
        }
    }
    match candidates.as_slice() {
        [only] => Ok((only.clone(), had_manifest)),
        [] => bail!(
            "the archive does not contain an Espanso configuration (missing 'config' folder); nothing was written"
        ),
        _ => bail!(
            "the archive contains multiple folders with a 'config' directory; re-export from Settings or zip a single configuration root"
        ),
    }
}
