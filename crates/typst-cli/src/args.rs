use std::fmt::{self, Display, Formatter};
use std::num::NonZeroUsize;
use std::ops::RangeInclusive;
use std::path::PathBuf;
use std::str::FromStr;

use chrono::{DateTime, Utc};
use clap::builder::{TypedValueParser, ValueParser};
use clap::{ArgAction, Args, ColorChoice, Parser, Subcommand, ValueEnum, ValueHint};
use semver::Version;

/// The character typically used to separate path components
/// in environment variables.
const ENV_PATH_SEP: char = if cfg!(windows) { ';' } else { ':' };

/// The overall structure of the help.
#[rustfmt::skip]
const HELP_TEMPLATE: &str = "\
Typst {version}

{usage-heading} {usage}

{all-args}{after-help}\
";

/// Adds a list of useful links after the normal help.
#[rustfmt::skip]
const AFTER_HELP: &str = color_print::cstr!("\
<s><u>Resources:</></>
  <s>Tutorial:</>                 https://typst.app/docs/tutorial/
  <s>Reference documentation:</>  https://typst.app/docs/reference/
  <s>Templates & Packages:</>     https://typst.app/universe/
  <s>Forum for questions:</>      https://forum.typst.app/
");

/// The Typst compiler.
#[derive(Debug, Clone, Parser)]
#[clap(
    name = "typst",
    version = crate::typst_version(),
    author,
    help_template = HELP_TEMPLATE,
    after_help = AFTER_HELP,
    max_term_width = 80,
)]
pub struct CliArguments {
    /// The command to run.
    #[command(subcommand)]
    pub command: Command,

    /// Whether to use color. When set to `auto` if the terminal to supports it.
    #[clap(long, default_value_t = ColorChoice::Auto, default_missing_value = "always")]
    pub color: ColorChoice,

    /// Path to a custom CA certificate to use when making network requests.
    #[clap(long, env = "TYPST_CERT")]
    pub cert: Option<PathBuf>,
}

/// What to do.
#[derive(Debug, Clone, Subcommand)]
#[command()]
pub enum Command {
    /// Compiles an input file into a supported output format.
    #[command(visible_alias = "c")]
    Compile(CompileCommand),

    /// Watches an input file and recompiles on changes.
    #[command(visible_alias = "w")]
    Watch(WatchCommand),

    /// Initializes a new project from a template.
    Init(InitCommand),

    /// Processes an input file to extract provided metadata.
    Query(QueryCommand),

    /// Lists all discovered fonts in system and custom font paths.
    Fonts(FontsCommand),

    /// Self update the Typst CLI.
    #[cfg_attr(not(feature = "self-update"), clap(hide = true))]
    Update(UpdateCommand),
}

/// Compiles an input file into a supported output format.
#[derive(Debug, Clone, Parser)]
pub struct CompileCommand {
    /// Arguments for compilation.
    #[clap(flatten)]
    pub args: CompileArgs,
}

/// Compiles an input file into a supported output format.
#[derive(Debug, Clone, Parser)]
pub struct WatchCommand {
    /// Arguments for compilation.
    #[clap(flatten)]
    pub args: CompileArgs,

    /// Disables the built-in HTTP server for HTML export.
    #[clap(long)]
    pub no_serve: bool,

    /// Disables the injected live reload script for HTML export. The HTML that
    /// is written to disk isn't affected either way.
    #[clap(long)]
    pub no_reload: bool,

    /// The port where HTML is served.
    ///
    /// Defaults to the first free port in the range 3000-3005.
    #[clap(long)]
    pub port: Option<u16>,
}

/// Initializes a new project from a template.
#[derive(Debug, Clone, Parser)]
pub struct InitCommand {
    /// The template to use, e.g. `@preview/charged-ieee`.
    ///
    /// You can specify the version by appending e.g. `:0.1.0`. If no version is
    /// specified, Typst will default to the latest version.
    ///
    /// Supports both local and published templates.
    pub template: String,

    /// The project directory, defaults to the template's name.
    pub dir: Option<String>,

    /// Arguments related to storage of packages in the system.
    #[clap(flatten)]
    pub package: PackageArgs,
}

/// Processes an input file to extract provided metadata.
#[derive(Debug, Clone, Parser)]
pub struct QueryCommand {
    /// Path to input Typst file. Use `-` to read input from stdin.
    #[clap(value_parser = input_value_parser(), value_hint = ValueHint::FilePath)]
    pub input: Input,

    /// Defines which elements to retrieve.
    pub selector: String,

    /// Extracts just one field from all retrieved elements.
    #[clap(long = "field")]
    pub field: Option<String>,

    /// Expects and retrieves exactly one element.
    #[clap(long = "one", default_value = "false")]
    pub one: bool,

    /// The format to serialize in.
    #[clap(long = "format", default_value_t)]
    pub format: SerializationFormat,

    /// Whether to pretty-print the serialized output.
    ///
    /// Only applies to JSON format.
    #[clap(long)]
    pub pretty: bool,

    /// World arguments.
    #[clap(flatten)]
    pub world: WorldArgs,

    /// Processing arguments.
    #[clap(flatten)]
    pub process: ProcessArgs,
}

/// Lists all discovered fonts in system and custom font paths.
#[derive(Debug, Clone, Parser)]
pub struct FontsCommand {
    /// Common font arguments.
    #[clap(flatten)]
    pub font: FontArgs,

    /// Also lists style variants of each font family.
    #[arg(long)]
    pub variants: bool,
}

/// Update the CLI using a pre-compiled binary from a Typst GitHub release.
#[derive(Debug, Clone, Parser)]
pub struct UpdateCommand {
    /// Which version to update to (defaults to latest).
    pub version: Option<Version>,

    /// Forces a downgrade to an older version (required for downgrading).
    #[clap(long, default_value_t = false)]
    pub force: bool,

    /// Reverts to the version from before the last update (only possible if
    /// `typst update` has previously ran).
    #[clap(
        long,
        default_value_t = false,
        conflicts_with = "version",
        conflicts_with = "force"
    )]
    pub revert: bool,

    /// Custom path to the backup file created on update and used by `--revert`,
    /// defaults to system-dependent location
    #[clap(long = "backup-path", env = "TYPST_UPDATE_BACKUP_PATH", value_name = "FILE")]
    pub backup_path: Option<PathBuf>,
}

/// Arguments for compilation and watching.
#[derive(Debug, Clone, Args)]
pub struct CompileArgs {
    /// Path to input Typst file. Use `-` to read input from stdin.
    #[clap(value_parser = input_value_parser(), value_hint = ValueHint::FilePath)]
    pub input: Input,

    /// Path to output file (PDF, PNG, SVG, or HTML). Use `-` to write output to
    /// stdout.
    ///
    /// For output formats emitting one file per page (PNG & SVG), a page number
    /// template must be present if the source document renders to multiple
    /// pages. Use `{p}` for page numbers, `{0p}` for zero padded page numbers
    /// and `{t}` for page count. For example, `page-{0p}-of-{t}.png` creates
    /// `page-01-of-10.png`, `page-02-of-10.png`, and so on.
    #[clap(
         required_if_eq("input", "-"),
         value_parser = output_value_parser(),
         value_hint = ValueHint::FilePath,
     )]
    pub output: Option<Output>,

    /// The format of the output file, inferred from the extension by default.
    #[arg(long = "format", short = 'f')]
    pub format: Option<OutputFormat>,

    /// World arguments.
    #[clap(flatten)]
    pub world: WorldArgs,

    /// Which pages to export. When unspecified, all pages are exported.
    ///
    /// Pages to export are separated by commas, and can be either simple page
    /// numbers (e.g. '2,5' to export only pages 2 and 5) or page ranges (e.g.
    /// '2,3-6,8-' to export page 2, pages 3 to 6 (inclusive), page 8 and any
    /// pages after it).
    ///
    /// Page numbers are one-indexed and correspond to physical page numbers in
    /// the document (therefore not being affected by the document's page
    /// counter).
    #[arg(long = "pages", value_delimiter = ',')]
    pub pages: Option<Vec<Pages>>,

    /// One (or multiple comma-separated) PDF standards that Typst will enforce
    /// conformance with.
    #[arg(long = "pdf-standard", value_delimiter = ',')]
    pub pdf_standard: Vec<PdfStandard>,

    /// The PPI (pixels per inch) to use for PNG export.
    #[arg(long = "ppi", default_value_t = 144.0)]
    pub ppi: f32,

    /// File path to which a Makefile with the current compilation's
    /// dependencies will be written.
    #[clap(long = "make-deps", value_name = "PATH")]
    pub make_deps: Option<PathBuf>,

    /// Processing arguments.
    #[clap(flatten)]
    pub process: ProcessArgs,

    /// Opens the output file with the default viewer or a specific program
    /// after compilation. Ignored if output is stdout.
    #[arg(long = "open", value_name = "VIEWER")]
    pub open: Option<Option<String>>,

    /// Produces performance timings of the compilation process. (experimental)
    ///
    /// The resulting JSON file can be loaded into a tracing tool such as
    /// https://ui.perfetto.dev. It does not contain any sensitive information
    /// apart from file names and line numbers.
    #[arg(long = "timings", value_name = "OUTPUT_JSON")]
    pub timings: Option<Option<PathBuf>>,
}

/// Arguments for the construction of a world. Shared by compile, watch, and
/// query.
#[derive(Debug, Clone, Args)]
pub struct WorldArgs {
    /// Configures the project root (for absolute paths).
    #[clap(long = "root", env = "TYPST_ROOT", value_name = "DIR")]
    pub root: Option<PathBuf>,

    /// Add a string key-value pair visible through `sys.inputs`.
    #[clap(
        long = "input",
        value_name = "key=value",
        action = ArgAction::Append,
        value_parser = ValueParser::new(parse_sys_input_pair),
    )]
    pub inputs: Vec<(String, String)>,

    /// Common font arguments.
    #[clap(flatten)]
    pub font: FontArgs,

    /// Arguments related to storage of packages in the system.
    #[clap(flatten)]
    pub package: PackageArgs,

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
}

/// Arguments for configuration the process of compilation itself.
#[derive(Debug, Clone, Args)]
pub struct ProcessArgs {
    /// Number of parallel jobs spawned during compilation. Defaults to number
    /// of CPUs. Setting it to 1 disables parallelism.
    #[clap(long, short)]
    pub jobs: Option<usize>,

    /// Enables in-development features that may be changed or removed at any
    /// time.
    #[arg(long = "features", value_delimiter = ',', env = "TYPST_FEATURES")]
    pub features: Vec<Feature>,

    /// The format to emit diagnostics in.
    #[clap(long, default_value_t)]
    pub diagnostic_format: DiagnosticFormat,
}

/// Arguments related to where packages are stored in the system.
#[derive(Debug, Clone, Args)]
pub struct PackageArgs {
    /// Custom path to local packages, defaults to system-dependent location.
    #[clap(long = "package-path", env = "TYPST_PACKAGE_PATH", value_name = "DIR")]
    pub package_path: Option<PathBuf>,

    /// Custom path to package cache, defaults to system-dependent location.
    #[clap(
        long = "package-cache-path",
        env = "TYPST_PACKAGE_CACHE_PATH",
        value_name = "DIR"
    )]
    pub package_cache_path: Option<PathBuf>,
}

/// Common arguments to customize available fonts
#[derive(Debug, Clone, Parser)]
pub struct FontArgs {
    /// Adds additional directories that are recursively searched for fonts.
    ///
    /// If multiple paths are specified, they are separated by the system's path
    /// separator (`:` on Unix-like systems and `;` on Windows).
    #[clap(
        long = "font-path",
        env = "TYPST_FONT_PATHS",
        value_name = "DIR",
        value_delimiter = ENV_PATH_SEP,
    )]
    pub font_paths: Vec<PathBuf>,

    /// Ensures system fonts won't be searched, unless explicitly included via
    /// `--font-path`.
    #[arg(long)]
    pub ignore_system_fonts: bool,
}

macro_rules! display_possible_values {
    ($ty:ty) => {
        impl Display for $ty {
            fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
                self.to_possible_value()
                    .expect("no values are skipped")
                    .get_name()
                    .fmt(f)
            }
        }
    };
}

/// An input that is either stdin or a real path.
#[derive(Debug, Clone)]
pub enum Input {
    /// Stdin, represented by `-`.
    Stdin,
    /// A non-empty path.
    Path(PathBuf),
}

impl Display for Input {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Input::Stdin => f.pad("stdin"),
            Input::Path(path) => path.display().fmt(f),
        }
    }
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

/// Which format to use for the generated output file.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, ValueEnum)]
pub enum OutputFormat {
    Pdf,
    Png,
    Svg,
    Html,
}

display_possible_values!(OutputFormat);

/// Which format to use for diagnostics.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, ValueEnum)]
pub enum DiagnosticFormat {
    #[default]
    Human,
    Short,
}

display_possible_values!(DiagnosticFormat);

/// An in-development feature that may be changed or removed at any time.
#[derive(Debug, Copy, Clone, Eq, PartialEq, ValueEnum)]
pub enum Feature {
    Html,
}

display_possible_values!(Feature);

/// A PDF standard that Typst can enforce conformance with.
#[derive(Debug, Copy, Clone, Eq, PartialEq, ValueEnum)]
#[allow(non_camel_case_types)]
pub enum PdfStandard {
    /// PDF 1.7.
    #[value(name = "1.7")]
    V_1_7,
    /// PDF/A-2b.
    #[value(name = "a-2b")]
    A_2b,
}

display_possible_values!(PdfStandard);

// Output file format for query command
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, ValueEnum)]
pub enum SerializationFormat {
    #[default]
    Json,
    Yaml,
}

display_possible_values!(SerializationFormat);

/// Implements parsing of page ranges (`1-3`, `4`, `5-`, `-2`), used by the
/// `CompileCommand.pages` argument, through the `FromStr` trait instead of a
/// value parser, in order to generate better errors.
///
/// See also: https://github.com/clap-rs/clap/issues/5065
#[derive(Debug, Clone)]
pub struct Pages(pub RangeInclusive<Option<NonZeroUsize>>);

impl FromStr for Pages {
    type Err = &'static str;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.split('-').map(str::trim).collect::<Vec<_>>().as_slice() {
            [] | [""] => Err("page export range must not be empty"),
            [single_page] => {
                let page_number = parse_page_number(single_page)?;
                Ok(Pages(Some(page_number)..=Some(page_number)))
            }
            ["", ""] => Err("page export range must have start or end"),
            [start, ""] => Ok(Pages(Some(parse_page_number(start)?)..=None)),
            ["", end] => Ok(Pages(None..=Some(parse_page_number(end)?))),
            [start, end] => {
                let start = parse_page_number(start)?;
                let end = parse_page_number(end)?;
                if start > end {
                    Err("page export range must end at a page after the start")
                } else {
                    Ok(Pages(Some(start)..=Some(end)))
                }
            }
            [_, _, _, ..] => Err("page export range must have a single hyphen"),
        }
    }
}

/// Parses a single page number.
fn parse_page_number(value: &str) -> Result<NonZeroUsize, &'static str> {
    if value == "0" {
        Err("page numbers start at one")
    } else {
        NonZeroUsize::from_str(value).map_err(|_| "not a valid page number")
    }
}

/// The clap value parser used by `SharedArgs.input`
fn input_value_parser() -> impl TypedValueParser<Value = Input> {
    clap::builder::OsStringValueParser::new().try_map(|value| {
        if value.is_empty() {
            Err(clap::Error::new(clap::error::ErrorKind::InvalidValue))
        } else if value == "-" {
            Ok(Input::Stdin)
        } else {
            Ok(Input::Path(value.into()))
        }
    })
}

/// The clap value parser used by `CompileCommand.output`
fn output_value_parser() -> impl TypedValueParser<Value = Output> {
    clap::builder::OsStringValueParser::new().try_map(|value| {
        // Empty value also handled by clap for `Option<Output>`
        if value.is_empty() {
            Err(clap::Error::new(clap::error::ErrorKind::InvalidValue))
        } else if value == "-" {
            Ok(Output::Stdout)
        } else {
            Ok(Output::Path(value.into()))
        }
    })
}

/// Parses key/value pairs split by the first equal sign.
///
/// This function will return an error if the argument contains no equals sign
/// or contains the key (before the equals sign) is empty.
fn parse_sys_input_pair(raw: &str) -> Result<(String, String), String> {
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

/// Parses a UNIX timestamp according to <https://reproducible-builds.org/specs/source-date-epoch/>
fn parse_source_date_epoch(raw: &str) -> Result<DateTime<Utc>, String> {
    let timestamp: i64 = raw
        .parse()
        .map_err(|err| format!("timestamp must be decimal integer ({err})"))?;
    DateTime::from_timestamp(timestamp, 0)
        .ok_or_else(|| "timestamp out of range".to_string())
}
