use espanso_settings::SettingsInstanceGuard;
use tempdir::TempDir;

#[test]
fn only_one_settings_instance_can_hold_the_runtime_lock() {
    let runtime = TempDir::new("settings-single-instance").unwrap();

    let first = SettingsInstanceGuard::try_acquire(runtime.path())
        .unwrap()
        .expect("first instance should acquire the lock");
    assert!(SettingsInstanceGuard::try_acquire(runtime.path())
        .unwrap()
        .is_none());

    drop(first);
    assert!(SettingsInstanceGuard::try_acquire(runtime.path())
        .unwrap()
        .is_some());
}
