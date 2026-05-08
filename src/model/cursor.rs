use serde::{Deserialize, Serialize};

use super::RowId;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CursorState {
    pub row_id: Option<RowId>,
}

impl CursorState {
    pub fn empty() -> Self {
        Self { row_id: None }
    }

    pub fn at_row(row_id: RowId) -> Self {
        Self {
            row_id: Some(row_id),
        }
    }
}
