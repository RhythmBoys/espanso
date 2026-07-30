use std::{cell::RefCell, path::Path, rc::Rc};

use anyhow::{Context, Result};
use slint::{ComponentHandle, ModelRc, SharedString, VecModel};

use crate::{
    ConfigLocationStore, EspansoConfigValidator, MatchRepository, MigrationService, ModelEffect,
    ModelMessage, SettingsLaunchOptions, SettingsModel, SettingsTab, UiMatch, UiMatchRepository,
};

pub fn run(options: SettingsLaunchOptions) -> Result<()> {
    let Some(_instance_guard) = crate::SettingsInstanceGuard::try_acquire(&options.paths.runtime)?
    else {
        crate::single_instance::notify_existing(&options.paths.runtime);
        return Ok(());
    };
    let window = crate::SettingsWindow::new()?;
    crate::single_instance::spawn_focus_listener(&options.paths.runtime, window.as_weak())?;
    let repository = Rc::new(UiMatchRepository::new(&options.paths.config));
    let mut initial_model = SettingsModel::default();
    let (initial_matches, load_error) = match repository.load() {
        Ok(matches) => (matches, None),
        Err(error) => (Vec::new(), Some(error)),
    };
    initial_model.set_matches(initial_matches);
    let model = Rc::new(RefCell::new(initial_model));

    window.set_config_path(path_text(&options.paths.config));
    window.set_source_label(options.config_override_source.display_name().into());
    window.set_can_change_location(options.config_override_source.allows_persistent_change());
    if let Some(error) = load_error {
        window.set_error_message(format!("Cannot load matches; file unchanged: {error}").into());
    }
    refresh_rows(&window, &model.borrow());

    bind_navigation(&window, Rc::clone(&model));
    bind_match_editor(&window, Rc::clone(&model), Rc::clone(&repository));
    bind_location(&window, options);

    window.run()?;
    Ok(())
}

fn bind_navigation(window: &crate::SettingsWindow, model: Rc<RefCell<SettingsModel>>) {
    let weak = window.as_weak();
    let switch_model = Rc::clone(&model);
    window.on_request_tab_switch(move |index| {
        let tab = if index == 0 {
            SettingsTab::Settings
        } else {
            SettingsTab::Configuration
        };
        let effect = switch_model
            .borrow_mut()
            .reduce(ModelMessage::SwitchTab(tab));
        if let Some(window) = weak.upgrade() {
            match effect {
                ModelEffect::ConfirmDiscard => window.set_confirm_discard_visible(true),
                ModelEffect::TabChanged(tab) => window.set_active_tab(tab_index(tab)),
                ModelEffect::None | ModelEffect::MatchesChanged => {}
            }
        }
    });

    let weak = window.as_weak();
    let confirm_model = Rc::clone(&model);
    window.on_confirm_discard(move || {
        let effect = confirm_model
            .borrow_mut()
            .reduce(ModelMessage::ConfirmDiscard);
        if let Some(window) = weak.upgrade() {
            window.set_confirm_discard_visible(false);
            if let ModelEffect::TabChanged(tab) = effect {
                window.set_active_tab(tab_index(tab));
            }
        }
    });

    let weak = window.as_weak();
    let cancel_model = Rc::clone(&model);
    window.on_cancel_discard(move || {
        cancel_model
            .borrow_mut()
            .reduce(ModelMessage::CancelDiscard);
        if let Some(window) = weak.upgrade() {
            window.set_confirm_discard_visible(false);
        }
    });

    let draft_model = Rc::clone(&model);
    window.on_draft_changed(move || {
        draft_model.borrow_mut().reduce(ModelMessage::DraftChanged);
    });
}

fn bind_match_editor(
    window: &crate::SettingsWindow,
    model: Rc<RefCell<SettingsModel>>,
    repository: Rc<UiMatchRepository>,
) {
    let weak = window.as_weak();
    let select_model = Rc::clone(&model);
    window.on_select_match(move |id| {
        let selected = select_model
            .borrow()
            .matches()
            .iter()
            .find(|item| item.id == id.as_str())
            .cloned();
        if let (Some(window), Some(selected)) = (weak.upgrade(), selected) {
            window.set_selected_id(selected.id.into());
            window.set_trigger_text(selected.trigger.into());
            window.set_replacement_text(selected.replace.into());
            window.set_status_message("Match loaded; edit it and save".into());
        }
    });

    let weak = window.as_weak();
    window.on_new_match(move || {
        if let Some(window) = weak.upgrade() {
            window.set_selected_id(SharedString::default());
            window.set_trigger_text(SharedString::default());
            window.set_replacement_text(SharedString::default());
            window.set_status_message("Creating a new match".into());
        }
    });

    let weak = window.as_weak();
    let save_model = Rc::clone(&model);
    let save_repository = Rc::clone(&repository);
    window.on_save_match(move || {
        let Some(window) = weak.upgrade() else {
            return;
        };
        let mut id = window.get_selected_id().to_string();
        if id.is_empty() {
            id = next_match_id();
        }
        let candidate = UiMatch {
            id: id.clone(),
            trigger: window.get_trigger_text().to_string(),
            replace: window.get_replacement_text().to_string(),
        };
        let mut matches = save_model.borrow().matches().to_vec();
        if let Some(existing) = matches.iter_mut().find(|item| item.id == id) {
            *existing = candidate;
        } else {
            matches.push(candidate);
        }

        match save_repository.save(&matches) {
            Ok(()) => {
                save_model.borrow_mut().set_matches(matches);
                save_model.borrow_mut().reduce(ModelMessage::Saved);
                window.set_selected_id(id.into());
                window.set_error_message(SharedString::default());
                window.set_status_message("Match saved; Espanso will reload automatically".into());
                refresh_rows(&window, &save_model.borrow());
            }
            Err(error) => {
                window.set_error_message(error.to_string().into());
            }
        }
    });

    let weak = window.as_weak();
    let delete_model = Rc::clone(&model);
    window.on_delete_match(move |id| {
        let effect = delete_model
            .borrow_mut()
            .reduce(ModelMessage::Delete(id.to_string()));
        if let Some(window) = weak.upgrade() {
            if effect == ModelEffect::MatchesChanged {
                window.set_selected_id(SharedString::default());
                window.set_trigger_text(SharedString::default());
                window.set_replacement_text(SharedString::default());
                window.set_undo_visible(true);
                window.set_status_message("Match removed; you can undo before saving".into());
                refresh_rows(&window, &delete_model.borrow());
            }
        }
    });

    let weak = window.as_weak();
    let undo_model = Rc::clone(&model);
    window.on_undo_delete(move || {
        let effect = undo_model.borrow_mut().reduce(ModelMessage::UndoDelete);
        if let Some(window) = weak.upgrade() {
            if effect == ModelEffect::MatchesChanged {
                window.set_undo_visible(false);
                window.set_status_message("Deletion undone".into());
                refresh_rows(&window, &undo_model.borrow());
            }
        }
    });

    let weak = window.as_weak();
    let commit_model = Rc::clone(&model);
    let commit_repository = Rc::clone(&repository);
    window.on_commit_deletion(move || {
        let Some(window) = weak.upgrade() else {
            return;
        };
        match commit_repository.save(commit_model.borrow().matches()) {
            Ok(()) => {
                commit_model.borrow_mut().reduce(ModelMessage::Saved);
                window.set_undo_visible(false);
                window.set_error_message(SharedString::default());
                window.set_status_message("Deletion saved safely".into());
            }
            Err(error) => window.set_error_message(error.to_string().into()),
        }
    });

    let weak = window.as_weak();
    let filter_model = Rc::clone(&model);
    window.on_apply_filter(move |filter| {
        filter_model
            .borrow_mut()
            .reduce(ModelMessage::FilterChanged(filter.to_string()));
        if let Some(window) = weak.upgrade() {
            refresh_rows(&window, &filter_model.borrow());
        }
    });
}

fn bind_location(window: &crate::SettingsWindow, options: SettingsLaunchOptions) {
    let weak = window.as_weak();
    let current_config = options.paths.config.clone();
    window.on_choose_folder(move || {
        let selected = rfd::FileDialog::new()
            .set_directory(&current_config)
            .pick_folder();
        if let (Some(window), Some(selected)) = (weak.upgrade(), selected) {
            match MigrationService::preflight(&current_config, &selected) {
                Ok(plan) => {
                    window.set_selected_path(path_text(&selected));
                    window.set_migration_summary(
                        format!(
                            "Will copy {} file(s) ({} bytes). The old directory will be kept.",
                            plan.file_count, plan.byte_count
                        )
                        .into(),
                    );
                    window.set_migration_ready(plan.conflicts.is_empty());
                    window.set_error_message(SharedString::default());
                }
                Err(error) => window.set_error_message(error.to_string().into()),
            }
        }
    });

    let weak = window.as_weak();
    let config_to_open = options.paths.config.clone();
    let migration_source = options.paths.config;
    let store_path = options.location_store_path;
    window.on_migrate(move || {
        let Some(window) = weak.upgrade() else {
            return;
        };
        let destination = std::path::PathBuf::from(window.get_selected_path().to_string());
        window.set_migration_ready(false);
        window.set_status_message("Validating and copying the configuration...".into());
        let source = migration_source.clone();
        let store_path = store_path.clone();
        let weak = window.as_weak();
        std::thread::spawn(move || {
            let result = MigrationService::preflight(&source, &destination).and_then(|plan| {
                MigrationService::execute(&plan, &EspansoConfigValidator)?;
                ConfigLocationStore::new(store_path).save_atomic(&destination)
            });
            let _ = weak.upgrade_in_event_loop(move |window| match result {
                Ok(()) => {
                    window.set_config_path(path_text(&destination));
                    window.set_status_message(
                        "Migration complete; old directory kept. Restart Espanso.".into(),
                    );
                    window.set_error_message(SharedString::default());
                }
                Err(error) => {
                    window.set_error_message(format!("Migration failed: {error}").into());
                }
            });
        });
    });

    let weak = window.as_weak();
    window.on_open_config(move || {
        if let Err(error) = open_path(&config_to_open) {
            if let Some(window) = weak.upgrade() {
                window.set_error_message(error.to_string().into());
            }
        }
    });
}

fn refresh_rows(window: &crate::SettingsWindow, model: &SettingsModel) {
    let rows = model
        .filtered_matches()
        .into_iter()
        .map(|item| crate::MatchRow {
            id: item.id.clone().into(),
            trigger: item.trigger.clone().into(),
            preview: item.replace.lines().next().unwrap_or_default().into(),
        })
        .collect::<Vec<_>>();
    window.set_match_rows(ModelRc::from(Rc::new(VecModel::from(rows))));
}

fn path_text(path: &Path) -> SharedString {
    path.to_string_lossy().to_string().into()
}

const fn tab_index(tab: SettingsTab) -> i32 {
    match tab {
        SettingsTab::Settings => 0,
        SettingsTab::Configuration => 1,
    }
}

fn next_match_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let value = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_millis());
    format!("ui-{value}")
}

fn open_path(path: &Path) -> Result<()> {
    opener::open(path).with_context(|| "unable to open the configuration directory")
}
