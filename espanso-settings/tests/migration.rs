use std::{fs, path::Path};

use espanso_settings::{ConfigValidator, EspansoConfigValidator, MigrationService};
use tempdir::TempDir;

struct AlwaysValid;

impl ConfigValidator for AlwaysValid {
    fn validate(&self, _root: &Path) -> anyhow::Result<()> {
        Ok(())
    }
}

#[test]
fn preflight_counts_files_and_rejects_unsafe_destinations() {
    let root = TempDir::new("settings-migration-plan").unwrap();
    let source = root.path().join("source");
    fs::create_dir(&source).unwrap();
    fs::write(source.join("one.yml"), "abc").unwrap();
    fs::create_dir(source.join("nested")).unwrap();
    fs::write(source.join("nested/two.yml"), "hello").unwrap();

    let destination = root.path().join("destination");
    let plan = MigrationService::preflight(&source, &destination).unwrap();
    assert_eq!(plan.file_count, 2);
    assert_eq!(plan.byte_count, 8);
    assert!(plan.conflicts.is_empty());

    assert!(MigrationService::preflight(&source, &source).is_err());
    assert!(MigrationService::preflight(&source, &source.join("child")).is_err());

    let non_empty = root.path().join("non-empty");
    fs::create_dir(&non_empty).unwrap();
    fs::write(non_empty.join("keep.txt"), "keep").unwrap();
    assert!(MigrationService::preflight(&source, &non_empty).is_err());
}

#[test]
fn execute_copies_validated_config_and_keeps_source() {
    let root = TempDir::new("settings-migration-execute").unwrap();
    let source = root.path().join("source");
    let destination = root.path().join("destination");
    fs::create_dir(&source).unwrap();
    fs::write(source.join("base.yml"), "content").unwrap();

    let plan = MigrationService::preflight(&source, &destination).unwrap();
    MigrationService::execute(&plan, &AlwaysValid).unwrap();

    assert_eq!(
        fs::read_to_string(destination.join("base.yml")).unwrap(),
        "content"
    );
    assert_eq!(
        fs::read_to_string(source.join("base.yml")).unwrap(),
        "content"
    );
}

#[test]
fn validation_failure_does_not_publish_destination() {
    struct Invalid;
    impl ConfigValidator for Invalid {
        fn validate(&self, _root: &Path) -> anyhow::Result<()> {
            anyhow::bail!("invalid config")
        }
    }

    let root = TempDir::new("settings-migration-invalid").unwrap();
    let source = root.path().join("source");
    let destination = root.path().join("destination");
    fs::create_dir(&source).unwrap();
    fs::write(source.join("base.yml"), "content").unwrap();

    let plan = MigrationService::preflight(&source, &destination).unwrap();
    assert!(MigrationService::execute(&plan, &Invalid).is_err());
    assert!(!destination.exists());
    assert!(source.join("base.yml").exists());
}

#[test]
fn espanso_validator_rejects_non_fatal_yaml_errors_before_publish() {
    let root = TempDir::new("settings-migration-yaml-error").unwrap();
    let source = root.path().join("source");
    let destination = root.path().join("destination");
    fs::create_dir_all(source.join("config")).unwrap();
    fs::create_dir_all(source.join("match")).unwrap();
    fs::write(source.join("config/default.yml"), "matches: []\n").unwrap();
    fs::write(source.join("match/broken.yml"), "matches: [\n").unwrap();

    let plan = MigrationService::preflight(&source, &destination).unwrap();
    assert!(MigrationService::execute(&plan, &EspansoConfigValidator).is_err());
    assert!(!destination.exists());
    assert!(source.join("match/broken.yml").exists());
}
