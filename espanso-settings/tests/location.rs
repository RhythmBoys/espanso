use std::path::{Path, PathBuf};

use espanso_settings::{resolve_config_location, ConfigLocationStore, ConfigPathSource};
use tempdir::TempDir;

#[test]
fn precedence_is_cli_then_environment_then_persisted_then_default() {
    let root = TempDir::new("settings-location-precedence").unwrap();
    let cli = create_dir(root.path(), "cli");
    let environment = create_dir(root.path(), "environment");
    let persisted = create_dir(root.path(), "persisted");
    let default = create_dir(root.path(), "default");
    let store = ConfigLocationStore::new(root.path().join("settings-location.json"));
    store.save_atomic(&persisted).unwrap();

    let resolved = resolve_config_location(
        Some(cli.as_path()),
        Some(environment.as_path()),
        &store,
        &default,
    )
    .unwrap();
    assert_eq!(resolved.path, cli.canonicalize().unwrap());
    assert_eq!(resolved.source, ConfigPathSource::Cli);

    let resolved =
        resolve_config_location(None, Some(environment.as_path()), &store, &default).unwrap();
    assert_eq!(resolved.path, environment.canonicalize().unwrap());
    assert_eq!(resolved.source, ConfigPathSource::Environment);

    let resolved = resolve_config_location(None, None, &store, &default).unwrap();
    assert_eq!(resolved.path, persisted.canonicalize().unwrap());
    assert_eq!(resolved.source, ConfigPathSource::Persisted);

    store.clear().unwrap();
    let resolved = resolve_config_location(None, None, &store, &default).unwrap();
    assert_eq!(resolved.path, default.canonicalize().unwrap());
    assert_eq!(resolved.source, ConfigPathSource::PlatformDefault);
}

#[test]
fn persisted_location_rejects_relative_and_missing_paths() {
    let root = TempDir::new("settings-location-validation").unwrap();
    let store = ConfigLocationStore::new(root.path().join("settings-location.json"));

    assert!(store.save_atomic(Path::new("relative/config")).is_err());
    assert!(store.save_atomic(&root.path().join("missing")).is_err());
    assert!(!store.path().exists());
}

fn create_dir(root: &Path, name: &str) -> PathBuf {
    let path = root.join(name);
    std::fs::create_dir(&path).unwrap();
    path
}
