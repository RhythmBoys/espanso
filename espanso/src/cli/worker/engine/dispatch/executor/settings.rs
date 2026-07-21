use std::process::Command;

use espanso_engine::dispatch::SettingsHandler;

use crate::cli::{util::CommandExt, PathsOverrides};

pub struct SettingsHandlerAdapter {
    paths_overrides: PathsOverrides,
}

impl SettingsHandlerAdapter {
    pub fn new(paths_overrides: &PathsOverrides) -> Self {
        Self {
            paths_overrides: paths_overrides.clone(),
        }
    }
}

impl SettingsHandler for SettingsHandlerAdapter {
    fn show_settings(&self) -> anyhow::Result<()> {
        Command::new(std::env::current_exe()?)
            .arg("settings")
            .with_paths_overrides(&self.paths_overrides)
            .spawn()?;
        Ok(())
    }
}
