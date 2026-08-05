use std::{fs, path::Path};

use espanso_settings::{ConfigTemplates, ConfigValidator, EspansoConfigValidator, ScaffoldService};
use tempdir::TempDir;

fn templates() -> ConfigTemplates {
    ConfigTemplates {
        default_yml: "# espanso configuration file\n".to_string(),
        base_yml: "matches:\n  - trigger: \":espanso\"\n    replace: \"Hi there!\"\n".to_string(),
    }
}

#[test]
fn preflight_accepts_empty_or_missing_directories() {
    let root = TempDir::new("settings-scaffold-plan").unwrap();
    let current = root.path().join("current");
    fs::create_dir(&current).unwrap();

    let missing = root.path().join("missing");
    let plan = ScaffoldService::preflight(&current, &missing).unwrap();
    assert_eq!(plan.destination.file_name().unwrap(), "missing");

    let empty = root.path().join("empty");
    fs::create_dir(&empty).unwrap();
    assert!(ScaffoldService::preflight(&current, &empty).is_ok());
}

#[test]
fn preflight_rejects_non_empty_files_and_overlapping_directories() {
    let root = TempDir::new("settings-scaffold-reject").unwrap();
    let current = root.path().join("current");
    fs::create_dir(&current).unwrap();

    let non_empty = root.path().join("non-empty");
    fs::create_dir(&non_empty).unwrap();
    fs::write(non_empty.join("keep.txt"), "keep").unwrap();
    assert!(ScaffoldService::preflight(&current, &non_empty).is_err());

    let file = root.path().join("a-file");
    fs::write(&file, "not a directory").unwrap();
    assert!(ScaffoldService::preflight(&current, &file).is_err());

    assert!(ScaffoldService::preflight(&current, &current).is_err());
    assert!(ScaffoldService::preflight(&current, &current.join("child")).is_err());
    assert!(ScaffoldService::preflight(&current, Path::new("relative")).is_err());
}

#[test]
fn execute_writes_the_default_layout_and_passes_espanso_validation() {
    let root = TempDir::new("settings-scaffold-execute").unwrap();
    let current = root.path().join("current");
    fs::create_dir(&current).unwrap();
    let destination = root.path().join("destination");

    let plan = ScaffoldService::preflight(&current, &destination).unwrap();
    ScaffoldService::execute(&plan, &templates(), &EspansoConfigValidator).unwrap();

    assert_eq!(
        fs::read_to_string(destination.join("config/default.yml")).unwrap(),
        templates().default_yml
    );
    assert_eq!(
        fs::read_to_string(destination.join("match/base.yml")).unwrap(),
        templates().base_yml
    );
}

#[test]
fn validation_failure_leaves_no_destination_and_no_staging_directory() {
    struct Invalid;
    impl ConfigValidator for Invalid {
        fn validate(&self, _root: &Path) -> anyhow::Result<()> {
            anyhow::bail!("invalid config")
        }
    }

    let root = TempDir::new("settings-scaffold-invalid").unwrap();
    let current = root.path().join("current");
    fs::create_dir(&current).unwrap();
    let destination = root.path().join("destination");

    let plan = ScaffoldService::preflight(&current, &destination).unwrap();
    assert!(ScaffoldService::execute(&plan, &templates(), &Invalid).is_err());
    assert!(!destination.exists());

    let leftovers = fs::read_dir(root.path())
        .unwrap()
        .filter_map(Result::ok)
        .filter(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .contains("espanso-scaffold")
        })
        .count();
    assert_eq!(leftovers, 0);
}
