use std::path::PathBuf;

// use typst::geom::Color;
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

    #[arg(long, short, default_value_t = false)]
    pub image: bool,

    #[arg(long, requires = "image", default_value_t = 1.0)]
    pub pixel_per_pt: f32,
    // #[arg(long, requires = "image")]
    // pub fill: Color,
}

/// List all discovered fonts in system and custom font paths
#[derive(Debug, Clone, Parser)]
pub struct FontsCommand {
    /// Also list style variants of each font family
    #[arg(long)]
    pub variants: bool,
}
