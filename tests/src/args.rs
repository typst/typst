use std::path::PathBuf;
use std::sync::atomic::{AtomicU8, Ordering};

use clap::{Parser, Subcommand, ValueEnum};
use regex::Regex;

use crate::collect::TestStages;

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
    /// Updates the reference output of non-passing tests.
    #[arg(short, long, group = "action")]
    pub update: bool,
    /// Specify which test targets/outputs to run.
    ///
    /// This is useful to only update specific reference outputs of a test.
    #[arg(long, value_delimiter = ',')]
    pub stages: Vec<TestStage>,
    /// The scaling factor to render the output image with.
    ///
    /// Does not affect the comparison or the reference image.
    #[arg(short, long, default_value_t = 1.0)]
    pub scale: f32,
    /// Displays the syntax tree before running tests.
    ///
    /// Note: This is ignored if using '--syntax-compare'.
    #[arg(long)]
    pub syntax: bool,
    /// Displays only one line per test, hiding details about failures.
    #[arg(short, long)]
    pub compact: bool,
    /// Prevents the terminal from being cleared of test names.
    #[arg(short, long)]
    pub verbose: bool,
    /// How many threads to spawn when running the tests.
    #[arg(short = 'j', long)]
    pub num_threads: Option<usize>,
    /// Changes testing behavior for debugging the parser: With no argument,
    /// outputs the concrete syntax trees of tests as files in
    /// 'tests/store/syntax/'. With a directory as argument, will treat it as a
    /// reference of correct syntax tree files and will print which output
    /// syntax trees differ (viewing the diffs is on you).
    ///
    /// This overrides the normal testing system. It parses, but does not run
    /// the test suite.
    ///
    /// If `cargo test` is run with `--no-default-features`, then compiling will
    /// not include Typst's core crates, only typst-syntax, greatly speeding up
    /// debugging when changing the parser.
    ///
    /// You can generate a correct reference directory by running on a known
    /// good commit and copying the generated outputs to a new directory.
    /// `_things` may be a good location as it is in the top-level gitignore.
    ///
    /// You can view diffs in VS Code with: `code --diff <ref_dir>/<test>.syntax
    /// tests/store/syntax/<test>.syntax`
    #[arg(long)]
    pub parser_compare: Option<Option<PathBuf>>,
    // ^ I'm not using a subcommand here because then test patterns don't parse
    // how you would expect and I'm too lazy to try to fix it.
}

impl CliArguments {
    /// The stages which should be run depending on the `--stages` flag.
    pub fn stages(&self) -> TestStages {
        static CACHED: AtomicU8 = AtomicU8::new(0xFF);

        if CACHED.load(Ordering::Relaxed) == 0xFF {
            let mut stages = TestStages::empty();
            if self.stages.is_empty() {
                stages = TestStages::all();
            } else {
                for &s in self.stages.iter() {
                    stages |= s.into();
                }

                stages = stages.with_implied();
            };

            CACHED.store(stages.bits(), Ordering::Relaxed);
        }

        TestStages::from_bits(CACHED.load(Ordering::Relaxed)).unwrap()
    }

    /// Whether the stage should be run depending on the `--stages` flag.
    pub fn should_run(&self, stage: TestStages) -> bool {
        self.stages().intersects(stage)
    }
}

/// What to do.
#[derive(Debug, Clone, Subcommand)]
#[command()]
pub enum Command {
    /// Clears the on-disk test artifact store.
    Clean,
    /// Deletes all dangling reference output.
    Undangle,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, ValueEnum)]
pub enum TestStage {
    Paged,
    Render,
    Pdf,
    Pdftags,
    Svg,
    Html,
}

impl From<TestStage> for TestStages {
    fn from(value: TestStage) -> Self {
        match value {
            TestStage::Paged => TestStages::PAGED,
            TestStage::Render => TestStages::RENDER,
            TestStage::Pdf => TestStages::PDF,
            TestStage::Pdftags => TestStages::PDFTAGS,
            TestStage::Svg => TestStages::SVG,
            TestStage::Html => TestStages::HTML,
        }
    }
}
