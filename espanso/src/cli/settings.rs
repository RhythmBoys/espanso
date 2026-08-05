use super::{CliModule, CliModuleArgs};

pub fn new() -> CliModule {
    CliModule {
        requires_paths: true,
        subcommand: "settings".to_string(),
        show_in_dock: true,
        entry: settings_main,
        ..Default::default()
    }
}

fn settings_main(args: CliModuleArgs) -> i32 {
    let paths = args.paths.expect("missing paths in settings main");
    let options = espanso_settings::SettingsLaunchOptions {
        paths: espanso_settings::SettingsPaths {
            config: paths.config,
            packages: paths.packages,
            runtime: paths.runtime,
        },
        location_store_path: args
            .location_store_path
            .expect("missing configuration location store path"),
        config_override_source: args
            .config_path_source
            .unwrap_or(espanso_settings::ConfigPathSource::PlatformDefault),
        templates: espanso_settings::ConfigTemplates {
            default_yml: crate::config::DEFAULT_CONFIG_FILE_CONTENT.to_string(),
            base_yml: crate::config::DEFAULT_MATCH_FILE_CONTENT.to_string(),
        },
    };

    if let Err(error) = espanso_settings::run(options) {
        crate::error_eprintln!("unable to open Settings: {error:?}");
        return 1;
    }
    0
}
