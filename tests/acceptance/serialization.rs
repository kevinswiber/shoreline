use serde::{Deserialize, Serialize};
use shore::model::{
    Anchor, AnnotationId, DiffSnapshot, FileId, HunkId, LineRange, ResolutionStatus, Review,
    ReviewId, ReviewStream, RowId, Side, SnapshotId,
};

#[test]
fn top_level_review_models_round_trip_through_json() {
    let review = Review {
        id: ReviewId::new("review-1"),
    };
    let snapshot = DiffSnapshot::empty(review.id.clone());
    let stream = ReviewStream::empty(review.id.clone());

    let review_json = serde_json::to_string(&review).expect("review serializes");
    let snapshot_json = serde_json::to_string(&snapshot).expect("snapshot serializes");
    let stream_json = serde_json::to_string(&stream).expect("stream serializes");

    let decoded_review: Review = serde_json::from_str(&review_json).expect("review deserializes");
    let decoded_snapshot: DiffSnapshot =
        serde_json::from_str(&snapshot_json).expect("snapshot deserializes");
    let decoded_stream: ReviewStream =
        serde_json::from_str(&stream_json).expect("stream deserializes");

    assert_eq!(decoded_review.id, review.id);
    assert_eq!(decoded_snapshot.review_id, review.id);
    assert_eq!(decoded_stream.review_id, review.id);
}

#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
struct DurableIds {
    review_id: ReviewId,
    file_id: FileId,
    annotation_id: AnnotationId,
}

#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
struct SnapshotLocalIds {
    snapshot_id: SnapshotId,
    hunk_id: HunkId,
    row_id: RowId,
}

#[test]
fn durable_and_snapshot_local_ids_round_trip_as_distinct_types() {
    let durable = DurableIds {
        review_id: ReviewId::new("review-1"),
        file_id: FileId::new("src/lib.rs"),
        annotation_id: AnnotationId::new("annotation-1"),
    };
    let snapshot_local = SnapshotLocalIds {
        snapshot_id: SnapshotId::new("snapshot-1"),
        hunk_id: HunkId::new("hunk-1"),
        row_id: RowId::new("row-1"),
    };

    let durable_json = serde_json::to_string(&durable).expect("durable ids serialize");
    let snapshot_json =
        serde_json::to_string(&snapshot_local).expect("snapshot-local ids serialize");

    assert_eq!(
        serde_json::from_str::<DurableIds>(&durable_json).expect("durable ids deserialize"),
        durable
    );
    assert_eq!(
        serde_json::from_str::<SnapshotLocalIds>(&snapshot_json)
            .expect("snapshot-local ids deserialize"),
        snapshot_local
    );
}

#[test]
fn anchors_store_durable_location_without_row_ids() {
    let anchor = Anchor {
        file_id: FileId::new("src/lib.rs"),
        side: Side::New,
        line_range: LineRange::new(10, 12),
        hunk_signature: "sha256:hunk-body".to_owned(),
        target_text_hash: "sha256:target-text".to_owned(),
        status: ResolutionStatus::Exact,
    };

    let json = serde_json::to_value(&anchor).expect("anchor serializes");

    assert_eq!(json["file_id"], "src/lib.rs");
    assert_eq!(json["side"], "new");
    assert_eq!(json["line_range"]["start"], 10);
    assert_eq!(json["line_range"]["end"], 12);
    assert!(json.get("row_id").is_none());

    let decoded: Anchor = serde_json::from_value(json).expect("anchor deserializes");
    assert_eq!(decoded, anchor);
}
