use std::{
    fs::{self, File},
    io::Write,
    path::Path,
};

use espanso_settings::{
    BackupService, EspansoConfigValidator, BACKUP_FORMAT, BACKUP_MANIFEST_NAME,
};
use tempdir::TempDir;
use zip::{write::FileOptions, CompressionMethod, ZipWriter};

fn write_config(root: &Path, base_yml: &str) {
    fs::create_dir_all(root.join("config")).unwrap();
    fs::create_dir_all(root.join("match")).unwrap();
    fs::write(root.join("config/default.yml"), "# espanso config\n").unwrap();
    fs::write(root.join("match/base.yml"), base_yml).unwrap();
}

#[test]
fn export_then_import_round_trips_a_configuration() {
    let root = TempDir::new("settings-backup-roundtrip").unwrap();
    let source = root.path().join("source");
    write_config(
        &source,
        "matches:\n  - trigger: \":hi\"\n    replace: \"Hello\"\n",
    );

    let archive = root.path().join("pack.espanso-backup.zip");
    let summary = BackupService::export(&source, &archive).unwrap();
    assert!(archive.is_file());
    assert!(summary.file_count >= 2);

    // Source must be untouched.
    assert_eq!(
        fs::read_to_string(source.join("match/base.yml")).unwrap(),
        "matches:\n  - trigger: \":hi\"\n    replace: \"Hello\"\n"
    );

    let destination = root.path().join("restored");
    let plan = BackupService::import(&archive, &destination, &EspansoConfigValidator).unwrap();
    assert!(plan.had_manifest);
    assert_eq!(plan.adopt.match_count, 1);
    assert!(destination.join("config/default.yml").is_file());
    assert!(destination.join("match/base.yml").is_file());
    assert!(!plan.staging_root.exists());
}

#[test]
fn export_writes_a_recognisable_manifest() {
    let root = TempDir::new("settings-backup-manifest").unwrap();
    let source = root.path().join("source");
    write_config(&source, "matches: []\n");
    let archive = root.path().join("pack.zip");
    BackupService::export(&source, &archive).unwrap();

    let file = File::open(&archive).unwrap();
    let mut zip = zip::ZipArchive::new(file).unwrap();
    let mut entry = zip.by_name(BACKUP_MANIFEST_NAME).unwrap();
    let mut body = String::new();
    std::io::Read::read_to_string(&mut entry, &mut body).unwrap();
    let manifest: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(manifest["format"], BACKUP_FORMAT);
    assert_eq!(manifest["format_version"], 1);
}

#[test]
fn import_accepts_a_hand_zipped_wrapper_folder() {
    let root = TempDir::new("settings-backup-wrapper").unwrap();
    let nested = root.path().join("nested");
    let inner = nested.join("espanso");
    write_config(
        &inner,
        "matches:\n  - trigger: \":x\"\n    replace: \"X\"\n",
    );

    // Hand-zip: archive root contains a single "espanso/" folder with config/.
    let archive = root.path().join("hand.zip");
    {
        let file = File::create(&archive).unwrap();
        let mut zip = ZipWriter::new(file);
        let options = FileOptions::default().compression_method(CompressionMethod::Deflated);
        for (name, path) in [
            (
                "espanso/config/default.yml",
                inner.join("config/default.yml"),
            ),
            ("espanso/match/base.yml", inner.join("match/base.yml")),
        ] {
            zip.start_file(name, options).unwrap();
            zip.write_all(&fs::read(path).unwrap()).unwrap();
        }
        zip.finish().unwrap();
    }

    let destination = root.path().join("out");
    let plan = BackupService::import(&archive, &destination, &EspansoConfigValidator).unwrap();
    assert!(!plan.had_manifest);
    assert!(destination.join("config/default.yml").is_file());
    assert_eq!(plan.adopt.match_count, 1);
}

#[test]
fn import_rejects_a_non_empty_destination() {
    let root = TempDir::new("settings-backup-nonempty-dest").unwrap();
    let source = root.path().join("source");
    write_config(&source, "matches: []\n");
    let archive = root.path().join("pack.zip");
    BackupService::export(&source, &archive).unwrap();

    let destination = root.path().join("dest");
    fs::create_dir(&destination).unwrap();
    fs::write(destination.join("keep.txt"), "nope").unwrap();

    let error = BackupService::plan_import(&archive, &destination)
        .unwrap_err()
        .to_string();
    assert!(error.contains("empty"), "got: {error}");
}

#[test]
fn import_rejects_archives_without_a_config_folder() {
    let root = TempDir::new("settings-backup-foreign").unwrap();
    let archive = root.path().join("noise.zip");
    {
        let file = File::create(&archive).unwrap();
        let mut zip = ZipWriter::new(file);
        let options = FileOptions::default().compression_method(CompressionMethod::Deflated);
        zip.start_file("readme.txt", options).unwrap();
        zip.write_all(b"not an espanso backup").unwrap();
        zip.finish().unwrap();
    }

    let destination = root.path().join("dest");
    let error = BackupService::plan_import(&archive, &destination)
        .unwrap_err()
        .to_string();
    assert!(error.contains("config"), "got: {error}");
    assert!(!destination.exists());
}

#[test]
fn discard_import_cleans_staging() {
    let root = TempDir::new("settings-backup-discard").unwrap();
    let source = root.path().join("source");
    write_config(&source, "matches: []\n");
    let archive = root.path().join("pack.zip");
    BackupService::export(&source, &archive).unwrap();

    let destination = root.path().join("dest");
    let plan = BackupService::plan_import(&archive, &destination).unwrap();
    assert!(plan.staging_root.exists());
    BackupService::discard_import(&plan);
    assert!(!plan.staging_root.exists());
    assert!(!destination.exists());
}
