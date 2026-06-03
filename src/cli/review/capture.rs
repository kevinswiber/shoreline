use std::io::Write;
use std::path::PathBuf;

use clap::Args;
use shoreline::documents::capture_document;
use shoreline::session::{CaptureOptions, capture_worktree_review};

use crate::cli::json;
use crate::cli_tracing::TracingArgs;

#[derive(Debug, Args)]
pub(super) struct CaptureArgs {
    #[arg(long, default_value = ".")]
    repo: PathBuf,
}

pub(super) fn run(
    args: CaptureArgs,
    tracing: &TracingArgs,
    stdout: &mut dyn Write,
) -> Result<(), Box<dyn std::error::Error>> {
    let span = tracing::info_span!("shore.review.capture");
    let _entered = span.enter();
    tracing::debug!(command = "review.capture", "command_start");
    let result = capture_worktree_review(capture_options(&args, tracing));
    let document = capture_document(result?);
    json::write_json(stdout, &document, false)
}

fn capture_options(args: &CaptureArgs, tracing: &TracingArgs) -> CaptureOptions {
    let mut options = CaptureOptions::new(&args.repo);
    if let Some(log_file) = &tracing.log_file {
        options = options.with_excluded_helper_path(log_file);
    }
    options
}
