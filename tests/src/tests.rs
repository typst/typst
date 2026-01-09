//! Typst's test runner.

mod args;
mod collect;
mod custom;
mod logger;
mod output;
mod pdftags;
mod report;
mod run;
mod world;

use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::LazyLock;
use std::sync::atomic::AtomicUsize;
use std::time::Duration;

use clap::Parser;
use parking_lot::{Mutex, RwLock};
use rustc_hash::FxHashMap;

use crate::args::{CliArguments, Command};
use crate::collect::{Test, TestParseErrorKind};
use crate::logger::{Logger, TestResult};
use crate::output::{HASH_OUTPUTS, HashedRefs};

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
    for dir in ["render", "html", "pdf", "pdftags", "svg", "by-hash"] {
        std::fs::create_dir_all(Path::new(STORE_PATH).join(dir)).unwrap();
    }
}

fn test() {
    let (mut hashes, tests, skipped) = match crate::collect::collect() {
        Ok(output) => output,
        Err(errors) => {
            eprintln!("failed to collect tests");
            for error in errors {
                eprintln!("❌ {error}");
            }
            std::process::exit(1);
        }
    };

    // Read the reference hashes at the specified git base revision instead.
    let repo = if let Some(rev) = &ARGS.base_revision {
        let repo = gix::open(".").unwrap();
        {
            let Some(tree) = commit_tree(&repo, rev) else {
                eprintln!("couldn't find base-revision: {rev}");
                std::process::exit(1);
            };
            eprintln!("compairing against base revision: {}", tree.id);

            hashes = HASH_OUTPUTS.map(|output| {
                // TODO: proper error handling
                let entry = tree.lookup_entry_by_path(output.hash_refs_path()).unwrap();
                let Some(entry) = entry else {
                    eprintln!("Couldn't read hashed references on revision `{rev}`");
                    std::process::exit(1);
                };
                let obj = entry.object().unwrap();
                assert_eq!(obj.kind, gix::object::Kind::Blob);
                let str = std::str::from_utf8(&obj.data).unwrap();
                HashedRefs::from_str(str).unwrap()
            });
        }

        Some(repo)
    } else {
        None
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

    let hashes = hashes.map(RwLock::new);
    let runner = |tree: Option<&gix::Tree>, test: &Test| {
        if let Some((live_path, ref_path)) = &parser_dirs {
            run_parser_test(test, live_path, ref_path)
        } else {
            run::run(tree, &hashes, test)
        }
    };

    // Run the tests.
    let logger = Mutex::new(Logger::new(selected, skipped));
    let (idle_sender, idle_receiver) = std::sync::mpsc::sync_channel(1);
    let tests = SyncSliceIter::new(&tests);
    std::thread::scope(|scope| {
        let logger = &logger;

        // Regularly refresh the logger in case we make no progress.
        scope.spawn(move || {
            while idle_receiver.recv_timeout(Duration::from_millis(500)).is_err() {
                if !logger.lock().refresh() {
                    eprintln!("tests seem to be stuck");
                    std::process::exit(1);
                }
            }
        });

        // Build a worker pool.
        let num_workers = (ARGS.num_threads)
            .or_else(|| std::thread::available_parallelism().ok())
            .unwrap_or(NonZeroUsize::new(4).unwrap());
        let workers = (0..num_workers.get())
            .map(|_| {
                let repo = repo.clone();
                let tests = &tests;
                scope.spawn(move || {
                    let base_rev_tree = repo
                        .as_ref()
                        .zip(ARGS.base_revision.as_ref())
                        .map(|(repo, rev)| commit_tree(repo, rev).unwrap());

                    while let Some(test) = tests.next() {
                        logger.lock().start(test);

                        // This is in fact not formally unwind safe, but the code paths that
                        // hold a lock of the hashes are quite short and shouldn't panic.
                        let closure = std::panic::AssertUnwindSafe(|| {
                            runner(base_rev_tree.as_ref(), test)
                        });
                        let result = std::panic::catch_unwind(closure);
                        logger.lock().end(test, result);
                    }
                })
            })
            .collect::<Vec<_>>();

        for worker in workers {
            worker.join().unwrap();
        }

        idle_sender.send(()).unwrap();
    });

    if ARGS.update {
        run::update_hash_refs::<output::Pdf>(&hashes);
        run::update_hash_refs::<output::Svg>(&hashes);
    }

    let mut logger = logger.into_inner();
    let report_path = Path::new(STORE_PATH).join("report.html");
    if ARGS.no_report {
        _ = std::fs::remove_file(report_path);
    } else {
        logger.reports.sort_by(|a, b| a.name.cmp(&b.name));
        let html = report::html::generate(&logger.reports);
        std::fs::write(report_path, html).unwrap();
    }

    let passed = logger.finish();
    if !passed {
        std::process::exit(1);
    }
}

fn clean() {
    std::fs::remove_dir_all(STORE_PATH).unwrap();
}

fn undangle() {
    match crate::collect::collect() {
        Ok(_) => eprintln!("no dangling reference output"),
        Err(errors) => {
            let mut dangling_hashes = FxHashMap::<&Path, Vec<usize>>::default();
            for error in errors.iter() {
                match &error.kind {
                    TestParseErrorKind::DanglingFile => {
                        std::fs::remove_file(&error.pos.path).unwrap();
                        eprintln!("✅ deleted {}", error.pos.path.display());
                    }
                    TestParseErrorKind::DanglingHash(name) => {
                        eprintln!("✅ removed hash {name} {}", error.pos);
                        let lines = dangling_hashes.entry(&error.pos.path).or_default();
                        lines.push(error.pos.line);
                    }
                    TestParseErrorKind::Other(_) => (),
                }
            }

            // Remove dangling hashes from file.
            for (path, mut line_nrs) in dangling_hashes {
                line_nrs.sort();
                let text = std::fs::read_to_string(path).unwrap();
                let mut lines = { text.lines().collect::<Vec<_>>() };
                for nr in line_nrs.iter().rev() {
                    lines.remove(nr - 1);
                }
                let mut updated = String::with_capacity(text.len());
                for line in lines.iter() {
                    updated.push_str(line);
                    updated.push('\n');
                }
                std::fs::write(path, updated).unwrap();
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
        report: None,
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

fn commit_tree<'r>(repo: &'r gix::Repository, rev: &str) -> Option<gix::Tree<'r>> {
    let id = repo.rev_parse_single(rev).ok()?;
    let commit = repo.find_commit(id).ok()?;
    commit.tree().ok()
}

struct SyncSliceIter<'a, T> {
    idx: AtomicUsize,
    slice: &'a [T],
}

impl<'a, T> SyncSliceIter<'a, T> {
    pub fn new(slice: &'a [T]) -> Self {
        Self { idx: AtomicUsize::new(0), slice }
    }

    pub fn next(&self) -> Option<&'a T> {
        let idx = self.idx.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        self.slice.get(idx)
    }
}
