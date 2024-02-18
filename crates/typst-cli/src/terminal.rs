use std::io::{self, IsTerminal, Write};
use std::sync::atomic::{AtomicBool, Ordering};

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
        // We can safely ignore the error as the only thing this handler would do
        // is leave an alternate screen if none was opened; not very important.
        let mut term_out = self.clone();
        ctrlc::set_handler(move || {
            let _ = term_out.leave_alternate_screen();

            // Exit with the exit code standard for Ctrl-C exits[^1].
            // There doesn't seem to be another standard exit code for Windows,
            // so we just use the same one there.
            // [^1]: https://tldp.org/LDP/abs/html/exitcodes.html
            std::process::exit(128 + 2);
        })
        .map_err(|err| eco_format!("failed to initialize exit handler ({err})"))
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
