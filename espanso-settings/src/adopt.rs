use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context, Result};

use crate::migration::canonicalize_destination;

/// An existing Espanso configuration directory that Settings can point at.
///
/// Adopting writes nothing: the directory keeps every file, comment and
/// hand-written match exactly as it is. That is what makes it safe to switch
/// without asking the user to confirm first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdoptPlan {
    pub destination: PathBuf,
    pub config_count: usize,
    pub match_count: usize,
    /// Files Espanso loaded but complained about. Non-fatal, so they do not
    /// block the switch; the caller surfaces them as a warning.
    pub warning_files: Vec<PathBuf>,
}

pub struct AdoptService;

impl AdoptService {
    /// Plans opening `selected` as the active configuration directory.
    ///
    /// Open never scaffolds: an empty folder is rejected with a pointer to
    /// “Create new configuration…”. Only a directory Espanso can load is
    /// accepted, and nothing is written to it.
    pub fn plan(current_config: &Path, selected: &Path) -> Result<AdoptPlan> {
        if !selected.is_absolute() {
            bail!("the selected directory must be an absolute path");
        }
        if selected.exists() && !selected.is_dir() {
            bail!("the selected path is not a directory");
        }

        let is_empty = !selected.exists() || fs::read_dir(selected)?.next().is_none();
        if is_empty {
            bail!(
                "the selected directory is empty; use “Create new configuration…” to generate a default configuration there"
            );
        }

        let destination = canonicalize_destination(selected)?;
        reject_overlap(current_config, &destination)?;

        if !destination.join("config").is_dir() {
            bail!(
                "the selected directory is not empty and does not contain a 'config' folder, so it is not an Espanso configuration; pick the folder that holds 'config' and 'match', use “Import backup…” for a portable archive, or “Create new configuration…” with an empty folder"
            );
        }

        Self::inspect(&destination)
    }

    /// Loads `root` with Espanso's own loader to confirm it is usable.
    ///
    /// Deliberately more forgiving than [`crate::EspansoConfigValidator`]: that
    /// one guards directories Settings just wrote, where any complaint means a
    /// bug of ours, while this one guards a directory the user has been editing
    /// by hand. A fatal load failure blocks; non-fatal errors are reported and
    /// let through.
    pub fn inspect(root: &Path) -> Result<AdoptPlan> {
        let (config_store, match_store, non_fatal_errors) = espanso_config::load(root)
            .with_context(|| "Espanso could not load that configuration directory")?;

        let match_paths = config_store
            .get_all_match_paths()
            .into_iter()
            .collect::<Vec<_>>();

        Ok(AdoptPlan {
            destination: root.to_path_buf(),
            config_count: config_store.configs().len(),
            match_count: match_store.query(&match_paths).matches.len(),
            warning_files: non_fatal_errors
                .into_iter()
                .map(|error_set| error_set.file)
                .collect(),
        })
    }
}

fn reject_overlap(current_config: &Path, destination: &Path) -> Result<()> {
    if !current_config.exists() {
        return Ok(());
    }
    let current = dunce::canonicalize(current_config)
        .with_context(|| "unable to canonicalize the current configuration directory")?;
    if current == *destination {
        bail!("that directory is already the active configuration directory");
    }
    if destination.starts_with(&current) || current.starts_with(destination) {
        bail!("the selected directory must not contain, or sit inside, the current configuration directory");
    }
    Ok(())
}
