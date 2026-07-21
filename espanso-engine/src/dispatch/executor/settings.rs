use anyhow::Result;
use log::error;

use crate::{
    dispatch::Executor,
    event::{Event, EventType},
};

pub trait SettingsHandler {
    fn show_settings(&self) -> Result<()>;
}

pub struct SettingsExecutor<'a> {
    handler: &'a dyn SettingsHandler,
}

impl<'a> SettingsExecutor<'a> {
    pub fn new(handler: &'a dyn SettingsHandler) -> Self {
        Self { handler }
    }
}

impl Executor for SettingsExecutor<'_> {
    fn execute(&self, event: &Event) -> bool {
        if matches!(event.etype, EventType::ShowSettings) {
            if let Err(error) = self.handler.show_settings() {
                error!("settings handler reported an error: {error:?}");
            }
            true
        } else {
            false
        }
    }
}
