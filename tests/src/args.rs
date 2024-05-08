use clap::{Parser, Subcommand};

/// Typst's test runner.
#[derive(Debug, Clone, Parser)]
#[clap(name = "typst-test", author)]
pub struct CliArguments {
    /// The command to run.
    #[command(subcommand)]
    pub command: Option<Command>,
    /// All the tests that contain the filter string will be run.
    pub filter: Vec<String>,
    /// Runs only the tests with the exact specified `filter` names.
    #[arg(short, long)]
    pub exact: bool,
    /// Whether to update the reference images of non-passing tests.
    #[arg(short, long)]
    pub update: bool,
    /// The scaling factor to render the output image with.
    ///
    /// Does not affect the comparison or the reference image.
    #[arg(short, long, default_value_t = 1.0)]
    pub scale: f32,
    /// Whether to run the tests in extended mode, including PDF and SVG
    /// export.
    ///
    /// This is used in CI.
    #[arg(long, env = "TYPST_TESTS_EXTENDED")]
    pub extended: bool,
    /// Runs PDF export.
    #[arg(long)]
    pub pdf: bool,
    /// Runs SVG export.
    #[arg(long)]
    pub svg: bool,
    /// Whether to display the syntax tree.
    #[arg(long)]
    pub syntax: bool,
    /// Prevents the terminal from being cleared of test names.
    #[arg(short, long)]
    pub verbose: bool,
    /// How many threads to spawn when running the tests.
    #[arg(short = 'j', long)]
    pub num_threads: Option<usize>,
}

impl CliArguments {
    /// Whether to run PDF export.
    pub fn pdf(&self) -> bool {
        self.pdf || self.extended
    }

    /// Whether to run SVG export.
    pub fn svg(&self) -> bool {
        self.svg || self.extended
    }
}

/// What to do.
#[derive(Debug, Clone, Subcommand)]
#[command()]
pub enum Command {
    /// Clears the on-disk test artifact store.
    Clean,
}
