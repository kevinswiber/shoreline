use crate::model::{
    Annotation, DiffFile, DiffRow, DiffSnapshot, FileId, HunkId, ReviewRow, ReviewRowKind,
    ReviewStream, RowId,
};
use crate::sidecar::{AgentContext, SidecarDiagnostic, apply_file_order, resolve_annotations};

fn build_review_stream(snapshot: &DiffSnapshot, annotations: &[Annotation]) -> ReviewStream {
    let builder = StreamBuilder::new(snapshot, annotations);
    builder.build()
}

impl ReviewStream {
    pub fn from_snapshot_and_annotations(
        snapshot: &DiffSnapshot,
        annotations: &[Annotation],
    ) -> Self {
        build_review_stream(snapshot, annotations)
    }

    pub fn from_snapshot_and_sidecar(
        snapshot: &DiffSnapshot,
        context: &AgentContext,
    ) -> BuiltReviewStream {
        build_review_stream_from_sidecar(snapshot, context)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BuiltReviewStream {
    pub stream: ReviewStream,
    pub diagnostics: Vec<SidecarDiagnostic>,
}

fn build_review_stream_from_sidecar(
    snapshot: &DiffSnapshot,
    context: &AgentContext,
) -> BuiltReviewStream {
    let ordered = apply_file_order(snapshot.files.clone(), context);
    let ordered_snapshot = DiffSnapshot::new(
        snapshot.review_id.clone(),
        snapshot.snapshot_id.clone(),
        ordered.files,
    );
    let resolved = resolve_annotations(&ordered_snapshot.files, context);
    let mut diagnostics = Vec::new();
    extend_unique_diagnostics(&mut diagnostics, ordered.diagnostics);
    extend_unique_diagnostics(&mut diagnostics, resolved.diagnostics);

    BuiltReviewStream {
        stream: build_review_stream(&ordered_snapshot, &resolved.annotations),
        diagnostics,
    }
}

fn extend_unique_diagnostics(
    diagnostics: &mut Vec<SidecarDiagnostic>,
    new_diagnostics: Vec<SidecarDiagnostic>,
) {
    for diagnostic in new_diagnostics {
        if !diagnostics.contains(&diagnostic) {
            diagnostics.push(diagnostic);
        }
    }
}

struct StreamBuilder<'a> {
    snapshot: &'a DiffSnapshot,
    annotations: &'a [Annotation],
    rows: Vec<ReviewRow>,
}

impl<'a> StreamBuilder<'a> {
    fn new(snapshot: &'a DiffSnapshot, annotations: &'a [Annotation]) -> Self {
        Self {
            snapshot,
            annotations,
            rows: Vec::new(),
        }
    }

    fn build(mut self) -> ReviewStream {
        if self.snapshot.files.is_empty() {
            self.push_row(
                None,
                None,
                ReviewRowKind::EmptyState {
                    message: "no changes".to_owned(),
                },
            );
        } else {
            for file in &self.snapshot.files {
                self.push_file(file);
            }
        }

        ReviewStream {
            review_id: self.snapshot.review_id.clone(),
            snapshot_id: self.snapshot.snapshot_id.clone(),
            rows: self.rows,
        }
    }

    fn push_file(&mut self, file: &DiffFile) {
        self.push_row(
            Some(file.id.clone()),
            None,
            ReviewRowKind::FileHeader {
                path: display_path(file),
                status: file.status.clone(),
            },
        );

        for metadata in &file.metadata_rows {
            self.push_row(
                Some(file.id.clone()),
                None,
                ReviewRowKind::Metadata {
                    metadata: metadata.clone(),
                },
            );
        }

        for hunk in &file.hunks {
            let hunk_signature = hunk.signature();
            self.push_row(
                Some(file.id.clone()),
                Some(hunk.id.clone()),
                ReviewRowKind::HunkHeader {
                    header: hunk.header.clone(),
                },
            );

            for diff_row in &hunk.rows {
                let target_row_id = self.push_row(
                    Some(file.id.clone()),
                    Some(hunk.id.clone()),
                    ReviewRowKind::Diff {
                        row: diff_row.clone(),
                    },
                );

                for annotation in self.annotation_rows_for_row(file, &hunk_signature, diff_row) {
                    self.push_row(
                        Some(file.id.clone()),
                        Some(hunk.id.clone()),
                        ReviewRowKind::Annotation {
                            annotation_id: annotation.annotation_id,
                            target_row_id: target_row_id.clone(),
                            summary: annotation.summary,
                        },
                    );
                }
            }
        }
    }

    fn annotation_rows_for_row(
        &self,
        file: &DiffFile,
        hunk_signature: &str,
        row: &DiffRow,
    ) -> Vec<AnnotationRowData> {
        self.annotations
            .iter()
            .filter(|annotation| {
                annotation.anchor.file_id == file.id
                    && annotation.anchor.hunk_signature == hunk_signature
                    && row
                        .line_on_side(annotation.anchor.side)
                        .is_some_and(|line| line == annotation.anchor.line_range.end)
            })
            .map(|annotation| AnnotationRowData {
                annotation_id: annotation.id.clone(),
                summary: annotation.summary.clone(),
            })
            .collect()
    }

    fn push_row(
        &mut self,
        file_id: Option<FileId>,
        hunk_id: Option<HunkId>,
        kind: ReviewRowKind,
    ) -> RowId {
        let ordinal = self.rows.len();
        let id = RowId::new(format!("row:{ordinal:04}"));
        self.rows.push(ReviewRow {
            id: id.clone(),
            ordinal,
            file_id,
            hunk_id,
            kind,
        });
        id
    }
}

struct AnnotationRowData {
    annotation_id: crate::model::AnnotationId,
    summary: String,
}

fn display_path(file: &DiffFile) -> String {
    file.new_path
        .clone()
        .or_else(|| file.old_path.clone())
        .unwrap_or_else(|| file.id.as_str().to_owned())
}
