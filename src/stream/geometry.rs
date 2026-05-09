use serde::{Deserialize, Serialize};

use crate::model::{ReviewStream, RowId};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ViewportSpec {
    pub width: usize,
    pub height: usize,
}

impl ViewportSpec {
    pub fn new(width: usize, height: usize) -> Self {
        Self { width, height }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RowSpan {
    pub row_id: RowId,
    pub ordinal: usize,
    pub start: usize,
    pub end: usize,
    pub height: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LayoutSnapshot {
    pub viewport: ViewportSpec,
    pub row_spans: Vec<RowSpan>,
    pub content_height: usize,
}

impl LayoutSnapshot {
    pub fn from_stream(stream: &ReviewStream, viewport: ViewportSpec) -> Self {
        layout_stream(stream, viewport)
    }

    pub fn row_span(&self, row_id: &RowId) -> Option<&RowSpan> {
        self.row_spans.iter().find(|span| &span.row_id == row_id)
    }

    pub fn reveal_row(&self, row_id: &RowId) -> Option<RevealPosition> {
        let span = self.row_span(row_id)?;
        let viewport_height = self.viewport.height;
        let max_scroll_top = self.content_height.saturating_sub(viewport_height);
        let scroll_top = span.start.min(max_scroll_top);
        let viewport_start = scroll_top;
        let viewport_end = scroll_top.saturating_add(viewport_height);

        Some(RevealPosition {
            row_id: row_id.clone(),
            scroll_top,
            viewport_start,
            viewport_end,
            contains_target: span.start >= viewport_start && span.end <= viewport_end,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RevealPosition {
    pub row_id: RowId,
    pub scroll_top: usize,
    pub viewport_start: usize,
    pub viewport_end: usize,
    pub contains_target: bool,
}

fn layout_stream(stream: &ReviewStream, viewport: ViewportSpec) -> LayoutSnapshot {
    let row_spans = stream
        .rows
        .iter()
        .enumerate()
        .map(|(ordinal, row)| {
            // V1 intentionally defers wrapping: every logical stream row is one visual row.
            let start = ordinal;
            RowSpan {
                row_id: row.id.clone(),
                ordinal,
                start,
                end: start + 1,
                height: 1,
            }
        })
        .collect::<Vec<_>>();

    LayoutSnapshot {
        viewport,
        content_height: row_spans.len(),
        row_spans,
    }
}
