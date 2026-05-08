use serde::{Deserialize, Serialize};

use super::ReviewId;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Review {
    pub id: ReviewId,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DiffSnapshot {
    pub review_id: ReviewId,
}

impl DiffSnapshot {
    pub fn empty(review_id: ReviewId) -> Self {
        Self { review_id }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ReviewStream {
    pub review_id: ReviewId,
}

impl ReviewStream {
    pub fn empty(review_id: ReviewId) -> Self {
        Self { review_id }
    }
}
