use std::{fs, path::Path};

use espanso_settings::{AdoptService, ConfigFolderPlan};
use tempdir::TempDir;

/// Writes a minimal but genuinely loadable Espanso configuration.
fn write_config(root: &Path, base_yml: &str) {
    fs::create_dir_all(root.join("config")).unwrap();
    fs::create_dir_all(root.join("match")).unwrap();
    fs::write(root.join("config/default.yml"), "# espanso config\n").unwrap();
    fs::write(root.join("match/base.yml"), base_yml).unwrap();
}

#[test]
fn empty_and_missing_directories_are_planned_as_scaffold() {
    let root = TempDir::new("settings-adopt-scaffold").unwrap();
    let current = root.path().join("current");
    write_config(&current, "matches: []\n");

    let missing = root.path().join("missing");
    assert!(matches!(
        AdoptService::plan(&current, &missing).unwrap(),
        ConfigFolderPlan::Scaffold(_)
    ));

    let empty = root.path().join("empty");
    fs::create_dir(&empty).unwrap();
    assert!(matches!(
        AdoptService::plan(&current, &empty).unwrap(),
        ConfigFolderPlan::Scaffold(_)
    ));
}

#[test]
fn an_existing_configuration_is_adopted_and_counted() {
    let root = TempDir::new("settings-adopt-existing").unwrap();
    let current = root.path().join("current");
    write_config(&current, "matches: []\n");

    let existing = root.path().join("existing");
    write_config(
        &existing,
        "matches:\n  - trigger: \":hi\"\n    replace: \"Hello\"\n  - trigger: \":bye\"\n    replace: \"Bye\"\n",
    );

    let ConfigFolderPlan::Adopt(plan) = AdoptService::plan(&current, &existing).unwrap() else {
        panic!("an existing configuration must be adopted, not scaffolded");
    };
    assert_eq!(plan.match_count, 2);
    assert_eq!(plan.config_count, 1);
    assert!(plan.warning_files.is_empty());
}

#[test]
fn adopting_does_not_touch_a_single_file() {
    let root = TempDir::new("settings-adopt-readonly").unwrap();
    let current = root.path().join("current");
    write_config(&current, "matches: []\n");

    let existing = root.path().join("existing");
    // Comments and hand-written formatting are exactly what a rewrite would
    // destroy, so assert they survive byte for byte.
    let base =
        "# my snippets\nmatches:\n  - trigger: \":hi\"   # greeting\n    replace: \"Hello\"\n";
    write_config(&existing, base);

    let before = fs::read_dir(&existing)
        .unwrap()
        .filter_map(Result::ok)
        .count();
    AdoptService::plan(&current, &existing).unwrap();

    assert_eq!(
        fs::read_to_string(existing.join("match/base.yml")).unwrap(),
        base
    );
    assert_eq!(
        fs::read_dir(&existing)
            .unwrap()
            .filter_map(Result::ok)
            .count(),
        before
    );
}

#[test]
fn non_empty_directories_without_a_config_folder_are_rejected() {
    let root = TempDir::new("settings-adopt-foreign").unwrap();
    let current = root.path().join("current");
    write_config(&current, "matches: []\n");

    let foreign = root.path().join("foreign");
    fs::create_dir(&foreign).unwrap();
    fs::write(foreign.join("notes.txt"), "unrelated").unwrap();

    let error = AdoptService::plan(&current, &foreign)
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("config"),
        "message must name what is missing"
    );
}

#[test]
fn overlapping_and_relative_directories_are_rejected() {
    let root = TempDir::new("settings-adopt-overlap").unwrap();
    let current = root.path().join("current");
    write_config(&current, "matches: []\n");

    assert!(AdoptService::plan(&current, &current).is_err());
    assert!(AdoptService::plan(&current, &current.join("config")).is_err());
    assert!(AdoptService::plan(&current, Path::new("relative")).is_err());
}

#[test]
fn a_directory_espanso_cannot_load_is_rejected() {
    let root = TempDir::new("settings-adopt-broken").unwrap();
    let current = root.path().join("current");
    write_config(&current, "matches: []\n");

    let broken = root.path().join("broken");
    fs::create_dir_all(broken.join("config")).unwrap();
    fs::write(broken.join("config/default.yml"), "\tthis: [is not: yaml\n").unwrap();

    assert!(AdoptService::plan(&current, &broken).is_err());
}
