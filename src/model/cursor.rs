use serde::{Deserialize, Serialize};

use super::RowId;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CursorState {
    pub row_id: Option<RowId>,
}
