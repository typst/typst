//! Typst's test runner.

mod args;
mod collect;
mod custom;
mod logger;
mod run;
mod world;

use std::path::Path;
use std::sync::LazyLock;
use std::time::Duration;

use clap::Parser;
use parking_lot::Mutex;
use rayon::iter::{ParallelBridge, ParallelIterator};

use crate::args::{CliArguments, Command};
use crate::logger::Logger;

/// The parsed command line arguments.
static ARGS: LazyLock<CliArguments> = LazyLock::new(CliArguments::parse);

/// The directory where the test suite is located.
const SUITE_PATH: &str = "tests/suite";

/// The directory where the full test results are stored.
const STORE_PATH: &str = "tests/store";

/// The directory where the reference images are stored.
const REF_PATH: &str = "tests/ref";

/// The file where the skipped tests are stored.
const SKIP_PATH: &str = "tests/skip.txt";

/// The maximum size of reference images that aren't marked as `// LARGE`.
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
    std::env::set_current_dir("..").unwrap();

    // Create the storage.
    for ext in ["render", "pdf", "svg"] {
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
            let result = std::panic::catch_unwind(|| run::run(test));
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
        Ok(_) => eprintln!("no danging reference images"),
        Err(errors) => {
            for error in errors {
                if error.message == "dangling reference image" {
                    std::fs::remove_file(&error.pos.path).unwrap();
                    eprintln!("✅ deleted {}", error.pos.path.display());
                }
            }
        }
    }
}
