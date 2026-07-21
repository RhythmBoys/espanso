use std::{
    fs::{self, File, OpenOptions},
    path::Path,
};

use anyhow::{Context, Result};
#[cfg(feature = "ui")]
use espanso_ipc::{EventHandlerResponse, IPCClient, IPCServer};
use fs2::FileExt;
#[cfg(feature = "ui")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "ui")]
use slint::ComponentHandle;

#[cfg(feature = "ui")]
const SETTINGS_IPC_ID: &str = "espansosettingsv1";

#[cfg(feature = "ui")]
#[derive(Debug, Serialize, Deserialize)]
enum SettingsIPCEvent {
    Focus,
}

pub struct SettingsInstanceGuard {
    file: File,
}

impl SettingsInstanceGuard {
    pub fn try_acquire(runtime_dir: &Path) -> Result<Option<Self>> {
        fs::create_dir_all(runtime_dir).with_context(|| "unable to create runtime directory")?;
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(runtime_dir.join("espanso-settings.lock"))?;
        match FileExt::try_lock_exclusive(&file) {
            Ok(()) => Ok(Some(Self { file })),
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => Ok(None),
            Err(error) => Err(error.into()),
        }
    }
}

impl Drop for SettingsInstanceGuard {
    fn drop(&mut self) {
        let _ = FileExt::unlock(&self.file);
    }
}

#[cfg(feature = "ui")]
pub(crate) fn notify_existing(runtime_dir: &Path) {
    for _ in 0..10 {
        if let Ok(mut client) =
            espanso_ipc::client::<SettingsIPCEvent>(SETTINGS_IPC_ID, runtime_dir)
        {
            let _ = client.send_async(SettingsIPCEvent::Focus);
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
}

#[cfg(feature = "ui")]
pub(crate) fn spawn_focus_listener(
    runtime_dir: &Path,
    window: slint::Weak<crate::SettingsWindow>,
) -> Result<()> {
    let server = espanso_ipc::server::<SettingsIPCEvent>(SETTINGS_IPC_ID, runtime_dir)?;
    std::thread::Builder::new()
        .name("settings-focus-ipc".to_string())
        .spawn(move || {
            let _ = server.run(Box::new(move |event| match event {
                SettingsIPCEvent::Focus => {
                    let _ = window.upgrade_in_event_loop(|window| {
                        let _ = window.show();
                    });
                    EventHandlerResponse::NoResponse
                }
            }));
        })?;
    Ok(())
}
