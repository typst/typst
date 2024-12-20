use std::io::{self, IsTerminal, StderrLock, Write};
use std::time::{Duration, Instant};

use crate::collect::Test;

/// The result of running a single test.
pub struct TestResult {
    /// The error log for this test. If empty, the test passed.
    pub errors: String,
    /// The info log for this test.
    pub infos: String,
    /// Whether the output was mismatched.
    pub mismatched_output: bool,
}

/// Receives status updates by individual test runs.
pub struct Logger<'a> {
    selected: usize,
    passed: usize,
    failed: usize,
    skipped: usize,
    mismatched_output: bool,
    active: Vec<&'a Test>,
    last_change: Instant,
    temp_lines: usize,
    terminal: bool,
}

impl<'a> Logger<'a> {
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
        }
    }

    /// Register the start of a test.
    pub fn start(&mut self, test: &'a Test) {
        self.active.push(test);
        self.last_change = Instant::now();
        self.refresh();
    }

    /// Register a finished test.
    pub fn end(&mut self, test: &'a Test, result: std::thread::Result<TestResult>) {
        self.active.retain(|t| t.name != test.name);

        let result = match result {
            Ok(result) => result,
            Err(_) => {
                self.failed += 1;
                self.temp_lines = 0;
                self.print(move |out| {
                    writeln!(out, "❌ {test} panicked")?;
                    Ok(())
                })
                .unwrap();
                return;
            }
        };

        if result.errors.is_empty() {
            self.passed += 1;
        } else {
            self.failed += 1;
        }

        self.mismatched_output |= result.mismatched_output;
        self.last_change = Instant::now();

        self.print(move |out| {
            if !result.errors.is_empty() {
                writeln!(out, "❌ {test}")?;
                if !crate::ARGS.compact {
                    for line in result.errors.lines() {
                        writeln!(out, "  {line}")?;
                    }
                }
            } else if crate::ARGS.verbose || !result.infos.is_empty() {
                writeln!(out, "✅ {test}")?;
            }
            for line in result.infos.lines() {
                writeln!(out, "  {line}")?;
            }
            Ok(())
        })
        .unwrap();
    }

    /// Prints a summary and returns whether the test suite passed.
    pub fn finish(&self) -> bool {
        let Self { selected, passed, failed, skipped, .. } = *self;

        eprintln!("{passed} passed, {failed} failed, {skipped} skipped");
        assert_eq!(selected, passed + failed, "not all tests were executed successfully");

        if self.mismatched_output {
            eprintln!("  pass the --update flag to update the reference output");
        }

        self.failed == 0
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
