use std::{
    collections::HashSet,
    fs,
    io::Write,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{bail, Context, Result};
use atomicwrites::{AllowOverwrite, AtomicFile};
use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

use crate::{migration::copy_tree, ConfigValidator, EspansoConfigValidator};

const MANAGED_HEADER: &str =
    "# Managed by Espanso Settings. Advanced rules may be edited in other files.\n";
const MAX_REPLACEMENT_CHARS: usize = 1_000_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiMatch {
    pub id: String,
    pub trigger: String,
    pub replace: String,
}

pub trait MatchRepository {
    fn load(&self) -> Result<Vec<UiMatch>>;
    fn save(&self, matches: &[UiMatch]) -> Result<()>;
}

#[derive(Debug, Clone)]
pub struct UiMatchRepository {
    config_root: PathBuf,
    path: PathBuf,
}

#[derive(Debug, Serialize, Deserialize)]
struct MatchDocument {
    #[serde(default)]
    matches: Vec<StoredMatch>,
}

#[derive(Debug, Serialize, Deserialize)]
struct StoredMatch {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    label: Option<String>,
    trigger: String,
    replace: String,
}

#[derive(Debug, Default, Deserialize)]
struct TriggerScanDocument {
    #[serde(default)]
    matches: Vec<TriggerScanMatch>,
}

#[derive(Debug, Default, Deserialize)]
struct TriggerScanMatch {
    trigger: Option<String>,
    #[serde(default)]
    triggers: Vec<String>,
}

impl UiMatchRepository {
    pub fn new(config_root: &Path) -> Self {
        Self {
            config_root: config_root.to_path_buf(),
            path: config_root.join("match").join("ui.yml"),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl MatchRepository for UiMatchRepository {
    fn load(&self) -> Result<Vec<UiMatch>> {
        if !self.path.exists() {
            return Ok(Vec::new());
        }
        let content =
            fs::read_to_string(&self.path).with_context(|| "unable to read UI-managed matches")?;
        let document: MatchDocument = serde_norway::from_str(&content)
            .with_context(|| "unable to parse UI-managed matches")?;
        let matches = document
            .matches
            .into_iter()
            .enumerate()
            .map(|(index, item)| UiMatch {
                id: item.label.unwrap_or_else(|| format!("ui-match-{index}")),
                trigger: item.trigger,
                replace: item.replace,
            })
            .collect::<Vec<_>>();
        validate_matches(&matches)?;
        Ok(matches)
    }

    fn save(&self, matches: &[UiMatch]) -> Result<()> {
        validate_matches(matches)?;
        self.validate_external_trigger_conflicts(matches)?;
        if self.config_root.exists() {
            EspansoConfigValidator
                .validate(&self.config_root)
                .with_context(|| "existing Espanso configuration is invalid")?;
        }

        let document = MatchDocument {
            matches: matches
                .iter()
                .map(|item| StoredMatch {
                    label: Some(item.id.clone()),
                    trigger: item.trigger.clone(),
                    replace: item.replace.clone(),
                })
                .collect(),
        };
        let mut payload = MANAGED_HEADER.as_bytes().to_vec();
        payload.extend(serde_norway::to_string(&document)?.as_bytes());

        self.validate_candidate(&payload)?;

        let parent = self
            .path
            .parent()
            .context("managed match file has no parent")?;
        fs::create_dir_all(parent)?;
        if self.path.exists() {
            fs::copy(&self.path, self.path.with_extension("yml.backup"))
                .with_context(|| "unable to back up the previous managed match file")?;
        }
        AtomicFile::new(&self.path, AllowOverwrite)
            .write(|file| {
                file.write_all(&payload)?;
                file.sync_all()
            })
            .with_context(|| "unable to save UI-managed matches atomically")?;
        Ok(())
    }
}

impl UiMatchRepository {
    fn validate_external_trigger_conflicts(&self, matches: &[UiMatch]) -> Result<()> {
        let candidate_triggers = matches
            .iter()
            .map(|item| item.trigger.trim())
            .collect::<HashSet<_>>();
        let match_root = self.config_root.join("match");
        if !match_root.exists() {
            return Ok(());
        }

        for entry in WalkDir::new(match_root).follow_links(false) {
            let entry = entry?;
            if !entry.file_type().is_file() || entry.path() == self.path {
                continue;
            }
            let extension = entry
                .path()
                .extension()
                .and_then(|value| value.to_str())
                .unwrap_or_default();
            if !extension.eq_ignore_ascii_case("yml") && !extension.eq_ignore_ascii_case("yaml") {
                continue;
            }

            let content = fs::read_to_string(entry.path())?;
            let document: TriggerScanDocument = serde_norway::from_str(&content)
                .with_context(|| format!("unable to parse {}", entry.path().display()))?;
            for item in document.matches {
                let external_triggers = item.trigger.into_iter().chain(item.triggers);
                for trigger in external_triggers {
                    if candidate_triggers.contains(trigger.trim()) {
                        bail!(
                            "trigger {trigger:?} already exists in {}",
                            entry.path().display()
                        );
                    }
                }
            }
        }
        Ok(())
    }

    fn validate_candidate(&self, payload: &[u8]) -> Result<()> {
        let parent = self
            .config_root
            .parent()
            .context("configuration root has no parent")?;
        let root_name = self
            .config_root
            .file_name()
            .context("configuration root has no directory name")?
            .to_string_lossy();
        let nonce = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        let staging = parent.join(format!(
            ".{root_name}.espanso-settings-validation-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir(&staging)
            .with_context(|| "unable to create candidate validation directory")?;

        let validation = (|| -> Result<()> {
            copy_tree(&self.config_root, &staging)?;
            let candidate_path = staging.join("match").join("ui.yml");
            if let Some(parent) = candidate_path.parent() {
                fs::create_dir_all(parent)?;
            }
            let mut candidate = fs::File::create(&candidate_path)?;
            candidate.write_all(payload)?;
            candidate.sync_all()?;
            EspansoConfigValidator
                .validate(&staging)
                .with_context(|| "candidate matches conflict with the complete configuration")
        })();
        let cleanup = fs::remove_dir_all(&staging)
            .with_context(|| "unable to remove candidate validation directory");

        validation?;
        cleanup?;
        Ok(())
    }
}

fn validate_matches(matches: &[UiMatch]) -> Result<()> {
    let mut triggers = HashSet::new();
    for item in matches {
        let normalized = item.trigger.trim();
        if normalized.is_empty() {
            bail!("trigger must not be blank");
        }
        if item.trigger.contains(['\n', '\r', '\0']) {
            bail!("trigger must be a single line without NUL characters");
        }
        if !triggers.insert(normalized.to_string()) {
            bail!("duplicate trigger: {normalized}");
        }
        if item.replace.chars().count() > MAX_REPLACEMENT_CHARS {
            bail!("replacement exceeds the supported character limit");
        }
    }
    Ok(())
}
