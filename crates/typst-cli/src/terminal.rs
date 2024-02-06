use std::io::{self, IsTerminal, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use codespan_reporting::term::termcolor;
use ecow::eco_format;
use once_cell::sync::Lazy;
use termcolor::{ColorChoice, WriteColor};
use typst::diag::StrResult;

use crate::ARGS;

/// Returns a handle to the optionally colored terminal output.
pub fn out() -> TermOut {
    static OUTPUT: Lazy<TermOutInner> = Lazy::new(TermOutInner::new);
    TermOut { inner: &OUTPUT }
}

/// The stuff that has to be shared between instances of [`TermOut`].
struct TermOutInner {
    active: AtomicBool,
    stream: termcolor::StandardStream,
    in_alternate_screen: AtomicBool,
}

impl TermOutInner {
    fn new() -> Self {
        let color_choice = match ARGS.color {
            clap::ColorChoice::Auto if std::io::stderr().is_terminal() => {
                ColorChoice::Auto
            }
            clap::ColorChoice::Always => ColorChoice::Always,
            _ => ColorChoice::Never,
        };

        let stream = termcolor::StandardStream::stderr(color_choice);
        TermOutInner {
            active: AtomicBool::new(true),
            stream,
            in_alternate_screen: AtomicBool::new(false),
        }
    }
}

/// A utility that allows users to write colored terminal output.
/// If colors are not supported by the terminal, they are disabled.
/// This type also allows for deletion of previously written lines.
#[derive(Clone)]
pub struct TermOut {
    inner: &'static TermOutInner,
}

impl TermOut {
    /// Initialize a handler that listens for Ctrl-C signals.
    /// This is used to exit the alternate screen that might have been opened.
    pub fn init_exit_handler(&mut self) -> StrResult<()> {
        /// The duration the application may keep running after an exit signal was received.
        const MAX_TIME_TO_EXIT: Duration = Duration::from_millis(750);

        // We can safely ignore the error as the only thing this handler would do
        // is leave an alternate screen if none was opened; not very important.
        let mut term_out = self.clone();
        ctrlc::set_handler(move || {
            term_out.inner.active.store(false, Ordering::Release);

            // Wait for some time and if the application is still running, simply exit.
            // Not exiting immediately potentially allows destructors to run and file writes
            // to complete.
            std::thread::sleep(MAX_TIME_TO_EXIT);

            // Leave alternate screen only after the timeout has expired.
            // This prevents console output intended only for within the alternate screen
            // from showing up outside it.
            // Remember that the alternate screen is also closed if the timeout is not reached,
            // just from a different location in code.
            let _ = term_out.leave_alternate_screen();

            // Exit with the exit code standard for Ctrl-C exits[^1].
            // There doesn't seem to be another standard exit code for Windows,
            // so we just use the same one there.
            // [^1]: https://tldp.org/LDP/abs/html/exitcodes.html
            std::process::exit(128 + 2);
        })
        .map_err(|err| eco_format!("failed to initialize exit handler ({err})"))
    }

    /// Whether this program is still active and was not stopped by the Ctrl-C handler.
    pub fn is_active(&self) -> bool {
        self.inner.active.load(Ordering::Acquire)
    }

    /// Clears the entire screen.
    pub fn clear_screen(&mut self) -> io::Result<()> {
        // We don't want to clear anything that is not a TTY.
        if self.inner.stream.supports_color() {
            let mut stream = self.inner.stream.lock();
            // Clear the screen and then move the cursor to the top left corner.
            write!(stream, "\x1B[2J\x1B[1;1H")?;
            stream.flush()?;
        }
        Ok(())
    }

    /// Clears the previously written line.
    pub fn clear_last_line(&mut self) -> io::Result<()> {
        // We don't want to clear anything that is not a TTY.
        if self.inner.stream.supports_color() {
            // First, move the cursor up `lines` lines.
            // Then, clear everything between between the cursor to end of screen.
            let mut stream = self.inner.stream.lock();
            write!(stream, "\x1B[1F\x1B[0J")?;
            stream.flush()?;
        }
        Ok(())
    }

    /// Enters the alternate screen if none was opened already.
    pub fn enter_alternate_screen(&mut self) -> io::Result<()> {
        if self.inner.stream.supports_color()
            && !self.inner.in_alternate_screen.load(Ordering::Acquire)
        {
            let mut stream = self.inner.stream.lock();
            write!(stream, "\x1B[?1049h")?;
            stream.flush()?;
            self.inner.in_alternate_screen.store(true, Ordering::Release);
        }
        Ok(())
    }

    /// Leaves the alternate screen if it is already open.
    pub fn leave_alternate_screen(&mut self) -> io::Result<()> {
        if self.inner.stream.supports_color()
            && self.inner.in_alternate_screen.load(Ordering::Acquire)
        {
            let mut stream = self.inner.stream.lock();
            write!(stream, "\x1B[?1049l")?;
            stream.flush()?;
            self.inner.in_alternate_screen.store(false, Ordering::Release);
        }
        Ok(())
    }
}

impl Write for TermOut {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner.stream.lock().write(buf)
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
