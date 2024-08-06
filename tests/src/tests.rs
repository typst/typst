//! Typst's test runner.

use std::path::Path;
use std::time::Duration;

use parking_lot::Mutex;
use rayon::iter::{ParallelBridge, ParallelIterator};
use typst_tests::args::Command;
use typst_tests::logger::Logger;
use typst_tests::{constants, run, ARGS};

fn main() {
    setup();

    match &ARGS.command {
        None => test(),
        Some(Command::Clean) => std::fs::remove_dir_all(constants::STORE_PATH).unwrap(),
    }
}

fn setup() {
    // Make all paths relative to the workspace. That's nicer for IDEs when
    // clicking on paths printed to the terminal.
    std::env::set_current_dir("..").unwrap();

    // Create the storage.
    for ext in ["render", "pdf", "svg"] {
        std::fs::create_dir_all(Path::new(constants::STORE_PATH).join(ext)).unwrap();
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
    let (tests, skipped) = typst_tests::collect::collect_or_exit();

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
                logger.lock().refresh();
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
