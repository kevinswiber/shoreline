mod annotation;
mod cursor;
mod file;
mod hunk;
mod ids;
mod review;
mod row;

pub(crate) use annotation::hash_normalized_lines;
pub use annotation::{Anchor, Annotation, AnnotationSource, LineRange, ResolutionStatus, Side};
pub use cursor::CursorState;
pub use file::{DiffFile, FileStatus};
pub use hunk::ReviewHunk;
pub use ids::{AnnotationId, FileId, HunkId, ReviewId, RowId, SnapshotId};
pub use review::{DiffSnapshot, Review, ReviewStream};
pub use row::{DiffRow, DiffRowKind, FileMetadataKind, FileMetadataRow, ReviewRow, ReviewRowKind};
