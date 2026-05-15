use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    ReviewInitialized,
    ReviewUnitCaptured,
    ReviewObservationRecorded,
    ReviewDispositionRecorded,
    InterventionRequested,
    InterventionResolved,
    ReviewNoteImported,
}
