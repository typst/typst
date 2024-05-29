use std::fmt::{self, Display, Formatter};
use std::num::NonZeroUsize;
use std::ops::RangeInclusive;
use std::path::PathBuf;
use std::str::FromStr;

use chrono::{DateTime, Utc};
use clap::builder::ValueParser;
use clap::{ArgAction, Args, ColorChoice, Parser, Subcommand, ValueEnum};
use semver::Version;

/// The character typically used to separate path components
/// in environment variables.
const ENV_PATH_SEP: char = if cfg!(windows) { ';' } else { ':' };

/// The Typst compiler.
#[derive(Debug, Clone, Parser)]
#[clap(name = "typst", version = crate::typst_version(), author)]
pub struct CliArguments {
    /// The command to run
    #[command(subcommand)]
    pub command: Command,

    /// Set when to use color.
    /// auto = use color if a capable terminal is detected
    #[clap(
        long,
        value_name = "WHEN",
        require_equals = true,
        num_args = 0..=1,
        default_value = "auto",
        default_missing_value = "always",
    )]
    pub color: ColorChoice,

    /// Path to a custom CA certificate to use when making network requests.
    #[clap(long = "cert", env = "TYPST_CERT")]
    pub cert: Option<PathBuf>,
}

/// What to do.
#[derive(Debug, Clone, Subcommand)]
#[command()]
pub enum Command {
    /// Compiles an input file into a supported output format
    #[command(visible_alias = "c")]
    Compile(CompileCommand),

    /// Watches an input file and recompiles on changes
    #[command(visible_alias = "w")]
    Watch(CompileCommand),

    /// Initializes a new project from a template
    Init(InitCommand),

    /// Processes an input file to extract provided metadata
    Query(QueryCommand),

    /// Lists all discovered fonts in system and custom font paths
    Fonts(FontsCommand),

    /// Self update the Typst CLI
    #[cfg_attr(not(feature = "self-update"), doc = " (disabled)")]
    Update(UpdateCommand),
}

/// Compiles an input file into a supported output format
#[derive(Debug, Clone, Parser)]
pub struct CompileCommand {
    /// Shared arguments
    #[clap(flatten)]
    pub common: SharedArgs,

    /// Path to output file (PDF, PNG, or SVG).
    /// Use `-` to write output to stdout; For output formats emitting one file per page,
    /// a page number template must be present if the source document renders to multiple pages.
    /// Use `{p}` for page numbers, `{0p}` for zero padded page numbers, `{t}` for page count.
    /// For example, `doc-page-{0p}-of-{t}.png` creates `doc-page-01-of-10.png` and so on.
    #[clap(required_if_eq("input", "-"), value_parser = ValueParser::new(output_value_parser))]
    pub output: Option<Output>,

    /// Which pages to export. When unspecified, all document pages are exported.
    ///
    /// Pages to export are separated by commas, and can be either simple page
    /// numbers (e.g. '2,5' to export only pages 2 and 5) or page ranges
    /// (e.g. '2,3-6,8-' to export page 2, pages 3 to 6 (inclusive), page 8 and
    /// any pages after it).
    ///
    /// Page numbers are one-indexed and correspond to real page numbers in the
    /// document (therefore not being affected by the document's page counter).
    #[arg(long = "pages", value_delimiter = ',')]
    pub pages: Option<Vec<PageRangeArgument>>,

    /// Output a Makefile rule describing the current compilation
    #[clap(long = "make-deps", value_name = "PATH")]
    pub make_deps: Option<PathBuf>,

    /// The format of the output file, inferred from the extension by default
    #[arg(long = "format", short = 'f')]
    pub format: Option<OutputFormat>,

    /// Opens the output file using the default viewer after compilation.
    /// Ignored if output is stdout
    #[arg(long = "open")]
    pub open: Option<Option<String>>,

    /// The PPI (pixels per inch) to use for PNG export
    #[arg(long = "ppi", default_value_t = 144.0)]
    pub ppi: f32,

    /// Produces performance timings of the compilation process (experimental)
    ///
    /// The resulting JSON file can be loaded into a tracing tool such as
    /// https://ui.perfetto.dev. It does not contain any sensitive information
    /// apart from file names and line numbers.
    #[arg(long = "timings", value_name = "OUTPUT_JSON")]
    pub timings: Option<Option<PathBuf>>,
}

/// Initializes a new project from a template
#[derive(Debug, Clone, Parser)]
pub struct InitCommand {
    /// The template to use, e.g. `@preview/charged-ieee`
    ///
    /// You can specify the version by appending e.g. `:0.1.0`. If no version is
    /// specified, Typst will default to the latest version.
    ///
    /// Supports both local and published templates.
    pub template: String,

    /// The project directory, defaults to the template's name
    pub dir: Option<String>,
}

/// Processes an input file to extract provided metadata
#[derive(Debug, Clone, Parser)]
pub struct QueryCommand {
    /// Shared arguments
    #[clap(flatten)]
    pub common: SharedArgs,

    /// Defines which elements to retrieve
    pub selector: String,

    /// Extracts just one field from all retrieved elements
    #[clap(long = "field")]
    pub field: Option<String>,

    /// Expects and retrieves exactly one element
    #[clap(long = "one", default_value = "false")]
    pub one: bool,

    /// The format to serialize in
    #[clap(long = "format", default_value = "json")]
    pub format: SerializationFormat,

    /// Whether to pretty-print the serialized output
    #[clap(long)]
    pub pretty: bool,
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
    /// Path to input Typst file, use `-` to read input from stdin
    #[clap(value_parser = input_value_parser)]
    pub input: Input,

    /// Configures the project root (for absolute paths)
    #[clap(long = "root", env = "TYPST_ROOT", value_name = "DIR")]
    pub root: Option<PathBuf>,

    /// Add a string key-value pair visible through `sys.inputs`
    #[clap(
        long = "input",
        value_name = "key=value",
        action = ArgAction::Append,
        value_parser = ValueParser::new(parse_input_pair),
    )]
    pub inputs: Vec<(String, String)>,

    /// Adds additional directories to search for fonts
    #[clap(
        long = "font-path",
        env = "TYPST_FONT_PATHS",
        value_name = "DIR",
        value_delimiter = ENV_PATH_SEP,
    )]
    pub font_paths: Vec<PathBuf>,

    /// The document's creation date formatted as a UNIX timestamp.
    ///
    /// For more information, see <https://reproducible-builds.org/specs/source-date-epoch/>.
    #[clap(
        long = "creation-timestamp",
        env = "SOURCE_DATE_EPOCH",
        value_name = "UNIX_TIMESTAMP",
        value_parser = parse_source_date_epoch,
    )]
    pub creation_timestamp: Option<DateTime<Utc>>,

    /// The format to emit diagnostics in
    #[clap(
        long,
        default_value_t = DiagnosticFormat::Human,
        value_parser = clap::value_parser!(DiagnosticFormat)
    )]
    pub diagnostic_format: DiagnosticFormat,
}

/// Parses a UNIX timestamp according to <https://reproducible-builds.org/specs/source-date-epoch/>
fn parse_source_date_epoch(raw: &str) -> Result<DateTime<Utc>, String> {
    let timestamp: i64 = raw
        .parse()
        .map_err(|err| format!("timestamp must be decimal integer ({err})"))?;
    DateTime::from_timestamp(timestamp, 0)
        .ok_or_else(|| "timestamp out of range".to_string())
}

/// An input that is either stdin or a real path.
#[derive(Debug, Clone)]
pub enum Input {
    /// Stdin, represented by `-`.
    Stdin,
    /// A non-empty path.
    Path(PathBuf),
}

/// An output that is either stdout or a real path.
#[derive(Debug, Clone)]
pub enum Output {
    /// Stdout, represented by `-`.
    Stdout,
    /// A non-empty path.
    Path(PathBuf),
}

impl Display for Output {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Output::Stdout => f.pad("stdout"),
            Output::Path(path) => path.display().fmt(f),
        }
    }
}

/// The clap value parser used by `SharedArgs.input`
fn input_value_parser(value: &str) -> Result<Input, clap::error::Error> {
    if value.is_empty() {
        Err(clap::Error::new(clap::error::ErrorKind::InvalidValue))
    } else if value == "-" {
        Ok(Input::Stdin)
    } else {
        Ok(Input::Path(value.into()))
    }
}

/// The clap value parser used by `CompileCommand.output`
fn output_value_parser(value: &str) -> Result<Output, clap::error::Error> {
    // Empty value also handled by clap for `Option<Output>`
    if value.is_empty() {
        Err(clap::Error::new(clap::error::ErrorKind::InvalidValue))
    } else if value == "-" {
        Ok(Output::Stdout)
    } else {
        Ok(Output::Path(value.into()))
    }
}

/// Parses key/value pairs split by the first equal sign.
///
/// This function will return an error if the argument contains no equals sign
/// or contains the key (before the equals sign) is empty.
fn parse_input_pair(raw: &str) -> Result<(String, String), String> {
    let (key, val) = raw
        .split_once('=')
        .ok_or("input must be a key and a value separated by an equal sign")?;
    let key = key.trim().to_owned();
    if key.is_empty() {
        return Err("the key was missing or empty".to_owned());
    }
    let val = val.trim().to_owned();
    Ok((key, val))
}

/// Implements parsing of page ranges (`1-3`, `4`, `5-`, `-2`), used by the
/// `CompileCommand.pages` argument, through the `FromStr` trait instead of
/// a value parser, in order to generate better errors.
///
/// See also: https://github.com/clap-rs/clap/issues/5065
#[derive(Debug, Clone)]
pub struct PageRangeArgument(RangeInclusive<Option<NonZeroUsize>>);

impl PageRangeArgument {
    pub fn to_range(&self) -> RangeInclusive<Option<NonZeroUsize>> {
        self.0.clone()
    }
}

impl FromStr for PageRangeArgument {
    type Err = &'static str;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.split('-').map(str::trim).collect::<Vec<_>>().as_slice() {
            [] | [""] => Err("page export range must not be empty"),
            [single_page] => {
                let page_number = parse_page_number(single_page)?;
                Ok(PageRangeArgument(Some(page_number)..=Some(page_number)))
            }
            ["", ""] => Err("page export range must have start or end"),
            [start, ""] => Ok(PageRangeArgument(Some(parse_page_number(start)?)..=None)),
            ["", end] => Ok(PageRangeArgument(None..=Some(parse_page_number(end)?))),
            [start, end] => {
                let start = parse_page_number(start)?;
                let end = parse_page_number(end)?;
                if start > end {
                    Err("page export range must end at a page after the start")
                } else {
                    Ok(PageRangeArgument(Some(start)..=Some(end)))
                }
            }
            [_, _, _, ..] => Err("page export range must have a single hyphen"),
        }
    }
}

fn parse_page_number(value: &str) -> Result<NonZeroUsize, &'static str> {
    if value == "0" {
        Err("page numbers start at one")
    } else {
        NonZeroUsize::from_str(value).map_err(|_| "not a valid page number")
    }
}

/// Lists all discovered fonts in system and custom font paths
#[derive(Debug, Clone, Parser)]
pub struct FontsCommand {
    /// Adds additional directories to search for fonts
    #[clap(
        long = "font-path",
        env = "TYPST_FONT_PATHS",
        value_name = "DIR",
        value_delimiter = ENV_PATH_SEP,
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

/// Update the CLI using a pre-compiled binary from a Typst GitHub release.
#[derive(Debug, Clone, Parser)]
pub struct UpdateCommand {
    /// Which version to update to (defaults to latest)
    pub version: Option<Version>,

    /// Forces a downgrade to an older version (required for downgrading)
    #[clap(long, default_value_t = false)]
    pub force: bool,

    /// Reverts to the version from before the last update (only possible if
    /// `typst update` has previously ran)
    #[clap(long, default_value_t = false, exclusive = true)]
    pub revert: bool,
}

/// Which format to use for the generated output file.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, ValueEnum)]
pub enum OutputFormat {
    Pdf,
    Png,
    Svg,
}

impl Display for OutputFormat {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.to_possible_value()
            .expect("no values are skipped")
            .get_name()
            .fmt(f)
    }
}
