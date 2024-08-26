//! Typst's benchmark runner.

use base64::Engine;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use typst_tests::args::Command;
use typst_tests::{constants, ARGS};

fn main() {
    setup();

    match &ARGS.command {
        None => prepare_benches(),
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

fn prepare_benches() {
    let (tests, skipped) = typst_tests::collect::collect_or_exit();

    let selected = tests.len();
    if ARGS.list {
        for test in tests.iter() {
            println!("{test}");
        }
        eprintln!("{selected} selected, {skipped} skipped");
        return;
    } else if selected == 0 {
        eprintln!("no benches selected");
        return;
    }

    // Prepare the benchmarks.
    let mut out_file = File::create("tests/store/fixtures").unwrap();
    let mut source_base64 = String::new();
    for test in &tests {
        source_base64.clear();
        base64::engine::general_purpose::STANDARD
            .encode_string(test.source.text(), &mut source_base64);
        writeln!(out_file, "{} {}", test.name, source_base64).unwrap();
    }
}
