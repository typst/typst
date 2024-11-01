use std::io::{self, IsTerminal, Write};

use codespan_reporting::term::termcolor;
use termcolor::{ColorChoice, WriteColor};
use typst::utils::singleton;

use crate::ARGS;

/// Returns a handle to the optionally colored terminal output.
pub fn out() -> TermOut {
    TermOut {
        inner: singleton!(TermOutInner, TermOutInner::new()),
    }
}

/// The stuff that has to be shared between instances of [`TermOut`].
struct TermOutInner {
    stream: termcolor::StandardStream,
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
        TermOutInner { stream }
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
            // Then, clear everything between the cursor to end of screen.
            let mut stream = self.inner.stream.lock();
            write!(stream, "\x1B[1F\x1B[0J")?;
            stream.flush()?;
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
