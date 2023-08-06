use std::fmt::{self, Display, Formatter};
use std::path::PathBuf;

use clap::{ArgAction, Args, Parser, Subcommand, ValueEnum};

/// The Typst compiler.
#[derive(Debug, Clone, Parser)]
#[clap(name = "typst", version = crate::typst_version(), author)]
pub struct CliArguments {
    /// The command to run
    #[command(subcommand)]
    pub command: Command,

    /// Sets the level of logging verbosity:
    /// -v = warning & error, -vv = info, -vvv = debug, -vvvv = trace
    #[clap(short, long, action = ArgAction::Count)]
    pub verbosity: u8,
}

/// What to do.
#[derive(Debug, Clone, Subcommand)]
#[command()]
pub enum Command {
    /// Compiles an input file into a PDF or PNG file
    #[command(visible_alias = "c")]
    Compile(CompileCommand),

    /// Watches an input file and recompiles on changes
    #[command(visible_alias = "w")]
    Watch(CompileCommand),

    /// Processes an input file to extract provided metadata
    Query(QueryCommand),

    /// Lists all discovered fonts in system and custom font paths
    Fonts(FontsCommand),
}

/// Compiles the input file into a PDF file
#[derive(Debug, Clone, Parser)]
pub struct CompileCommand {
    /// Shared arguments.
    #[clap(flatten)]
    pub common: SharedArgs,

    /// Path to output PDF file or PNG file(s)
    pub output: Option<PathBuf>,

    /// Opens the output file using the default viewer after compilation
    #[arg(long = "open")]
    pub open: Option<Option<String>>,

    /// The PPI (pixels per inch) to use for PNG export
    #[arg(long = "ppi", default_value_t = 144.0)]
    pub ppi: f32,

    /// Produces a flamegraph of the compilation process
    #[arg(long = "flamegraph", value_name = "OUTPUT_SVG")]
    pub flamegraph: Option<Option<PathBuf>>,
}

impl CompileCommand {
    /// The output path.
    pub fn output(&self) -> PathBuf {
        self.output
            .clone()
            .unwrap_or_else(|| self.common.input.with_extension("pdf"))
    }
}

/// Processes an input file to extract provided metadata
#[derive(Debug, Clone, Parser)]
pub struct QueryCommand {
    /// Shared arguments.
    #[clap(flatten)]
    pub common: SharedArgs,

    /// Define what elements to retrieve
    pub selector: String,

    /// Extract just one field from all retrieved elements
    #[clap(long = "field")]
    pub field: Option<String>,

    /// Expect and retrieve exactly one element
    #[clap(long = "one", default_value = "false")]
    pub one: bool,

    /// The format to serialization in
    #[clap(long = "format", default_value = "json")]
    pub format: SerializationFormat,
}

// Output file format for query command
#[derive(Debug, Copy, Clone, Eq, PartialEq, ValueEnum)]
pub enum SerializationFormat {
    Json,
    Yaml,
}

/// Common arguments of compile, watch, and query.
#[derive(Debug, Clone, Args)]
pub struct SharedArgs {
    /// Path to input Typst file
    pub input: PathBuf,

    /// Configures the project root
    #[clap(long = "root", env = "TYPST_ROOT", value_name = "DIR")]
    pub root: Option<PathBuf>,

    /// Adds additional directories to search for fonts
    #[clap(
        long = "font-path",
        env = "TYPST_FONT_PATHS",
        value_name = "DIR",
        action = ArgAction::Append,
    )]
    pub font_paths: Vec<PathBuf>,

    /// In which format to emit diagnostics
    #[clap(
        long,
        default_value_t = DiagnosticFormat::Human,
        value_parser = clap::value_parser!(DiagnosticFormat)
    )]
    pub diagnostic_format: DiagnosticFormat,
}

/// Lists all discovered fonts in system and custom font paths
#[derive(Debug, Clone, Parser)]
pub struct FontsCommand {
    /// Adds additional directories to search for fonts
    #[clap(
        long = "font-path",
        env = "TYPST_FONT_PATHS",
        value_name = "DIR",
        action = ArgAction::Append,
    )]
    pub font_paths: Vec<PathBuf>,

    /// Also lists style variants of each font family
    #[arg(long)]
    pub variants: bool,
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
