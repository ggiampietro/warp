use std::borrow::Cow;

use chrono::{DateTime, Utc};
use serde_json::Value;

#[derive(Clone, Debug)]
pub struct Event {
    pub payload: EventPayload,
    pub session_created_at: DateTime<Utc>,
    pub timestamp: DateTime<Utc>,
    pub contains_ugc: bool,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum EventPayload {
    IdentifyUser {
        user_id: String,
        anonymous_id: String,
    },
    AppActive {
        user_id: Option<String>,
        anonymous_id: String,
    },
    NamedEvent {
        user_id: Option<String>,
        anonymous_id: String,
        name: Cow<'static, str>,
        value: Option<Value>,
    },
}
