use std::{
    fs::{self, File},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{bail, Context, Result};
use walkdir::WalkDir;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrationPlan {
    pub source: PathBuf,
    pub destination: PathBuf,
    pub file_count: usize,
    pub byte_count: u64,
    pub conflicts: Vec<PathBuf>,
}

pub trait ConfigValidator {
    fn validate(&self, root: &Path) -> Result<()>;
}

pub struct EspansoConfigValidator;

impl ConfigValidator for EspansoConfigValidator {
    fn validate(&self, root: &Path) -> Result<()> {
        let (_, _, non_fatal_errors) = espanso_config::load(root)
            .with_context(|| "Espanso rejected the copied configuration")?;
        if !non_fatal_errors.is_empty() {
            bail!(
                "copied configuration contains {} parsing or validation error set(s)",
                non_fatal_errors.len()
            );
        }
        Ok(())
    }
}

pub struct MigrationService;

impl MigrationService {
    pub fn preflight(source: &Path, destination: &Path) -> Result<MigrationPlan> {
        if !source.is_absolute() || !destination.is_absolute() {
            bail!("source and destination must be absolute paths");
        }
        if !source.is_dir() {
            bail!("source configuration directory does not exist");
        }

        let source = dunce::canonicalize(source)
            .with_context(|| "unable to canonicalize source directory")?;
        let destination = canonicalize_destination(destination)?;
        if source == destination
            || destination.starts_with(&source)
            || source.starts_with(&destination)
        {
            bail!("source and destination must not overlap");
        }

        if destination.exists() {
            if !destination.is_dir() {
                bail!("destination is not a directory");
            }
            if fs::read_dir(&destination)?.next().is_some() {
                bail!("destination must be empty");
            }
        }

        let mut file_count = 0;
        let mut byte_count = 0_u64;
        let mut conflicts = Vec::new();
        for entry in WalkDir::new(&source).follow_links(false) {
            let entry = entry?;
            let metadata = fs::symlink_metadata(entry.path())?;
            if metadata.file_type().is_symlink() {
                conflicts.push(entry.path().strip_prefix(&source)?.to_path_buf());
            } else if metadata.is_file() {
                file_count += 1;
                byte_count = byte_count.saturating_add(metadata.len());
            }
        }

        Ok(MigrationPlan {
            source,
            destination,
            file_count,
            byte_count,
            conflicts,
        })
    }

    pub fn execute(plan: &MigrationPlan, validator: &dyn ConfigValidator) -> Result<()> {
        if !plan.conflicts.is_empty() {
            bail!("migration contains unsupported symbolic links or reparse points");
        }
        let staging = staging_path(&plan.destination)?;
        if staging.exists() {
            bail!("migration staging directory already exists");
        }
        fs::create_dir(&staging).with_context(|| "unable to create migration staging directory")?;

        copy_tree(&plan.source, &staging)?;
        validator.validate(&staging)?;

        if plan.destination.exists() {
            fs::remove_dir(&plan.destination)
                .with_context(|| "unable to remove empty destination directory")?;
        }
        fs::rename(&staging, &plan.destination)
            .with_context(|| "unable to publish the validated configuration directory")?;
        Ok(())
    }
}

pub(crate) fn copy_tree(source: &Path, staging: &Path) -> Result<()> {
    for entry in WalkDir::new(source).min_depth(1).follow_links(false) {
        let entry = entry?;
        let relative = entry.path().strip_prefix(source)?;
        let target = staging.join(relative);
        let metadata = fs::symlink_metadata(entry.path())?;
        if metadata.file_type().is_symlink() {
            bail!("symbolic links and reparse points are not supported");
        }
        if metadata.is_dir() {
            fs::create_dir_all(&target)?;
        } else if metadata.is_file() {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(entry.path(), &target)?;
            // The handle must be writable: on Windows `sync_all` maps to
            // `FlushFileBuffers`, which requires GENERIC_WRITE and fails with
            // `ERROR_ACCESS_DENIED` on a read-only handle.
            File::options()
                .write(true)
                .open(&target)
                .with_context(|| format!("unable to reopen {} to flush it", target.display()))?
                .sync_all()
                .with_context(|| format!("unable to flush {} to disk", target.display()))?;
        }
    }
    Ok(())
}

// `dunce::canonicalize` rather than `Path::canonicalize`: the latter returns a
// `\\?\` verbatim path on Windows, and these paths are handed to
// `espanso_config::load`, whose glob patterns contain `..`. `glob` 0.3 computes
// its root offset by subtracting two `PathBuf` lengths, and only the verbatim
// one collapses `..`, so the subtraction underflows and panics. espanso-config
// uses dunce for the same reason.
fn canonicalize_destination(destination: &Path) -> Result<PathBuf> {
    if destination.exists() {
        return dunce::canonicalize(destination)
            .with_context(|| "unable to canonicalize destination directory");
    }
    let parent = dunce::canonicalize(
        destination
            .parent()
            .context("destination directory has no parent")?,
    )
    .with_context(|| "destination parent does not exist")?;
    let name = destination
        .file_name()
        .context("destination directory has no name")?;
    Ok(parent.join(name))
}

fn staging_path(destination: &Path) -> Result<PathBuf> {
    let parent = destination
        .parent()
        .context("destination directory has no parent")?;
    let name = destination
        .file_name()
        .context("destination directory has no name")?
        .to_string_lossy();
    let nonce = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    Ok(parent.join(format!(
        "{name}.espanso-migration-{}-{nonce}",
        std::process::id()
    )))
}
