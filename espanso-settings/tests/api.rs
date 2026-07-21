use std::path::PathBuf;

use espanso_settings::{ConfigPathSource, SettingsLaunchOptions, SettingsPaths};

#[test]
fn launch_options_are_constructible_without_opening_a_window() {
    let options = SettingsLaunchOptions {
        paths: SettingsPaths {
            config: PathBuf::from("/config"),
            packages: PathBuf::from("/packages"),
            runtime: PathBuf::from("/runtime"),
        },
        location_store_path: PathBuf::from("/bootstrap/settings-location.json"),
        config_override_source: ConfigPathSource::PlatformDefault,
    };

    assert_eq!(options.paths.config, PathBuf::from("/config"));
    assert!(options.config_override_source.allows_persistent_change());
}
