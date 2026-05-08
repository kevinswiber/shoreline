use serde::{Deserialize, Serialize};

use super::FileId;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DiffFile {
    pub id: FileId,
}
