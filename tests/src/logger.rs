use std::fs;
use std::io::{self, IsTerminal, StderrLock, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use ecow::EcoString;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use rustc_hash::FxHashMap;

use crate::collect::{FilePos, Test};
use crate::report::{ReportFile, TestReport};
use crate::{ARGS, report};

#[derive(Copy, Clone)]
pub enum SuiteError {
    Generic,
    /// The code is used by the wrapper to prompt regenerating missing
    /// old live output.
    PromptRegen,
}

impl SuiteError {
    pub fn exit_code(self) -> i32 {
        match self {
            SuiteError::Generic => 1,
            SuiteError::PromptRegen => 15,
        }
    }
}

/// The result of running a single test.
#[derive(Default)]
pub struct TestResult {
    /// The error log for this test. If empty, the test passed.
    pub errors: String,
    /// The info log for this test.
    pub infos: String,
    /// Whether the output was mismatched.
    pub mismatched_output: bool,
    /// The data necessary to generate a HTML report.
    pub report: Option<TestReport>,
    /// Updates for a test's error annotations.
    pub updated_body: Option<String>,
}

impl TestResult {
    /// Add a report to this result, potentially initializing the option.
    pub fn add_report(&mut self, name: EcoString, file_report: ReportFile) {
        let report = self.report.get_or_insert_with(|| TestReport::new(name));
        report.files.push(file_report);
    }
}

/// Receives status updates by individual test runs.
pub struct Logger {
    selected: usize,
    passed: usize,
    failed: usize,
    skipped: usize,
    mismatched_output: bool,
    active: Vec<EcoString>,
    last_change: Instant,
    temp_lines: usize,
    terminal: bool,
    pub reports: Vec<TestReport>,
    /// Updates to error annotations per file (so we can write each file
    /// atomically).
    pub test_updates: FxHashMap<Arc<PathBuf>, Vec<(usize, String)>>,
}

impl Logger {
    /// Create a new logger.
    pub fn new(selected: usize, skipped: usize) -> Self {
        Self {
            selected,
            passed: 0,
            failed: 0,
            skipped,
            mismatched_output: false,
            active: vec![],
            temp_lines: 0,
            last_change: Instant::now(),
            terminal: std::io::stderr().is_terminal(),
            reports: vec![],
            test_updates: FxHashMap::default(),
        }
    }

    /// Register the start of a test.
    pub fn start(&mut self, test: &Test) {
        self.active.push(test.name.clone());
        self.last_change = Instant::now();
        self.refresh();
    }

    /// Register a finished test.
    pub fn end(&mut self, test: Test, result: std::thread::Result<TestResult>) {
        self.active.retain(|name| *name != test.name);

        let Ok(result) = result else {
            self.failed += 1;
            self.temp_lines = 0;
            self.print(move |out| {
                writeln!(out, "❌ {test} panicked")?;
                Ok(())
            })
            .unwrap();
            return;
        };
        let TestResult {
            errors,
            infos,
            mismatched_output,
            report,
            updated_body,
        } = result;

        if errors.is_empty() {
            self.passed += 1;
        } else {
            self.failed += 1;
        }

        self.mismatched_output |= mismatched_output;
        self.last_change = Instant::now();

        self.reports.extend(report);

        if let Some(new_body) = updated_body {
            let FilePos { path, line } = test.body.pos.clone();
            self.test_updates.entry(path).or_default().push((line, new_body));
        }

        self.print(move |out| {
            if !errors.is_empty() {
                if ARGS.use_github_annotations {
                    let file = test.body.pos.path.display();
                    let line = test.body.pos.line;
                    write!(out, "::error file={file},line={line}::{test}")?;
                    for line in errors.lines() {
                        write!(out, "%0A  {line}")?;
                    }
                    writeln!(out)?;
                } else {
                    writeln!(out, "❌ {test}")?;
                    if !ARGS.compact {
                        for line in errors.lines() {
                            writeln!(out, "  {line}")?;
                        }
                    }
                }
            } else if ARGS.verbose || !infos.is_empty() {
                writeln!(out, "✅ {test}")?;
            }
            for line in infos.lines() {
                writeln!(out, "  {line}")?;
            }
            Ok(())
        })
        .unwrap();
    }

    /// Prints a summary and returns whether the test suite passed.
    pub fn finish(self) -> Result<(), SuiteError> {
        let Self {
            selected,
            passed,
            failed,
            skipped,
            reports,
            test_updates,
            ..
        } = self;

        test_updates.into_par_iter().for_each(|(path, updates)| {
            update_test_bodies(&path, updates);
        });

        eprintln!("{passed} passed, {failed} failed, {skipped} skipped");
        assert_eq!(selected, passed + failed, "not all tests were executed successfully");

        if self.mismatched_output {
            eprintln!(
                "  pass '--update' to update error annotations or reference outputs"
            );
            eprintln!("  for a rich diff, view tests/store/report.html");
        }

        let mut prompt_regen = false;
        if ARGS.gen_report() {
            prompt_regen = report::write(reports).unwrap_or(false);

            if ARGS.open_report {
                let res = open::that("tests/store/report.html");
                if let Err(err) = res {
                    eprintln!("failed to open `tests/store/report.html`: {err}");
                }
            }
        }

        if self.failed == 0 {
            Ok(())
        } else if prompt_regen {
            Err(SuiteError::PromptRegen)
        } else {
            Err(SuiteError::Generic)
        }
    }

    /// Refresh the status. Returns whether we still seem to be making progress.
    pub fn refresh(&mut self) -> bool {
        self.print(|_| Ok(())).unwrap();
        self.last_change.elapsed() < Duration::from_secs(10)
    }

    /// Refresh the status print.
    fn print(
        &mut self,
        inner: impl FnOnce(&mut StderrLock<'_>) -> io::Result<()>,
    ) -> io::Result<()> {
        let mut out = std::io::stderr().lock();

        // Clear the status lines.
        for _ in 0..self.temp_lines {
            write!(out, "\x1B[1F\x1B[0J")?;
            self.temp_lines = 0;
        }

        // Print the result of a finished test.
        inner(&mut out)?;

        // Print the status line.
        let done = self.failed + self.passed;
        if done < self.selected {
            if self.last_change.elapsed() > Duration::from_secs(2) {
                for test in &self.active {
                    writeln!(out, "⏰ {test} is taking a long time ...")?;
                    if self.terminal {
                        self.temp_lines += 1;
                    }
                }
            }
            if self.terminal {
                writeln!(out, "💨 {done} / {}", self.selected)?;
                self.temp_lines += 1;
            }
        }

        Ok(())
    }
}

/// Write updated test bodies.
pub fn update_test_bodies(path: &Path, mut updates: Vec<(usize, String)>) {
    let old = fs::read_to_string(path).unwrap();
    let mut new = String::with_capacity(old.len());
    let mut lines = old.lines().enumerate();

    updates.sort_by_key(|(line, _body)| *line);
    for (index, new_body) in updates {
        // Copy over all lines up to the body.
        while let Some((i, line)) = lines.next()
            && i + 1 < index
        {
            new.push_str(line);
            new.push('\n');
        }
        // Write the new body.
        new.push_str(&new_body);
        // Skip lines from the original body.
        for (_, line) in lines.by_ref() {
            if line.starts_with("---") {
                // At a new test, write its header and stop skipping.
                new.push_str(line);
                new.push('\n');
                break;
            }
        }
    }
    // Write any remaining lines.
    for (_, line) in lines {
        new.push_str(line);
        new.push('\n');
    }

    fs::write(path, new).unwrap();
}
