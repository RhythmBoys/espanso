use std::fs;

use espanso_settings::scan_external_matches;
use tempdir::TempDir;

#[test]
fn matches_from_other_files_are_collected_and_the_owned_file_is_skipped() {
    let root = TempDir::new("settings-inventory-scan").unwrap();
    let config_root = root.path().join("config-root");
    let match_dir = config_root.join("match");
    fs::create_dir_all(&match_dir).unwrap();

    let owned = match_dir.join("ui.yml");
    fs::write(
        &owned,
        "matches:\n  - trigger: \":owned\"\n    replace: \"mine\"\n",
    )
    .unwrap();
    fs::write(
        match_dir.join("base.yml"),
        "matches:\n  - trigger: \":hi\"\n    replace: \"Hello\\nthere\"\n",
    )
    .unwrap();

    let external = scan_external_matches(&config_root, &owned);

    assert_eq!(external.len(), 1);
    assert_eq!(external[0].trigger, ":hi");
    assert_eq!(external[0].preview, "Hello");
    assert_eq!(external[0].source_label, "match/base.yml");
}

#[test]
fn advanced_matches_say_specifically_why_they_are_read_only() {
    let root = TempDir::new("settings-inventory-notes").unwrap();
    let config_root = root.path().join("config-root");
    let match_dir = config_root.join("match");
    fs::create_dir_all(&match_dir).unwrap();

    fs::write(
        match_dir.join("advanced.yml"),
        r#"matches:
  - regex: "greet(?P<name>.*)"
    replace: "Hi {{name}}"
  - trigger: ":form"
    form: "Hey [[name]]"
  - trigger: ":date"
    replace: "{{mydate}}"
    vars:
      - name: mydate
        type: date
"#,
    )
    .unwrap();

    let external = scan_external_matches(&config_root, &match_dir.join("ui.yml"));
    let notes = external
        .iter()
        .map(|item| item.note.as_str())
        .collect::<Vec<_>>();

    assert_eq!(external.len(), 3);
    assert!(notes.iter().any(|note| note.contains("regex")));
    assert!(notes.iter().any(|note| note.contains("form")));
    assert!(notes.iter().any(|note| note.contains("variables")));
}

#[test]
fn unreadable_files_and_a_missing_match_directory_are_skipped_quietly() {
    let root = TempDir::new("settings-inventory-tolerant").unwrap();
    let config_root = root.path().join("config-root");
    let match_dir = config_root.join("match");
    fs::create_dir_all(&match_dir).unwrap();

    fs::write(match_dir.join("broken.yml"), "\tmatches: [oops\n").unwrap();
    fs::write(match_dir.join("notes.txt"), "not yaml at all").unwrap();
    fs::write(
        match_dir.join("good.yml"),
        "matches:\n  - trigger: \":ok\"\n    replace: \"fine\"\n",
    )
    .unwrap();

    // One broken neighbour must not stop the editor from listing the rest.
    let external = scan_external_matches(&config_root, &match_dir.join("ui.yml"));
    assert_eq!(external.len(), 1);
    assert_eq!(external[0].trigger, ":ok");

    let empty = scan_external_matches(&root.path().join("no-such-root"), &match_dir.join("ui.yml"));
    assert!(empty.is_empty());
}
