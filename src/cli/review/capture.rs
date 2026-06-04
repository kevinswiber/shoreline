use std::io::Write;
use std::path::PathBuf;

use clap::Args;
use shoreline::documents::{capture_document, capture_with_lineage_document};
use shoreline::model::{ReviewUnitId, ReviewUnitLineageId};
use shoreline::session::{
    CaptureOptions, LineageAttachOptions, attach_review_unit_to_lineage, capture_worktree_review,
};

use crate::cli::json;
use crate::cli_tracing::TracingArgs;

#[derive(Debug, Args)]
pub(super) struct CaptureArgs {
    #[arg(long, default_value = ".")]
    repo: PathBuf,

    /// Attach the captured ReviewUnit to this lineage.
    #[arg(long)]
    lineage: Option<String>,

    /// Previous captured ReviewUnit in the lineage.
    #[arg(long)]
    predecessor: Option<String>,

    /// Optional Change-Id metadata to record on the lineage round.
    #[arg(long)]
    change_id: Option<String>,
}

pub(super) fn run(
    args: CaptureArgs,
    tracing: &TracingArgs,
    stdout: &mut dyn Write,
) -> Result<(), Box<dyn std::error::Error>> {
    let span = tracing::info_span!("shore.review.capture");
    let _entered = span.enter();
    tracing::debug!(command = "review.capture", "command_start");
    if args.predecessor.is_some() && args.lineage.is_none() {
        return Err("predecessor requires --lineage".into());
    }
    let capture = capture_worktree_review(capture_options(&args, tracing))?;
    let Some(lineage) = args.lineage.as_ref() else {
        let document = capture_document(capture);
        return json::write_json(stdout, &document, false);
    };
    let attach = attach_review_unit_to_lineage(capture_lineage_attach_options(
        &args,
        lineage,
        &capture.review_unit_id,
    ))?;
    let document = capture_with_lineage_document(capture, attach);
    json::write_json(stdout, &document, false)
}

fn capture_options(args: &CaptureArgs, tracing: &TracingArgs) -> CaptureOptions {
    let mut options = CaptureOptions::new(&args.repo);
    if let Some(log_file) = &tracing.log_file {
        options = options.with_excluded_helper_path(log_file);
    }
    options
}

fn capture_lineage_attach_options(
    args: &CaptureArgs,
    lineage: &str,
    review_unit_id: &ReviewUnitId,
) -> LineageAttachOptions {
    let mut options =
        LineageAttachOptions::new(&args.repo, ReviewUnitLineageId::new(lineage.to_owned()))
            .with_review_unit_id(review_unit_id.clone());
    if let Some(predecessor) = &args.predecessor {
        options = options.with_predecessor_review_unit_id(ReviewUnitId::new(predecessor.clone()));
    }
    if let Some(change_id) = &args.change_id {
        options = options.with_change_id(change_id.clone());
    }
    options
}
