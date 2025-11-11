//! Typst's test runner.

mod args;
mod collect;
mod custom;
mod logger;
mod run;
mod world;

use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use std::time::Duration;

use clap::Parser;
use parking_lot::Mutex;
use rayon::iter::{ParallelBridge, ParallelIterator};

use crate::args::{CliArguments, Command};
use crate::collect::Test;
use crate::logger::{Logger, TestResult};

/// The parsed command line arguments.
static ARGS: LazyLock<CliArguments> = LazyLock::new(CliArguments::parse);

/// The directory where the test suite is located.
const SUITE_PATH: &str = "tests/suite";

/// The directory where the full test results are stored.
const STORE_PATH: &str = "tests/store";

/// The directory where syntax trees are stored.
const SYNTAX_PATH: &str = "tests/store/syntax";

/// The directory where the reference output is stored.
const REF_PATH: &str = "tests/ref";

/// The file where the skipped tests are stored.
const SKIP_PATH: &str = "tests/skip.txt";

/// The maximum size of reference output that isn't marked as `large`.
const REF_LIMIT: usize = 20 * 1024;

fn main() {
    setup();

    match &ARGS.command {
        None => test(),
        Some(Command::Clean) => clean(),
        Some(Command::Undangle) => undangle(),
    }
}

fn setup() {
    // Make all paths relative to the workspace. That's nicer for IDEs when
    // clicking on paths printed to the terminal.
    let workspace_dir =
        Path::new(env!("CARGO_MANIFEST_DIR")).join(std::path::Component::ParentDir);
    std::env::set_current_dir(workspace_dir).unwrap();

    // Create the storage.
    for ext in ["render", "html", "pdf", "pdftags", "svg"] {
        std::fs::create_dir_all(Path::new(STORE_PATH).join(ext)).unwrap();
    }

    // Set up the thread pool.
    if let Some(num_threads) = ARGS.num_threads {
        rayon::ThreadPoolBuilder::new()
            .num_threads(num_threads)
            .build_global()
            .unwrap();
    }
}

fn test() {
    let (tests, skipped) = match crate::collect::collect() {
        Ok(output) => output,
        Err(errors) => {
            eprintln!("failed to collect tests");
            for error in errors {
                eprintln!("❌ {error}");
            }
            std::process::exit(1);
        }
    };

    let selected = tests.len();
    if ARGS.list {
        for test in tests.iter() {
            println!("{test}");
        }
        eprintln!("{selected} selected, {skipped} skipped");
        return;
    } else if selected == 0 {
        eprintln!("no test selected");
        return;
    }

    let parser_dirs = ARGS.parser_compare.clone().map(create_syntax_store);

    let runner = |test: &Test| {
        if let Some((live_path, ref_path)) = &parser_dirs {
            run_parser_test(test, live_path, ref_path)
        } else {
            run::run(test)
        }
    };

    // Run the tests.
    let logger = Mutex::new(Logger::new(selected, skipped));
    std::thread::scope(|scope| {
        let logger = &logger;
        let (sender, receiver) = std::sync::mpsc::channel();

        // Regularly refresh the logger in case we make no progress.
        scope.spawn(move || {
            while receiver.recv_timeout(Duration::from_millis(500)).is_err() {
                if !logger.lock().refresh() {
                    eprintln!("tests seem to be stuck");
                    std::process::exit(1);
                }
            }
        });

        // Run the tests.
        //
        // We use `par_bridge` instead of `par_iter` because the former
        // results in a stack overflow during PDF export. Probably related
        // to `typst::utils::Deferred` yielding.
        tests.iter().par_bridge().for_each(|test| {
            logger.lock().start(test);
            let result = std::panic::catch_unwind(|| runner(test));
            logger.lock().end(test, result);
        });

        sender.send(()).unwrap();
    });

    let passed = logger.into_inner().finish();
    if !passed {
        std::process::exit(1);
    }
}

fn clean() {
    std::fs::remove_dir_all(STORE_PATH).unwrap();
}

fn undangle() {
    match crate::collect::collect() {
        Ok(_) => eprintln!("no danging reference output"),
        Err(errors) => {
            for error in errors {
                if error.message == "dangling reference output" {
                    std::fs::remove_file(&error.pos.path).unwrap();
                    eprintln!("✅ deleted {}", error.pos.path.display());
                }
            }
        }
    }
}

fn create_syntax_store(ref_path: Option<PathBuf>) -> (&'static Path, Option<PathBuf>) {
    if ref_path.as_ref().is_some_and(|p| !p.exists()) {
        eprintln!("syntax reference path doesn't exist");
        std::process::exit(1);
    }

    let live_path = Path::new(SYNTAX_PATH);
    std::fs::remove_dir_all(live_path).ok();
    std::fs::create_dir_all(live_path).unwrap();
    (live_path, ref_path)
}

fn run_parser_test(
    test: &Test,
    live_path: &Path,
    ref_path: &Option<PathBuf>,
) -> TestResult {
    let mut result = TestResult {
        errors: String::new(),
        infos: String::new(),
        mismatched_output: false,
    };

    let syntax_file = live_path.join(format!("{}.syntax", test.name));
    let tree = format!("{:#?}\n", test.source.root());
    std::fs::write(syntax_file, &tree).unwrap();

    let Some(ref_path) = ref_path else { return result };
    let ref_file = ref_path.join(format!("{}.syntax", test.name));
    match std::fs::read_to_string(&ref_file) {
        Ok(ref_tree) => {
            if tree != ref_tree {
                result.errors = "differs".to_string();
            }
        }
        Err(_) => {
            result.errors = format!("missing reference: {}", ref_file.display());
        }
    }

    result
}
