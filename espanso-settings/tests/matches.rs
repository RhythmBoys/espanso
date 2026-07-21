use std::fs;

use espanso_settings::{MatchRepository, UiMatch, UiMatchRepository};
use tempdir::TempDir;

#[test]
fn round_trip_preserves_multiline_unicode_and_long_text() {
    let root = TempDir::new("settings-matches-roundtrip").unwrap();
    create_config(root.path());
    let repository = UiMatchRepository::new(root.path());
    let long = format!("第一行 🦀\n{}\n最后一行", "x".repeat(100_000));
    let matches = vec![UiMatch {
        id: "hello".to_string(),
        trigger: ":你好".to_string(),
        replace: long.clone(),
    }];

    repository.save(&matches).unwrap();
    let loaded = repository.load().unwrap();

    assert_eq!(loaded, matches);
    assert_eq!(loaded[0].replace, long);
    assert!(repository.path().ends_with("match/ui.yml"));
}

#[test]
fn invalid_matches_never_replace_the_last_valid_file() {
    let root = TempDir::new("settings-matches-validation").unwrap();
    create_config(root.path());
    let repository = UiMatchRepository::new(root.path());
    let valid = vec![UiMatch {
        id: "valid".to_string(),
        trigger: ":ok".to_string(),
        replace: "works".to_string(),
    }];
    repository.save(&valid).unwrap();
    let before = fs::read(repository.path()).unwrap();

    let duplicate = vec![
        UiMatch {
            id: "one".to_string(),
            trigger: ":same".to_string(),
            replace: "one".to_string(),
        },
        UiMatch {
            id: "two".to_string(),
            trigger: ":same".to_string(),
            replace: "two".to_string(),
        },
    ];
    assert!(repository.save(&duplicate).is_err());
    assert_eq!(fs::read(repository.path()).unwrap(), before);

    let newline_trigger = vec![UiMatch {
        id: "bad".to_string(),
        trigger: ":bad\ntrigger".to_string(),
        replace: "bad".to_string(),
    }];
    assert!(repository.save(&newline_trigger).is_err());
    assert_eq!(fs::read(repository.path()).unwrap(), before);
}

#[test]
fn invalid_external_yaml_blocks_save_and_preserves_managed_file() {
    let root = TempDir::new("settings-matches-external-invalid").unwrap();
    create_config(root.path());
    let repository = UiMatchRepository::new(root.path());
    let original = vec![UiMatch {
        id: "original".to_string(),
        trigger: ":original".to_string(),
        replace: "safe".to_string(),
    }];
    repository.save(&original).unwrap();
    let before = fs::read(repository.path()).unwrap();
    fs::write(root.path().join("match/broken.yml"), "matches: [\n").unwrap();

    let changed = vec![UiMatch {
        id: "changed".to_string(),
        trigger: ":changed".to_string(),
        replace: "must not publish".to_string(),
    }];
    assert!(repository.save(&changed).is_err());
    assert_eq!(fs::read(repository.path()).unwrap(), before);
}

#[test]
fn candidate_is_validated_with_the_complete_configuration_before_publish() {
    let root = TempDir::new("settings-matches-candidate-validation").unwrap();
    create_config(root.path());
    fs::write(
        root.path().join("match/external.yml"),
        "matches:\n  - trigger: ':external'\n    replace: external\n",
    )
    .unwrap();
    let repository = UiMatchRepository::new(root.path());
    let original = vec![UiMatch {
        id: "original".to_string(),
        trigger: ":original".to_string(),
        replace: "safe".to_string(),
    }];
    repository.save(&original).unwrap();
    let before = fs::read(repository.path()).unwrap();

    let conflicting = vec![UiMatch {
        id: "conflicting".to_string(),
        trigger: ":external".to_string(),
        replace: "must not publish".to_string(),
    }];
    assert!(repository.save(&conflicting).is_err());
    assert_eq!(fs::read(repository.path()).unwrap(), before);
}

fn create_config(root: &std::path::Path) {
    fs::create_dir_all(root.join("config")).unwrap();
    fs::create_dir_all(root.join("match")).unwrap();
    fs::write(root.join("config/default.yml"), "matches: []\n").unwrap();
}
