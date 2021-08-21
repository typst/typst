use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::process;

use anyhow::Context as _;
use codespan_reporting::diagnostic::{Diagnostic, Label};
use codespan_reporting::term::{self, termcolor, Config, Styles};
use same_file::is_same_file;
use termcolor::{ColorChoice, StandardStream, WriteColor};

use typst::diag::{Error, Tracepoint};
use typst::source::SourceStore;

fn main() {
    if let Err(error) = try_main() {
        print_error(error).unwrap();
        process::exit(1);
    }
}

/// The main compiler logic.
fn try_main() -> anyhow::Result<()> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() < 2 || args.len() > 3 {
        print_usage().unwrap();
        process::exit(2);
    }

    // Determine source and destination path.
    let src_path = Path::new(&args[1]);
    let dest_path = match args.get(2) {
        Some(path) => path.into(),
        None => {
            let name = src_path.file_name().context("source path is not a file")?;
            Path::new(name).with_extension("pdf")
        }
    };

    // Ensure that the source file is not overwritten.
    if is_same_file(src_path, &dest_path).unwrap_or(false) {
        anyhow::bail!("source and destination files are the same");
    }

    // Create a loader for fonts and files.
    let loader = typst::loading::FsLoader::new()
        .with_path("fonts")
        .with_system()
        .wrap();

    // Create the context which holds loaded source files, fonts, images and
    // cached artifacts.
    let mut ctx = typst::Context::new(loader);

    // Load the source file.
    let id = ctx.sources.load(&src_path).context("source file not found")?;

    // Typeset.
    match ctx.typeset(id) {
        // Export the PDF.
        Ok(document) => {
            let buffer = typst::export::pdf(&ctx, &document);
            fs::write(&dest_path, buffer).context("failed to write PDF file")?;
        }

        // Print diagnostics.
        Err(errors) => {
            print_diagnostics(&ctx.sources, *errors)
                .context("failed to print diagnostics")?;
        }
    }

    Ok(())
}

/// Print a usage message.
fn print_usage() -> io::Result<()> {
    let mut writer = StandardStream::stderr(ColorChoice::Always);
    let styles = Styles::default();

    writer.set_color(&styles.header_help)?;
    write!(writer, "usage")?;

    writer.set_color(&styles.header_message)?;
    writeln!(writer, ": typst document.typ [output.pdf]")?;

    writer.reset()
}

/// Print an error outside of a source file.
fn print_error(error: anyhow::Error) -> io::Result<()> {
    let mut writer = StandardStream::stderr(ColorChoice::Always);
    let styles = Styles::default();

    for (i, cause) in error.chain().enumerate() {
        writer.set_color(&styles.header_error)?;
        write!(writer, "{}", if i == 0 { "error" } else { "cause" })?;

        writer.set_color(&styles.header_message)?;
        writeln!(writer, ": {}", cause)?;
    }

    writer.reset()
}

/// Print diagnostics messages to the terminal.
fn print_diagnostics(
    sources: &SourceStore,
    errors: Vec<Error>,
) -> Result<(), codespan_reporting::files::Error> {
    let mut writer = StandardStream::stderr(ColorChoice::Always);
    let config = Config { tab_width: 2, ..Default::default() };

    for error in errors {
        // The main diagnostic.
        let diag = Diagnostic::error().with_message(error.message).with_labels(vec![
            Label::primary(error.span.source, error.span.to_range()),
        ]);

        term::emit(&mut writer, &config, sources, &diag)?;

        // Stacktrace-like helper diagnostics.
        for point in error.trace {
            let message = match point.v {
                Tracepoint::Call(Some(name)) => {
                    format!("error occured in this call of function `{}`", name)
                }
                Tracepoint::Call(None) => "error occured in this function call".into(),
                Tracepoint::Import => "error occured while importing this module".into(),
            };

            let help = Diagnostic::help().with_message(message).with_labels(vec![
                Label::primary(point.span.source, point.span.to_range()),
            ]);

            term::emit(&mut writer, &config, sources, &help)?;
        }
    }

    Ok(())
}
