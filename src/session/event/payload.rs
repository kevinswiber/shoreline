use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize};

use super::kind::EventType;
use crate::error::ShoreError;

pub trait EventPayload: Serialize {
    fn event_type(&self) -> EventType;
}

/// Canonical media type for note-shaped event body text.
///
/// Missing legacy fields read as `text/plain`; writers only serialize this field
/// when a body is explicitly Markdown, preserving the old plain-text event shape.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum BodyContentType {
    #[default]
    #[serde(rename = "text/plain")]
    TextPlain,
    #[serde(rename = "text/markdown")]
    TextMarkdown,
}

impl BodyContentType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::TextPlain => "text/plain",
            Self::TextMarkdown => "text/markdown",
        }
    }

    pub fn is_text_plain(&self) -> bool {
        *self == Self::TextPlain
    }

    pub(crate) fn identity_tag(self) -> Option<&'static str> {
        match self {
            Self::TextPlain => None,
            Self::TextMarkdown => Some(self.as_str()),
        }
    }
}

impl fmt::Display for BodyContentType {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for BodyContentType {
    type Err = ShoreError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "text/plain" => Ok(Self::TextPlain),
            "text/markdown" => Ok(Self::TextMarkdown),
            other => Err(ShoreError::WorkflowInputInvalid {
                reason: format!(
                    "unsupported body content type {other:?}; expected text/plain or text/markdown"
                ),
            }),
        }
    }
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
