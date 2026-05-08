use serde::{Deserialize, Serialize};

use super::RowId;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ReviewRow {
    pub id: RowId,
}
