use espanso_settings::{ModelEffect, ModelMessage, SettingsModel, SettingsTab, UiMatch};

#[test]
fn dirty_tab_switch_requires_confirmation() {
    let mut model = SettingsModel::default();
    assert_eq!(model.reduce(ModelMessage::DraftChanged), ModelEffect::None);
    assert!(model.is_dirty());

    assert_eq!(
        model.reduce(ModelMessage::SwitchTab(SettingsTab::Configuration)),
        ModelEffect::ConfirmDiscard
    );
    assert_eq!(model.active_tab(), SettingsTab::Settings);

    assert_eq!(
        model.reduce(ModelMessage::ConfirmDiscard),
        ModelEffect::TabChanged(SettingsTab::Configuration)
    );
    assert_eq!(model.active_tab(), SettingsTab::Configuration);
    assert!(!model.is_dirty());
}

#[test]
fn filter_is_case_insensitive_and_checks_trigger_and_replacement() {
    let mut model = SettingsModel::default();
    model.set_matches(vec![
        ui_match("one", ":Hello", "世界"),
        ui_match("two", ":bye", "Good Night"),
    ]);

    model.reduce(ModelMessage::FilterChanged("hello".to_string()));
    assert_eq!(model.filtered_matches()[0].id, "one");

    model.reduce(ModelMessage::FilterChanged("night".to_string()));
    assert_eq!(model.filtered_matches()[0].id, "two");
}

#[test]
fn deleted_match_can_be_undone_before_save() {
    let mut model = SettingsModel::default();
    model.set_matches(vec![ui_match("one", ":one", "first")]);

    assert_eq!(
        model.reduce(ModelMessage::Delete("one".to_string())),
        ModelEffect::MatchesChanged
    );
    assert!(model.matches().is_empty());
    assert!(model.is_dirty());

    assert_eq!(
        model.reduce(ModelMessage::UndoDelete),
        ModelEffect::MatchesChanged
    );
    assert_eq!(model.matches()[0].id, "one");
}

#[test]
fn discarding_dirty_navigation_restores_pending_deletion() {
    let mut model = SettingsModel::default();
    model.set_matches(vec![ui_match("one", ":one", "first")]);
    model.reduce(ModelMessage::Delete("one".to_string()));
    model.reduce(ModelMessage::SwitchTab(SettingsTab::Configuration));

    assert_eq!(
        model.reduce(ModelMessage::ConfirmDiscard),
        ModelEffect::TabChanged(SettingsTab::Configuration)
    );
    assert_eq!(model.matches()[0].id, "one");
    assert!(!model.is_dirty());
}

fn ui_match(id: &str, trigger: &str, replace: &str) -> UiMatch {
    UiMatch {
        id: id.to_string(),
        trigger: trigger.to_string(),
        replace: replace.to_string(),
    }
}
