use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process;

use anyhow::Context as _;
use codespan_reporting::diagnostic::{Diagnostic, Label};
use codespan_reporting::term::{self, termcolor, Config, Styles};
use same_file::is_same_file;
use termcolor::{ColorChoice, StandardStream, WriteColor};

use typst::diag::Error;
use typst::source::SourceStore;

fn main() {
    if let Err(error) = try_main() {
        print_error(error).unwrap();
        process::exit(1);
    }
}

/// The main compiler logic.
fn try_main() -> anyhow::Result<()> {
    let args = Args::from_env().unwrap_or_else(|_| {
        print_usage().unwrap();
        process::exit(2);
    });

    // Create a loader for fonts and files.
    let loader = typst::loading::FsLoader::new()
        .with_path("fonts")
        .with_system()
        .wrap();

    // Create the context which holds loaded source files, fonts, images and
    // cached artifacts.
    let mut ctx = typst::Context::new(loader);

    // Ensure that the source file is not overwritten.
    if is_same_file(&args.input, &args.output).unwrap_or(false) {
        anyhow::bail!("source and destination files are the same");
    }

    // Load the source file.
    let id = ctx.sources.load(&args.input).context("source file not found")?;

    // Typeset.
    match ctx.typeset(id) {
        // Export the PDF.
        Ok(document) => {
            let buffer = typst::export::pdf(&ctx, &document);
            fs::write(&args.output, buffer).context("failed to write PDF file")?;
        }

        // Print diagnostics.
        Err(errors) => {
            print_diagnostics(&ctx.sources, *errors)
                .context("failed to print diagnostics")?;
        }
    }

    Ok(())
}

struct Args {
    input: PathBuf,
    output: PathBuf,
}

impl Args {
    fn from_env() -> Result<Self, anyhow::Error> {
        let mut parser = pico_args::Arguments::from_env();

        // Parse free-standing arguments.
        let input = parser.free_from_str::<PathBuf>()?;
        let output = match parser.opt_free_from_str()? {
            Some(output) => output,
            None => {
                let name = input.file_name().context("source path is not a file")?;
                Path::new(name).with_extension("pdf")
            }
        };

        // Don't allow excess arguments.
        if !parser.finish().is_empty() {
            anyhow::bail!("too many arguments");
        }

        Ok(Self { input, output })
    }
}

/// Print a usage message.
fn print_usage() -> io::Result<()> {
    let mut w = StandardStream::stderr(ColorChoice::Always);
    let styles = Styles::default();

    w.set_color(&styles.header_help)?;
    write!(w, "usage")?;

    w.set_color(&styles.header_message)?;
    writeln!(w, ": typst <input.typ> [output.pdf]")
}

/// Print an error outside of a source file.
fn print_error(error: anyhow::Error) -> io::Result<()> {
    let mut w = StandardStream::stderr(ColorChoice::Always);
    let styles = Styles::default();

    for (i, cause) in error.chain().enumerate() {
        w.set_color(&styles.header_error)?;
        write!(w, "{}", if i == 0 { "error" } else { "cause" })?;

        w.set_color(&styles.header_message)?;
        writeln!(w, ": {}", cause)?;
    }

    w.reset()
}

/// Print diagnostics messages to the terminal.
fn print_diagnostics(
    sources: &SourceStore,
    errors: Vec<Error>,
) -> Result<(), codespan_reporting::files::Error> {
    let mut w = StandardStream::stderr(ColorChoice::Always);
    let config = Config { tab_width: 2, ..Default::default() };

    for error in errors {
        // The main diagnostic.
        let diag = Diagnostic::error().with_message(error.message).with_labels(vec![
            Label::primary(error.span.source, error.span.to_range()),
        ]);

        term::emit(&mut w, &config, sources, &diag)?;

        // Stacktrace-like helper diagnostics.
        for point in error.trace {
            let message = point.v.to_string();
            let help = Diagnostic::help().with_message(message).with_labels(vec![
                Label::primary(point.span.source, point.span.to_range()),
            ]);

            term::emit(&mut w, &config, sources, &help)?;
        }
    }

    Ok(())
}
