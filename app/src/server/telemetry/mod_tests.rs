use futures::executor::block_on;
use serde_json::json;
use warp_core::{register_telemetry_event, telemetry::{EnablementState, TelemetryEvent}};

use super::*;

#[derive(Debug)]
enum TestTelemetryEvent {
    Simple,
}

register_telemetry_event!(TestTelemetryEvent);

impl TelemetryEvent for TestTelemetryEvent {
    fn name(&self) -> &'static str {
        match self {
            Self::Simple => "test telemetry event",
        }
    }

    fn payload(&self) -> Option<serde_json::Value> {
        match self {
            Self::Simple => Some(json!({ "kind": "simple" })),
        }
    }

    fn description(&self) -> &'static str {
        "Test-only telemetry event"
    }

    fn enablement_state(&self) -> EnablementState {
        EnablementState::Always
    }

    fn contains_ugc(&self) -> bool {
        false
    }

    fn event_descs() -> impl Iterator<Item = Box<dyn warp_core::telemetry::TelemetryEventDesc>> {
        std::iter::empty()
    }
}

#[test]
fn flush_and_persist_events_is_a_noop_but_succeeds() {
    let telemetry_api = TelemetryApi::new();

    telemetry_api
        .flush_and_persist_events(10, PrivacySettingsSnapshot::mock())
        .expect("no-op telemetry persistence should succeed");
}

#[test]
fn flush_persisted_events_to_rudder_is_a_noop_but_succeeds() {
    let telemetry_api = TelemetryApi::new();
    let path = std::env::temp_dir().join("warp-telemetry-test.json");

    block_on(async {
        telemetry_api
            .flush_persisted_events_to_rudder(&path, PrivacySettingsSnapshot::mock())
            .await
            .expect("no-op persisted flush should succeed");
    });
}

#[test]
fn send_telemetry_event_is_a_noop_but_succeeds() {
    let telemetry_api = TelemetryApi::new();

    block_on(async {
        telemetry_api
            .send_telemetry_event(
                None,
                "anonymous-id".to_owned(),
                TestTelemetryEvent::Simple,
                PrivacySettingsSnapshot::mock(),
            )
            .await
            .expect("no-op telemetry send should succeed");
    });
}
