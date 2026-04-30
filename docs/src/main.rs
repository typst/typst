mod args;
mod example;
mod live;
mod reflect;
mod search;
mod world;

use std::io::Write;
use std::path::Path;
use std::process::ExitCode;
use std::sync::{Arc, LazyLock};

use clap::Parser;
use ecow::eco_format;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use typst::diag::{SourceResult, Warned};
use typst::foundations::Bytes;
use typst::syntax::VirtualPath;
use typst_bundle::{Bundle, BundleFile, BundleOptions, VirtualFs};
use typst_kit::diagnostics::{self, termcolor};
use typst_kit::server::HttpServer;
use typst_kit::timer::Timer;
use typst_kit::watcher::Watcher;
use typst_layout::PagedDocument;
use typst_pdf::PdfOptions;

use crate::args::{
    CliArguments, Command, CompileArgs, CompileCommand, OutputFormat, WatchCommand,
};
use crate::world::DocWorld;

/// The parsed command line arguments.
static ARGS: LazyLock<CliArguments> = LazyLock::new(CliArguments::parse);

// Paths.
const ROOT: &str = ".";
const ENTRYPOINT: &str = "docs/main.typ";
const PDF_PATH: &str = "docs/dist/docs.pdf";
const SITE_PATH: &str = "docs/dist/site";

/// Entry point.
fn main() -> ExitCode {
    // Make all paths relative to the workspace.
    let workspace_dir =
        Path::new(env!("CARGO_MANIFEST_DIR")).join(std::path::Component::ParentDir);
    std::env::set_current_dir(workspace_dir).unwrap();

    match &ARGS.command {
        Command::Compile(command) => compile(command),
        Command::Watch(command) => watch(command),
    }
}

/// Execute a compilation command.
fn compile(command: &CompileCommand) -> ExitCode {
    let mut timer = Timer::new_or_placeholder(command.args.timings.clone());
    let mut config = Config::new(&command.args, false);
    let mut world = DocWorld::new(&config, ROOT, ENTRYPOINT);
    let report = timer
        .record(&mut world, |world| compile_once(world, &mut config))
        .unwrap();
    report.print(&world);

    if report.0.output.is_err()
        || (command.deny_warnings && !report.0.warnings.is_empty())
    {
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}

/// Execute a watching compilation command.
fn watch(command: &WatchCommand) -> ! {
    let mut timer = Timer::new_or_placeholder(command.args.timings.clone());
    let mut watcher = Watcher::new(None).unwrap();
    let mut config = Config::new(&command.args, true);
    let mut world = DocWorld::new(&config, ROOT, ENTRYPOINT);

    loop {
        print_watch_header(&config);
        writeln!(out(), "compiling ...").unwrap();
        let report = timer
            .record(&mut world, |world| compile_once(world, &mut config))
            .unwrap();

        print_watch_header(&config);
        report.print(&world);

        comemo::evict(10);
        watcher.update(world.dependencies()).unwrap();
        watcher.wait().unwrap();
        world.reset();
    }
}

/// Prints the status line in watch mode.
fn print_watch_header(config: &Config) {
    let mut out = out();
    let clear = "\x1B[2J\x1B[1;1H";
    write!(out, "{clear}").unwrap();
    if let Some(server) = &config.server {
        writeln!(out, "serving docs on http://{}", server.addr()).unwrap();
        writeln!(out).unwrap();
    }
    if let Some(path) = &config.output {
        writeln!(out, "writing to {}", path.display()).unwrap();
        writeln!(out).unwrap();
    }
}

/// Preprocessing configuration for compilation.
struct Config {
    /// The output path to which the compilation output is written.
    output: Option<&'static Path>,
    /// The kind of output to produce.
    output_format: OutputFormat,
    /// A live reload server for the `watch` subcommand.
    server: Option<HttpServer>,
    /// Whether to open the output after compilation.
    open: bool,
    /// The base path for the documentation.
    base: String,
}

impl Config {
    /// Preprocess `CompileArgs`, producing a compilation config.
    fn new(args: &CompileArgs, watching: bool) -> Self {
        Self {
            output: match args.format {
                OutputFormat::Pdf => Some(PDF_PATH),
                OutputFormat::Website if watching => None,
                OutputFormat::Website => Some(SITE_PATH),
            }
            .map(Path::new),
            output_format: args.format,
            server: (watching && args.format == OutputFormat::Website)
                .then(|| HttpServer::new("docs", None, true).unwrap()),
            open: args.open,
            base: {
                let mut base = args.base.clone();
                if !base.starts_with('/') {
                    base.insert(0, '/');
                }
                if !base.ends_with('/') {
                    base.push('/');
                }
                base
            },
        }
    }
}

/// Compiles the documentation, potentially writing it to disk and/or updating
/// the live reload server, and returns compilation diagnostics ready for
/// printing.
fn compile_once(world: &DocWorld, config: &mut Config) -> Report {
    let mut warned = match config.output_format {
        OutputFormat::Website => {
            let Warned { output, warnings } = typst::compile::<Bundle>(world);
            let result = output.and_then(|bundle| export_website(bundle, config));
            Warned { output: result, warnings }
        }
        OutputFormat::Pdf => {
            let Warned { output, warnings } = typst::compile::<PagedDocument>(world);
            let result = output.and_then(|document| export_pdf(&document, config));
            Warned { output: result, warnings }
        }
    };

    if config.open && warned.output.is_ok() {
        if let Some(server) = &config.server {
            let url = format!("http://{}", server.addr());
            open::that_detached(url).unwrap();
        } else if let Some(output) = config.output {
            open::that_detached(output).unwrap();
        }
        config.open = false;
    }

    warned
        .warnings
        .retain(|diag| !diag.message.starts_with("bundle export is experimental"));

    Report(warned)
}

/// Exports the built website, adding a search index, and refreshes the live
/// reload server.
fn export_website(mut bundle: Bundle, config: &Config) -> SourceResult<()> {
    let index = crate::search::build_search_index(&bundle)?;
    Arc::make_mut(&mut bundle.files).insert(
        VirtualPath::new(eco_format!("{}assets/search.json", config.base)).unwrap(),
        BundleFile::Asset(Bytes::new(serde_json::to_vec(&index).unwrap())),
    );

    let options = BundleOptions { pixel_per_pt: 1.0, pdf: PdfOptions::default() };
    let fs = typst_bundle::export(&bundle, &options)?;

    if let Some(path) = &config.output {
        write_virtual_fs(path, &fs);
    }

    if let Some(server) = &config.server {
        server.set_bundle(bundle, fs);
    }

    Ok(())
}

/// Writes a bundle's files to disk.
fn write_virtual_fs(root: &Path, fs: &VirtualFs) {
    std::fs::create_dir_all(root).unwrap();
    fs.par_iter().for_each(|(path, data)| {
        let realized = path.realize(root);
        if let Some(parent) = realized.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&realized, data).unwrap();
    })
}

/// Exports a document to PDF and writes it to disk.
fn export_pdf(document: &PagedDocument, config: &Config) -> SourceResult<()> {
    let data = typst_pdf::pdf(document, &typst_pdf::PdfOptions::default())?;
    if let Some(path) = &config.output {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, data).unwrap();
    }
    Ok(())
}

/// Acquires the output stream for user-facing messages.
fn out() -> termcolor::StandardStream {
    termcolor::StandardStream::stderr(termcolor::ColorChoice::Auto)
}

/// Holds diagnostics, ready for printing.
struct Report(Warned<SourceResult<()>>);

impl Report {
    /// Prints the diagnostics.
    fn print(&self, world: &DocWorld) {
        let Warned { output, warnings } = &self.0;
        let errors = output.as_ref().err().map(|v| v.as_slice()).unwrap_or_default();
        diagnostics::emit(
            &mut out(),
            world,
            errors.iter().chain(warnings),
            diagnostics::DiagnosticFormat::Human,
        )
        .unwrap();
    }
}
