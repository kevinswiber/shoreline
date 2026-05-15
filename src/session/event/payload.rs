use serde::{Deserialize, Deserializer, Serialize};

use super::kind::EventType;

pub trait EventPayload: Serialize {
    fn event_type(&self) -> EventType;
}

pub(super) fn deserialize_non_empty_idempotency_key<'de, D>(
    deserializer: D,
) -> std::result::Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    if value.trim().is_empty() {
        return Err(serde::de::Error::custom("idempotencyKey cannot be empty"));
    }

    Ok(value)
}
