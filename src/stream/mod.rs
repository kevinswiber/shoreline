mod build;
mod geometry;
mod navigation;

pub use build::{BuiltReviewStream, build_review_stream, build_review_stream_from_sidecar};
pub use geometry::{LayoutSnapshot, RevealPosition, RowSpan, ViewportSpec};
pub use navigation::{NavigationCommand, NavigationResult, RevealTarget, navigate_review_stream};
