use std::path::PathBuf;

use clap::{Parser, Subcommand};
use regex::Regex;

/// Typst's test runner.
#[derive(Debug, Clone, Parser)]
#[command(bin_name = "cargo test --workspace --test tests --")]
#[clap(name = "typst-test", author)]
pub struct CliArguments {
    /// The command to run.
    #[command(subcommand)]
    pub command: Option<Command>,
    /// All the tests whose names match the test name pattern will be run.
    #[arg(value_parser = Regex::new)]
    pub pattern: Vec<Regex>,
    /// Restricts test selection within the given path.
    #[arg(short, long, value_parser = |s: &str| PathBuf::from(s).canonicalize())]
    pub path: Vec<PathBuf>,
    /// Only selects the test that matches with the test name verbatim.
    #[arg(short, long)]
    pub exact: bool,
    /// Lists what tests will be run, without actually running them.
    #[arg(long, group = "action")]
    pub list: bool,
    /// Updates the reference images of non-passing tests.
    #[arg(short, long, group = "action")]
    pub update: bool,
    /// The scaling factor to render the output image with.
    ///
    /// Does not affect the comparison or the reference image.
    #[arg(short, long, default_value_t = 1.0)]
    pub scale: f32,
    /// Exports PDF outputs into the artifact store.
    #[arg(long)]
    pub pdf: bool,
    /// Exports SVG outputs into the artifact store.
    #[arg(long)]
    pub svg: bool,
    /// Displays the syntax tree.
    #[arg(long)]
    pub syntax: bool,
    /// Prevents the terminal from being cleared of test names.
    #[arg(short, long)]
    pub verbose: bool,
    /// How many threads to spawn when running the tests.
    #[arg(short = 'j', long)]
    pub num_threads: Option<usize>,
}

/// What to do.
#[derive(Debug, Clone, Subcommand)]
#[command()]
pub enum Command {
    /// Clears the on-disk test artifact store.
    Clean,
}
