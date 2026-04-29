mod context;
pub mod context_provider;
mod events;
mod macros;
pub mod secret_redaction;

pub use context::telemetry_context;
pub use events::*;

use crate::auth::UserUid;
use crate::settings::PrivacySettingsSnapshot;
use anyhow::Result;
use std::path::Path;


pub struct TelemetryApi {
    pub(super) client: http_client::Client,
}

impl Default for TelemetryApi {
    fn default() -> Self {
        Self::new()
    }
}

impl TelemetryApi {
    pub fn new() -> Self {
        Self {
            client: http_client::Client::default(),
        }
    }

    pub async fn flush_events(&self, _settings_snapshot: PrivacySettingsSnapshot) -> Result<usize> {
        Ok(0)
    }

    pub async fn flush_persisted_events_to_rudder(
        &self,
        _path: &Path,
        _settings_snapshot: PrivacySettingsSnapshot,
    ) -> Result<()> {
        Ok(())
    }

    pub fn flush_and_persist_events(
        &self,
        _max_event_count: usize,
        _settings_snapshot: PrivacySettingsSnapshot,
    ) -> Result<()> {
        Ok(())
    }

    pub async fn send_telemetry_event(
        &self,
        _user_id: Option<UserUid>,
        _anonymous_id: String,
        _event: impl warp_core::telemetry::TelemetryEvent,
        _settings_snapshot: PrivacySettingsSnapshot,
    ) -> Result<()> {
        Ok(())
    }
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
