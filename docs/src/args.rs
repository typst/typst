use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};
use typst_utils::display_possible_values;

/// Generator for Typst's documentation.
#[derive(Debug, Clone, Parser)]
#[command(bin_name = "cargo docit")]
#[clap(name = "typst-docs", author)]
pub struct CliArguments {
    /// The command to run.
    #[command(subcommand)]
    pub command: Command,
}

/// What to do.
#[derive(Debug, Clone, Subcommand)]
#[command()]
pub enum Command {
    /// Compiles the documentation into a directory on-disk.
    ///
    /// - When exporting to a website, the output is written to `docs/dist/site`
    /// - When exporting to a PDF, the output is written to `docs/dist/docs.pdf`
    #[command(visible_alias = "c")]
    Compile(CompileCommand),
    /// Compiles the documentation continuously.
    ///
    /// - When exporting to a website, serves on a live-reload server instead of
    ///   writing to disk.
    /// - When exporting to a PDF, writes to disk.
    #[command(visible_alias = "w")]
    Watch(WatchCommand),
}

/// Compiles the documentation into a directory on-disk.
#[derive(Debug, Clone, Parser)]
pub struct CompileCommand {
    /// Arguments for compilation.
    #[clap(flatten)]
    pub args: CompileArgs,
    /// Whether to exit with an error code if there were warnings.
    #[arg(long)]
    pub deny_warnings: bool,
}

/// Compiles the documentation continuously.
#[derive(Debug, Clone, Parser)]
pub struct WatchCommand {
    /// Arguments for compilation.
    #[clap(flatten)]
    pub args: CompileArgs,
}

/// Arguments for compilation and watching.
#[derive(Debug, Clone, Parser)]
pub struct CompileArgs {
    /// The output format.
    #[arg(long = "format", short = 'f', default_value_t)]
    pub format: OutputFormat,
    /// The base path for the documentation. Only applies to the website output
    /// format.
    ///
    /// Should be an absolute path like `/docs/` and should start and end with a
    /// forward slash (the slashes will be added if missing).
    ///
    /// The default is `/`.
    #[arg(long, default_value = "/")]
    pub base: String,
    /// Produces performance timings of the compilation process.
    #[arg(long = "timings", value_name = "OUTPUT_JSON")]
    pub timings: Option<PathBuf>,
    /// Open the generated output when finished.
    #[arg(long)]
    pub open: bool,
}

/// Which kind of output to generate.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, ValueEnum)]
pub enum OutputFormat {
    #[default]
    Website,
    Pdf,
}

display_possible_values!(OutputFormat);
