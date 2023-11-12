use std::io::{self, IsTerminal, Write};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

use codespan_reporting::term::{self, termcolor};
use termcolor::{ColorChoice, WriteColor};

use crate::args::{Command, DiagnosticFormat};

/// A utility that allows users to write colored terminal output.
/// If colors are not supported by the terminal, they are disabled.
/// This type also allows for deletion of previously written lines.
#[derive(Clone)]
pub struct TermOut {
    inner: Arc<TermOutInner>,
}

/// The stuff that has to be shared between instances of [`TermOut`].
struct TermOutInner {
    active: AtomicBool,
    stream: termcolor::StandardStream,
    lines_written: AtomicUsize,
    in_alternate_screen: AtomicBool,
}

impl TermOut {
    pub fn new() -> Self {
        let color_choice = match ARGS.color {
            clap::ColorChoice::Auto if std::io::stderr().is_terminal() => color_choice,
            clap::ColorChoice::Always => ColorChoice::Always,
            _ => ColorChoice::Never,
        };

        let stream = termcolor::StandardStream::stderr(color_choice);
        TermOut {
            inner: Arc::new(TermOutInner {
                active: AtomicBool::new(true),
                stream,
                lines_written: AtomicUsize::new(0),
                in_alternate_screen: AtomicBool::new(false),
            }),
        }
    }

    /// Initialize a handler that listens for Ctrl-C signals.
    /// This is used to exit the alternate screen that might have been opened.
    pub fn init_exit_handler(&mut self) {
        // We can safely ignore the error as the only thing this handler would do
        // is leave an alternate screen if none was opened; not very important.
        let mut term_out = self.clone();
        let res = ctrlc::set_handler(move || {
            let _ = term_out.leave_alternate_screen();
            term_out.inner.active.store(false, Ordering::Release);
        });
        if let Err(err) = res {
            let _ = write!(self, "failed to initialize exit handler ({err})");
        }
    }

    /// Whether this program is still active and was not stopped by the Ctrl-C handler.
    pub fn active(&self) -> bool {
        self.inner.active.load(Ordering::Acquire)
    }

    /// Clears everything printed so far.
    pub fn clear(&mut self) -> io::Result<()> {
        let lines = self.inner.lines_written.load(Ordering::Acquire);
        self.clear_lines(lines)?;
        Ok(())
    }

    /// Clears a given number of lines.
    pub fn clear_lines(&mut self, lines: usize) -> io::Result<()> {
        // We don't want to clear anything that is not a TTY.
        if lines != 0 && self.inner.stream.supports_color() {
            // First, move the cursor up `lines` lines.
            // Then, clear everything between between the cursor to end of screen.
            let mut stream = self.inner.stream.lock();
            write!(stream, "\x1B[{lines}F\x1B[0J")?;
            stream.flush()?;
            self.inner.lines_written.fetch_sub(lines, Ordering::Release);
        }
        Ok(())
    }

    /// Enters the alternate screen if none was opened already.
    pub fn enter_alternate_screen(&mut self) -> io::Result<()> {
        if !self.inner.in_alternate_screen.load(Ordering::Acquire) {
            let mut stream = self.inner.stream.lock();
            write!(stream, "\x1B[?1049h")?;
            stream.flush()?;
            self.inner.in_alternate_screen.store(true, Ordering::Release);
        }
        Ok(())
    }

    /// Leaves the alternate screen if it is already open.
    pub fn leave_alternate_screen(&mut self) -> io::Result<()> {
        if self.inner.in_alternate_screen.load(Ordering::Acquire) {
            write!(self.inner.stream.lock(), "\x1B[?1049l")?;
            self.inner.stream.lock().flush()?;
            self.inner.in_alternate_screen.store(false, Ordering::Release);
        }
        Ok(())
    }
}

impl Write for TermOut {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let n = self.inner.stream.lock().write(buf)?;
        // Determine the number of lines just written.
        let lines = buf[..n].iter().filter(|&&b| b == b'\n').count();
        self.inner.lines_written.fetch_add(lines, Ordering::Release);
        Ok(n)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.stream.lock().flush()
    }
}

impl WriteColor for TermOut {
    fn supports_color(&self) -> bool {
        self.inner.stream.supports_color()
    }

    fn set_color(&mut self, spec: &termcolor::ColorSpec) -> io::Result<()> {
        self.inner.stream.lock().set_color(spec)
    }

    fn reset(&mut self) -> io::Result<()> {
        self.inner.stream.lock().reset()
    }
}

/// Print an application-level error (independent from a source file).
pub fn print_error(output: &mut TermOut, msg: &str) -> io::Result<()> {
    let styles = term::Styles::default();

    output.set_color(&styles.header_error)?;
    write!(output, "error")?;

    output.reset()?;
    writeln!(output, ": {msg}.")
}
