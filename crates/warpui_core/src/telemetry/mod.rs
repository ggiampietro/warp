mod event_store;

use chrono::{DateTime, Utc};
use serde_json::Value;
use std::borrow::Cow;

pub use event_store::{Event, EventPayload};

#[macro_export]
macro_rules! record_telemetry_from_ctx {
    ($user_id: expr, $anonymous_id: expr, $name:expr, $payload: expr, $contains_ugc: expr, $ctx: expr) => {{
        let _ = (&$ctx, &$user_id, &$anonymous_id, &$name, &$payload, &$contains_ugc);
    }};
}

#[macro_export]
macro_rules! record_telemetry_on_executor {
    ($user_id: expr, $anonymous_id: expr, $name:expr, $payload: expr, $contains_ugc: expr, $executor: expr) => {{
        let _ = (&$executor, &$user_id, &$anonymous_id, &$name, &$payload, &$contains_ugc);
    }};
}

pub fn create_event(
    user_id: Option<String>,
    anonymous_id: String,
    name: Cow<'static, str>,
    payload: Option<Value>,
    contains_ugc: bool,
    timestamp: DateTime<Utc>,
) -> Event {
    Event {
        payload: EventPayload::NamedEvent {
            user_id,
            anonymous_id,
            name,
            value: payload,
        },
        session_created_at: timestamp,
        timestamp,
        contains_ugc,
    }
}

pub fn record_event(
    user_id: Option<String>,
    anonymous_id: String,
    name: Cow<'static, str>,
    payload: Option<Value>,
    contains_ugc: bool,
    timestamp: DateTime<Utc>,
) {
    let _ = (user_id, anonymous_id, name, payload, contains_ugc, timestamp);
}

pub fn record_identify_user_event(user_id: String, anonymous_id: String, timestamp: DateTime<Utc>) {
    let _ = (user_id, anonymous_id, timestamp);
}

pub fn record_app_active_event(
    user_id: Option<String>,
    anonymous_id: String,
    timestamp: DateTime<Utc>,
) {
    let _ = (user_id, anonymous_id, timestamp);
}

pub fn flush_events() -> Vec<Event> {
    Vec::new()
}
