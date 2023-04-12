use std::path::PathBuf;

use clap::{ArgAction, Parser, Subcommand};

/// typst creates PDF files from .typ files
#[derive(Debug, Clone, Parser)]
#[clap(name = "typst", version = crate::typst_version(), author)]
pub struct CliArguments {
    /// Add additional directories to search for fonts
    #[clap(long = "font-path", value_name = "DIR", action = ArgAction::Append)]
    pub font_paths: Vec<PathBuf>,

    /// Configure the root for absolute paths
    #[clap(long = "root", value_name = "DIR")]
    pub root: Option<PathBuf>,

    /// The typst command to run
    #[command(subcommand)]
    pub command: Command,

    /// Sets the level of verbosity: 0 = warning & error, 1 = info, 2 = debug, 3 = trace
    #[clap(short, long, action = ArgAction::Count)]
    pub verbosity: u8,

    /// Whether to enable debug mode
    #[clap(long)]
    pub debug: bool,
}

/// What to do.
#[derive(Debug, Clone, Subcommand)]
#[command()]
pub enum Command {
    /// Compiles the input file into a PDF file
    #[command(visible_alias = "c")]
    Compile(CompileCommand),

    /// Watches the input file and recompiles on changes
    #[command(visible_alias = "w")]
    Watch(CompileCommand),

    /// List all discovered fonts in system and custom font paths
    Fonts(FontsCommand),
}

impl Command {
    /// Returns the compile command if this is a compile or watch command.
    // TODO: there is an error where this is not recognized as dead code.
    #[allow(dead_code)]
    pub fn as_compile(&self) -> Option<&CompileCommand> {
        match self {
            Command::Compile(cmd) => Some(cmd),
            Command::Watch(cmd) => Some(cmd),
            Command::Fonts(_) => None,
        }
    }
}

/// Compiles the input file into a PDF file
#[derive(Debug, Clone, Parser)]
pub struct CompileCommand {
    /// Path to input Typst file
    pub input: PathBuf,

    /// Path to output PDF file
    pub output: Option<PathBuf>,

    /// Opens the output file after compilation using the default PDF viewer
    #[arg(long = "open")]
    pub open: Option<Option<String>>,

    /// Produces a flamegraph of the compilation process and saves it to the
    /// given file or to `flamegraph.svg` in the current working directory.
    #[arg(long = "flamegraph", value_name = "OUTPUT_SVG")]
    pub flamegraph: Option<Option<PathBuf>>,
}

/// List all discovered fonts in system and custom font paths
#[derive(Debug, Clone, Parser)]
pub struct FontsCommand {
    /// Also list style variants of each font family
    #[arg(long)]
    pub variants: bool,
}
