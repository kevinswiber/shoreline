use serde::{Deserialize, Serialize};

use super::HunkId;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ReviewHunk {
    pub id: HunkId,
}
