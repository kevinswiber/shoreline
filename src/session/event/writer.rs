use serde::{Deserialize, Serialize};

use crate::model::ActorId;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Writer {
    pub actor_id: ActorId,
    pub role: WriterRole,
    pub tool: WriterTool,
}

impl Writer {
    pub fn shore_local_author(version: impl Into<String>) -> Self {
        Self {
            actor_id: ActorId::new("actor:local"),
            role: WriterRole::Author,
            tool: WriterTool {
                name: "shore".to_owned(),
                version: version.into(),
            },
        }
    }

    pub fn shore_local_reviewer(version: impl Into<String>) -> Self {
        Self {
            actor_id: ActorId::new("actor:local"),
            role: WriterRole::Reviewer,
            tool: WriterTool {
                name: "shore".to_owned(),
                version: version.into(),
            },
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WriterRole {
    Author,
    Reviewer,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WriterTool {
    pub name: String,
    pub version: String,
}
