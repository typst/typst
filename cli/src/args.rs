use std::fmt::{self, Display, Formatter};
use std::path::PathBuf;

use clap::{ArgAction, Parser, Subcommand, ValueEnum};

/// typst creates PDF files from .typ files
#[derive(Debug, Clone, Parser)]
#[clap(name = "typst", version = crate::typst_version(), author)]
pub struct CliArguments {
    /// The typst command to run
    #[command(subcommand)]
    pub command: Command,

    /// Add additional directories to search for fonts
    #[clap(long = "font-path", env = "TYPST_FONT_PATHS", value_name = "DIR", action = ArgAction::Append)]
    pub font_paths: Vec<PathBuf>,

    /// Configure the root for absolute paths
    #[clap(long = "root", env = "TYPST_ROOT", value_name = "DIR")]
    pub root: Option<PathBuf>,

    /// Sets the level of logging verbosity:
    /// -v = warning & error, -vv = info, -vvv = debug, -vvvv = trace
    #[clap(short, long, action = ArgAction::Count)]
    pub verbosity: u8,
}

/// Which format to use for diagnostics.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, ValueEnum)]
pub enum DiagnosticFormat {
    Human,
    Short,
}

impl Display for DiagnosticFormat {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.to_possible_value()
            .expect("no values are skipped")
            .get_name()
            .fmt(f)
    }
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
    pub fn as_compile(&self) -> Option<&CompileCommand> {
        match self {
            Command::Compile(cmd) => Some(cmd),
            Command::Watch(cmd) => Some(cmd),
            Command::Fonts(_) => None,
        }
    }

    /// Returns whether this is a watch command.
    pub fn is_watch(&self) -> bool {
        matches!(self, Command::Watch(_))
    }
}

/// Compiles the input file into a PDF file
#[derive(Debug, Clone, Parser)]
pub struct CompileCommand {
    /// Path to input Typst file
    pub input: PathBuf,

    /// Path to output PDF file or PNG file(s)
    pub output: Option<PathBuf>,

    /// Opens the output file after compilation using the default PDF viewer
    #[arg(long = "open")]
    pub open: Option<Option<String>>,

    /// The PPI to use if exported as PNG
    #[arg(long = "ppi")]
    pub ppi: Option<f32>,

    /// In which format to emit diagnostics
    #[clap(
        long,
        default_value_t = DiagnosticFormat::Human,
        value_parser = clap::value_parser!(DiagnosticFormat)
    )]
    pub diagnostic_format: DiagnosticFormat,

    /// Produces a flamegraph of the compilation process
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
