use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context, Result};

use crate::migration::{canonicalize_destination, staging_path, ConfigValidator};

/// Default file contents used to bootstrap a fresh configuration directory.
///
/// The templates are injected by the caller rather than embedded here: they
/// live in the `espanso` binary crate next to `populate_default_config`, and
/// this crate must not depend on a binary crate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigTemplates {
    pub default_yml: String,
    pub base_yml: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScaffoldPlan {
    pub destination: PathBuf,
}

pub struct ScaffoldService;

impl ScaffoldService {
    /// Checks that `destination` is an absolute, empty directory that does not
    /// overlap the currently active configuration directory.
    pub fn preflight(current_config: &Path, destination: &Path) -> Result<ScaffoldPlan> {
        if !destination.is_absolute() {
            bail!("the selected directory must be an absolute path");
        }

        let destination = canonicalize_destination(destination)?;
        if destination.exists() {
            if !destination.is_dir() {
                bail!("the selected path is not a directory");
            }
            if fs::read_dir(&destination)?.next().is_some() {
                bail!(
                    "the selected directory is not empty; create a new empty folder and select it"
                );
            }
        }

        if current_config.exists() {
            let current = dunce::canonicalize(current_config)
                .with_context(|| "unable to canonicalize the current configuration directory")?;
            if current == destination
                || destination.starts_with(&current)
                || current.starts_with(&destination)
            {
                bail!(
                    "the selected directory must not overlap the current configuration directory"
                );
            }
        }

        Ok(ScaffoldPlan { destination })
    }

    /// Writes the default configuration into a staging directory, validates it
    /// with Espanso's own loader, then publishes it with a single rename.
    /// Every failure mode leaves the active configuration directory untouched.
    pub fn execute(
        plan: &ScaffoldPlan,
        templates: &ConfigTemplates,
        validator: &dyn ConfigValidator,
    ) -> Result<()> {
        let staging = staging_path(&plan.destination, "espanso-scaffold")?;
        if staging.exists() {
            bail!("scaffold staging directory already exists");
        }
        fs::create_dir(&staging).with_context(|| "unable to create scaffold staging directory")?;

        let result = populate(&staging, templates).and_then(|()| validator.validate(&staging));
        if let Err(error) = result {
            let _ = fs::remove_dir_all(&staging);
            return Err(error);
        }

        if plan.destination.exists() {
            fs::remove_dir(&plan.destination)
                .with_context(|| "unable to remove the empty destination directory")?;
        }
        fs::rename(&staging, &plan.destination)
            .with_context(|| "unable to publish the generated configuration directory")?;
        Ok(())
    }
}

/// Mirrors the layout produced by `espanso::config::populate_default_config` so
/// that a directory created here is indistinguishable from one created on first
/// run.
fn populate(root: &Path, templates: &ConfigTemplates) -> Result<()> {
    let config_dir = root.join("config");
    let match_dir = root.join("match");
    fs::create_dir(&config_dir).with_context(|| "unable to create the config directory")?;
    fs::create_dir(&match_dir).with_context(|| "unable to create the match directory")?;

    fs::write(config_dir.join("default.yml"), &templates.default_yml)
        .with_context(|| "unable to write default.yml")?;
    fs::write(match_dir.join("base.yml"), &templates.base_yml)
        .with_context(|| "unable to write base.yml")?;
    Ok(())
}
