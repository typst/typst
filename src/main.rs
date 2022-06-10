use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process;
use std::sync::Arc;

use codespan_reporting::diagnostic::{Diagnostic, Label};
use codespan_reporting::term::{self, termcolor};
use pico_args::Arguments;
use same_file::is_same_file;
use termcolor::{ColorChoice, StandardStream, WriteColor};

use typst::diag::{Error, StrResult};
use typst::font::{FaceInfo, FontStore};
use typst::library::text::THEME;
use typst::loading::FsLoader;
use typst::parse::TokenMode;
use typst::source::SourceStore;
use typst::{Config, Context};

/// What to do.
enum Command {
    Typeset(TypesetCommand),
    Highlight(HighlightCommand),
    Fonts(FontsCommand),
}

/// Typeset a .typ file into a PDF file.
struct TypesetCommand {
    input: PathBuf,
    output: PathBuf,
    root: Option<PathBuf>,
}

const HELP: &'static str = "\
typst creates PDF files from .typ files

USAGE:
  typst [OPTIONS] <input.typ> [output.pdf]
  typst [SUBCOMMAND] ...

ARGS:
  <input.typ>    Path to input Typst file
  [output.pdf]   Path to output PDF file

OPTIONS:
  -h, --help     Print this help
  --root <dir>   Configure the root for absolute paths

SUBCOMMANDS:
  --highlight    Highlight .typ files to HTML
  --fonts        List all discovered system fonts
";

/// Highlight a .typ file into a HTML file.
struct HighlightCommand {
    input: PathBuf,
    output: PathBuf,
}

const HELP_HIGHLIGHT: &'static str = "\
typst --highlight creates highlighted HTML from .typ files

USAGE:
  typst --highlight [OPTIONS] <input.typ> [output.html]

ARGS:
  <input.typ>    Path to input Typst file
  [output.html]  Path to output HTML file

OPTIONS:
  -h, --help     Print this help
";

/// List discovered fonts.
struct FontsCommand {
    variants: bool,
}

const HELP_FONTS: &'static str = "\
typst --fonts lists all discovered system fonts

USAGE:
  typst --fonts [OPTIONS]

OPTIONS:
  -h, --help     Print this help
  --variants     Also list style variants of each font family
";

/// Entry point.
fn main() {
    let command = parse_args();
    let ok = command.is_ok();
    if let Err(msg) = command.and_then(dispatch) {
        print_error(&msg).unwrap();
        if !ok {
            println!("\nfor more information, try --help");
        }
        process::exit(1);
    }
}

/// Parse command line arguments.
fn parse_args() -> StrResult<Command> {
    let mut args = Arguments::from_env();
    let help = args.contains(["-h", "--help"]);

    let command = if args.contains("--highlight") {
        if help {
            print_help(HELP_HIGHLIGHT);
        }

        let (input, output) = parse_input_output(&mut args, "html")?;
        Command::Highlight(HighlightCommand { input, output })
    } else if args.contains("--fonts") {
        if help {
            print_help(HELP_FONTS);
        }

        Command::Fonts(FontsCommand { variants: args.contains("--variants") })
    } else {
        if help {
            print_help(HELP);
        }

        let root = args.opt_value_from_str("--root").map_err(|_| "missing root path")?;
        let (input, output) = parse_input_output(&mut args, "pdf")?;
        Command::Typeset(TypesetCommand { input, output, root })
    };

    // Don't allow excess arguments.
    let rest = args.finish();
    if !rest.is_empty() {
        Err(format!(
            "unexpected argument{}",
            if rest.len() > 1 { "s" } else { "" }
        ))?;
    }

    Ok(command)
}

/// Parse two freestanding path arguments, with the output path being optional.
/// If it is omitted, it is determined from the input path's filename with the
/// given extension.
fn parse_input_output(args: &mut Arguments, ext: &str) -> StrResult<(PathBuf, PathBuf)> {
    let input: PathBuf = args.free_from_str().map_err(|_| "missing input file")?;
    let output = match args.opt_free_from_str().ok().flatten() {
        Some(output) => output,
        None => {
            let name = input.file_name().ok_or("source path does not point to a file")?;
            Path::new(name).with_extension(ext)
        }
    };

    // Ensure that the source file is not overwritten.
    if is_same_file(&input, &output).unwrap_or(false) {
        Err("source and destination files are the same")?;
    }

    Ok((input, output))
}

/// Print a help string and quit.
fn print_help(help: &'static str) {
    print!("{help}");
    std::process::exit(0);
}

/// Print an application-level error (independent from a source file).
fn print_error(msg: &str) -> io::Result<()> {
    let mut w = StandardStream::stderr(ColorChoice::Always);
    let styles = term::Styles::default();

    w.set_color(&styles.header_error)?;
    write!(w, "error")?;

    w.reset()?;
    writeln!(w, ": {msg}.")
}

/// Dispatch a command.
fn dispatch(command: Command) -> StrResult<()> {
    match command {
        Command::Typeset(command) => typeset(command),
        Command::Highlight(command) => highlight(command),
        Command::Fonts(command) => fonts(command),
    }
}

/// Execute a typesetting command.
fn typeset(command: TypesetCommand) -> StrResult<()> {
    let mut config = Config::builder();
    if let Some(root) = &command.root {
        config.root(root);
    } else if let Some(dir) = command.input.parent() {
        config.root(dir);
    }

    // Create a loader for fonts and files.
    let loader = FsLoader::new().with_system();

    // Create the context which holds loaded source files, fonts, images and
    // cached artifacts.
    let mut ctx = Context::new(Arc::new(loader), config.build());

    // Load the source file.
    let id = ctx
        .sources
        .load(&command.input)
        .map_err(|_| "failed to load source file")?;

    // Typeset.
    match typst::typeset(&mut ctx, id) {
        // Export the PDF.
        Ok(frames) => {
            let buffer = typst::export::pdf(&ctx, &frames);
            fs::write(&command.output, buffer).map_err(|_| "failed to write PDF file")?;
        }

        // Print diagnostics.
        Err(errors) => {
            print_diagnostics(&ctx.sources, *errors)
                .map_err(|_| "failed to print diagnostics")?;
        }
    }

    Ok(())
}

/// Print diagnostics messages to the terminal.
fn print_diagnostics(
    sources: &SourceStore,
    errors: Vec<Error>,
) -> Result<(), codespan_reporting::files::Error> {
    let mut w = StandardStream::stderr(ColorChoice::Always);
    let config = term::Config { tab_width: 2, ..Default::default() };

    for error in errors {
        // The main diagnostic.
        let range = sources.range(error.span);
        let diag = Diagnostic::error()
            .with_message(error.message)
            .with_labels(vec![Label::primary(error.span.source(), range)]);

        term::emit(&mut w, &config, sources, &diag)?;

        // Stacktrace-like helper diagnostics.
        for point in error.trace {
            let message = point.v.to_string();
            let help = Diagnostic::help().with_message(message).with_labels(vec![
                Label::primary(point.span.source(), sources.range(point.span)),
            ]);

            term::emit(&mut w, &config, sources, &help)?;
        }
    }

    Ok(())
}

/// Execute a highlighting command.
fn highlight(command: HighlightCommand) -> StrResult<()> {
    let input = std::fs::read_to_string(&command.input)
        .map_err(|_| "failed to load source file")?;

    let html = typst::syntax::highlight_html(&input, TokenMode::Markup, &THEME);
    fs::write(&command.output, html).map_err(|_| "failed to write HTML file")?;

    Ok(())
}

/// Execute a font listing command.
fn fonts(command: FontsCommand) -> StrResult<()> {
    let loader = FsLoader::new().with_system();
    let fonts = FontStore::new(Arc::new(loader));

    for (name, infos) in fonts.families() {
        println!("{name}");
        if command.variants {
            for &FaceInfo { variant, .. } in infos {
                println!(
                    "- Style: {:?}, Weight: {:?}, Stretch: {:?}",
                    variant.style, variant.weight, variant.stretch,
                );
            }
        }
    }

    Ok(())
}
