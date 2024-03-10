use std::io::{self, IsTerminal, Write};
use std::sync::OnceLock;

use codespan_reporting::term::termcolor;
use crossterm as ct;
use minus::Pager;
use parking_lot::Mutex;
use termcolor::{ColorChoice, WriteColor};

use crate::args::{Command, PaginationChoice};
use crate::ARGS;

static OUTPUT: OnceLock<TermOutInner> = OnceLock::new();

pub fn init() {
    let color_choice = match ARGS.color {
        clap::ColorChoice::Auto if std::io::stderr().is_terminal() => ColorChoice::Auto,
        clap::ColorChoice::Always => ColorChoice::Always,
        _ => ColorChoice::Never,
    };

    let use_pager = match ARGS.pager {
        PaginationChoice::Auto => {
            color_choice != ColorChoice::Never
                && matches!(ARGS.command, Command::Watch(_))
                && std::io::stdout().is_terminal()
        }
        PaginationChoice::Never => false,
        PaginationChoice::Always => true,
    };

    let output = if use_pager {
        let pager = Pager::new();
        std::thread::spawn({
            let pager = pager.clone();
            move || minus::dynamic_paging(pager).expect("tried to initialize pager")
        });
        Output::Pager { buffer: Mutex::new(String::new()), pager }
    } else {
        Output::Direct(termcolor::StandardStream::stderr(color_choice))
    };
    // We can safely ignore the returned result because the set value is always the same.
    let _ = OUTPUT.set(TermOutInner { output, color_choice });
}

/// Returns a handle to the optionally colored terminal output.
pub fn out() -> TermOut {
    TermOut {
        inner: OUTPUT.get().expect("output was not initialized yet"),
    }
}

/// The stuff that has to be shared between instances of [`TermOut`].
struct TermOutInner {
    output: Output,
    color_choice: ColorChoice,
}

/// The type of output stream.
enum Output {
    Direct(termcolor::StandardStream),
    Pager { buffer: Mutex<String>, pager: Pager },
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
        // We only want to clear the screen inside the pager.
        if let Output::Pager { buffer, pager } = &self.inner.output {
            buffer.lock().clear();
            pager.set_text(String::new()).map_err(io::Error::other)?;
        }
        Ok(())
    }

    /// Clears the previously written line.
    pub fn clear_last_line(&mut self) -> io::Result<()> {
        match &self.inner.output {
            Output::Direct(out) => {
                // We don't want to clear anything that is not a TTY.
                if self.supports_color() {
                    ct::execute!(
                        out.lock(),
                        ct::cursor::MoveToPreviousLine(1),
                        ct::terminal::Clear(ct::terminal::ClearType::FromCursorDown),
                    )?;
                }
            }
            Output::Pager { buffer, pager } => {
                let mut buffer = buffer.lock();
                // Compute the index of the last newline.
                // By splitting off the last newline, we make sure that the previous line
                // is still deleted, even if there is a trailing newline.
                let newline_idx =
                    buffer[..buffer.len() - 1].rfind('\n').map_or(0, |x| x + 1);
                buffer.truncate(newline_idx);
                pager.set_text(buffer.clone()).map_err(io::Error::other)?;
            }
        }
        Ok(())
    }
}

impl io::Write for TermOut {
    /// Write a buffer into this output, returning the number of bytes written.
    ///
    /// # Panics
    /// If the input is not completely UTF-8 encoded.
    /// This is not ideal, but there is otherwise no way to support
    /// [`termcolor::WriteColor`] and [`minus::Pager`] at the same time.
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match &self.inner.output {
            Output::Direct(out) => out.lock().write(buf),
            Output::Pager { buffer, pager } => {
                let str_buf = std::str::from_utf8(buf)
                    .expect("we should never output non UTF-8 content");
                pager.push_str(str_buf).map_err(io::Error::other)?;
                buffer.lock().push_str(str_buf);
                Ok(str_buf.len())
            }
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match &self.inner.output {
            Output::Direct(out) => out.lock().flush(),
            Output::Pager { .. } => Ok(()),
        }
    }
}

impl WriteColor for TermOut {
    fn supports_color(&self) -> bool {
        match &self.inner.output {
            Output::Direct(out) => out.supports_color(),
            Output::Pager { .. } => self.inner.color_choice != ColorChoice::Never,
        }
    }

    fn set_color(&mut self, spec: &termcolor::ColorSpec) -> io::Result<()> {
        if self.supports_color() {
            let mut buf = termcolor::Buffer::ansi();
            buf.set_color(spec)?;
            self.write_all(buf.as_slice())?;
        }
        Ok(())
    }

    fn reset(&mut self) -> io::Result<()> {
        if self.supports_color() {
            let mut buf = termcolor::Buffer::ansi();
            buf.reset()?;
            self.write_all(buf.as_slice())?;
        }
        Ok(())
    }
}
