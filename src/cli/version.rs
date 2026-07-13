use std::io::Write;

use clap::Args;
use pointbreak::documents::{DiagnosticDocument, VERSION_SCHEMA, VersionBody};

use crate::cli::output::{self, FormatArgs};

/// Report CLI and document versions for compatibility checks.
#[derive(Debug, Args)]
pub(super) struct VersionArgs {
    #[command(flatten)]
    format: FormatArgs,
}

pub(super) fn run(
    args: VersionArgs,
    stdout: &mut dyn Write,
) -> Result<(), Box<dyn std::error::Error>> {
    let format = output::resolve_format(args.format.explicit(), output::OutputFormat::Json)?;
    let body = VersionBody::current();
    let text_source = matches!(format.format, output::OutputFormat::Text).then(|| body.clone());
    let document = DiagnosticDocument::new(VERSION_SCHEMA, body, vec![]);
    output::write_document(stdout, format, &document, || {
        render_version_text(
            text_source
                .as_ref()
                .expect("text lane resolves the version source"),
        )
    })
}

fn render_version_text(body: &VersionBody) -> String {
    format!(
        "shore {}\ndocuments: {}",
        body.cli_version,
        body.documents.len()
    )
}
