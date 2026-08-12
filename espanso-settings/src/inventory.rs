use std::path::{Path, PathBuf};

use serde::Deserialize;
use walkdir::WalkDir;

/// A match that lives in a file Settings does not own.
///
/// These are shown so that an adopted configuration is visible in the editor,
/// but never written back. Espanso matches may use regex triggers, forms,
/// variables and images that this editor cannot represent, and no Rust YAML
/// library preserves comments across a round trip, so rewriting such a file
/// would silently drop the user's content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternalMatch {
    pub trigger: String,
    pub preview: String,
    /// Absolute path, used to open the file in the system editor.
    pub source: PathBuf,
    /// Path relative to the configuration root, used as the on-screen badge.
    pub source_label: String,
    /// Why this entry is read-only, phrased so the user knows where to go next.
    pub note: String,
}

#[derive(Debug, Default, Deserialize)]
struct ScanDocument {
    #[serde(default)]
    matches: Vec<ScanMatch>,
}

#[derive(Debug, Default, Deserialize)]
struct ScanMatch {
    label: Option<String>,
    trigger: Option<String>,
    #[serde(default)]
    triggers: Vec<String>,
    regex: Option<String>,
    replace: Option<String>,
    form: Option<serde_norway::Value>,
    #[serde(default)]
    vars: Vec<serde_norway::Value>,
    image_path: Option<String>,
}

/// Collects every match under `<config_root>/match` except those in `owned`.
///
/// Files that fail to parse are skipped rather than propagated: this only feeds
/// the read-only list, and a single unrelated broken file should not stop the
/// editor from opening. Configurations Espanso itself rejects are already
/// blocked upstream, when the directory is adopted.
pub fn scan_external_matches(config_root: &Path, owned: &Path) -> Vec<ExternalMatch> {
    let match_root = config_root.join("match");
    if !match_root.is_dir() {
        return Vec::new();
    }

    let mut collected = Vec::new();
    for entry in WalkDir::new(&match_root)
        .follow_links(false)
        .into_iter()
        .flatten()
    {
        if !entry.file_type().is_file() || entry.path() == owned {
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

        let Ok(content) = std::fs::read_to_string(entry.path()) else {
            continue;
        };
        let Ok(document) = serde_norway::from_str::<ScanDocument>(&content) else {
            continue;
        };

        let source_label = entry
            .path()
            .strip_prefix(config_root)
            .unwrap_or(entry.path())
            .to_string_lossy()
            .replace('\\', "/");
        for item in document.matches {
            collected.push(describe(item, entry.path(), &source_label));
        }
    }

    collected.sort_by(|left, right| {
        left.source_label
            .cmp(&right.source_label)
            .then_with(|| left.trigger.cmp(&right.trigger))
    });
    collected
}

fn describe(item: ScanMatch, source: &Path, source_label: &str) -> ExternalMatch {
    let trigger = item
        .trigger
        .or_else(|| item.triggers.first().cloned())
        .or_else(|| item.regex.clone())
        .or_else(|| item.label.clone())
        .unwrap_or_else(|| "(no trigger)".to_string());

    // The most specific reason wins: telling the user "this is a form" points
    // them at the right documentation, where "read-only" alone would not.
    let note = if item.regex.is_some() {
        "regex trigger — edit in the file"
    } else if item.form.is_some() {
        "form — edit in the file"
    } else if !item.vars.is_empty() {
        "uses variables — edit in the file"
    } else if item.image_path.is_some() {
        "image match — edit in the file"
    } else {
        "defined outside Settings — edit in the file"
    };

    let preview = item
        .replace
        .as_deref()
        .and_then(|replace| replace.lines().next())
        .unwrap_or(note)
        .to_string();

    ExternalMatch {
        trigger,
        preview,
        source: source.to_path_buf(),
        source_label: source_label.to_string(),
        note: note.to_string(),
    }
}
