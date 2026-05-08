use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::{AnnotationId, FileId};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Annotation {
    pub id: AnnotationId,
    pub anchor: Anchor,
    pub source: AnnotationSource,
    pub summary: String,
    pub rationale: Option<String>,
    pub tags: Vec<String>,
    pub confidence: Option<String>,
    pub external_source: Option<String>,
    pub author: Option<String>,
    pub created_at: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnnotationSource {
    Sidecar,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Side {
    Old,
    New,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LineRange {
    pub start: u32,
    pub end: u32,
}

impl LineRange {
    pub fn new(start: u32, end: u32) -> Self {
        Self { start, end }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResolutionStatus {
    Exact,
    Relocated,
    FileLevel,
    Stale,
    Orphaned,
    Unresolved,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Anchor {
    pub file_id: FileId,
    pub side: Side,
    pub line_range: LineRange,
    pub hunk_signature: String,
    pub target_text_hash: String,
    pub status: ResolutionStatus,
}

pub(crate) fn hash_normalized_lines<'a>(lines: impl IntoIterator<Item = &'a str>) -> String {
    let mut payload = String::new();
    for line in lines {
        push_normalized_line(&mut payload, line);
    }
    sha256_prefixed(&payload)
}

pub(crate) fn push_normalized_line(payload: &mut String, line: &str) {
    payload.push_str(&line.replace("\r\n", "\n").replace('\r', "\n"));
    payload.push('\n');
}

pub(crate) fn sha256_prefixed(payload: &str) -> String {
    let digest = Sha256::digest(payload.as_bytes());
    format!("sha256:{digest:x}")
}
