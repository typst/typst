use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process;

use codespan_reporting::diagnostic::{Diagnostic, Label};
use codespan_reporting::term::{self, termcolor, Config, Styles};
use same_file::is_same_file;
use termcolor::{ColorChoice, StandardStream, WriteColor};

use typst::diag::Error;
use typst::export;
use typst::loading::FsLoader;
use typst::source::SourceStore;
use typst::Context;

const HELP: &'static str = "\
typst creates PDF files from .typ files

USAGE:
  typst [OPTIONS] <input.typ> [output.pdf]

OPTIONS:
  -h, --help     Print this help
  --root <dir>   Configure the root for absolute paths

ARGS:
  <input.typ>    Path to input Typst file
  [output.pdf]   Path to output PDF
";

fn main() {
    let args = parse_args();
    let ok = args.is_ok();
    if let Err(msg) = args.and_then(try_main) {
        print_error(&msg).unwrap();
        if !ok {
            println!("\nfor more information, try --help");
        }
        process::exit(1);
    }
}

/// The main compiler logic.
fn try_main(args: Args) -> Result<(), String> {
    // Create a loader for fonts and files.
    let mut loader = FsLoader::new();
    let mut builder = Context::builder();
    if let Some(root) = &args.root {
        builder.root(root);
    }

    // Search for fonts in the project directory.
    if let Some(dir) = args.input.parent() {
        if args.root.is_none() {
            builder.root(dir);
        }

        if dir.as_os_str().is_empty() {
            // Just a filename, so directory is current directory.
            loader.search_path(".");
        } else {
            loader.search_path(dir);
        }
    }

    // Search system fonts only now to give local fonts priority.
    loader.search_system();

    // Create the context which holds loaded source files, fonts, images and
    // cached artifacts.
    let mut ctx = builder.build(loader.wrap());

    // Ensure that the source file is not overwritten.
    if is_same_file(&args.input, &args.output).unwrap_or(false) {
        Err("source and destination files are the same")?;
    }

    // Load the source file.
    let id = ctx
        .sources
        .load(&args.input)
        .map_err(|_| "failed to load source file")?;

    // Typeset.
    match ctx.typeset(id) {
        // Export the PDF.
        Ok(frames) => {
            let buffer = export::pdf(&ctx, &frames);
            fs::write(&args.output, buffer).map_err(|_| "failed to write PDF file")?;
        }

        // Print diagnostics.
        Err(errors) => {
            print_diagnostics(&ctx.sources, *errors)
                .map_err(|_| "failed to print diagnostics")?;
        }
    }

    Ok(())
}

struct Args {
    input: PathBuf,
    output: PathBuf,
    root: Option<PathBuf>,
}

/// Parse command line arguments.
fn parse_args() -> Result<Args, String> {
    let mut args = pico_args::Arguments::from_env();
    if args.contains(["-h", "--help"]) {
        print!("{}", HELP);
        std::process::exit(0);
    }

    let root = args.opt_value_from_str("--root").map_err(|_| "malformed root")?;
    let input: PathBuf = args.free_from_str().map_err(|_| "missing input file")?;
    let output = match args.opt_free_from_str().ok().flatten() {
        Some(output) => output,
        None => {
            let name = input.file_name().ok_or("source path does not point to a file")?;
            Path::new(name).with_extension("pdf")
        }
    };

    // Don't allow excess arguments.
    if !args.finish().is_empty() {
        Err("too many arguments")?;
    }

    Ok(Args { input, output, root })
}

/// Print an application-level error (independent from a source file).
fn print_error(msg: &str) -> io::Result<()> {
    let mut w = StandardStream::stderr(ColorChoice::Always);
    let styles = Styles::default();

    w.set_color(&styles.header_error)?;
    write!(w, "error")?;

    w.reset()?;
    writeln!(w, ": {msg}.")
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
