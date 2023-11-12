mod args;
mod compile;
mod download;
mod fonts;
mod package;
mod query;
mod timings;
#[cfg(feature = "self-update")]
mod update;
mod watch;
mod world;

use std::cell::{Cell, RefCell};
use std::io::{self, IsTerminal, Write};
use std::process::ExitCode;
use std::rc::Rc;

use args::DiagnosticFormat;
use clap::Parser;
use codespan_reporting::term::{self, termcolor};
use once_cell::sync::Lazy;
use termcolor::{ColorChoice, WriteColor};

use crate::args::{CliArguments, Command};
use crate::timings::Timer;

thread_local! {
    /// The CLI's exit code.
    static EXIT: Cell<ExitCode> = Cell::new(ExitCode::SUCCESS);
}

/// The parsed commandline arguments.
static ARGS: Lazy<CliArguments> = Lazy::new(CliArguments::parse);

/// Entry point.
fn main() -> ExitCode {
    let timer = Timer::new(&ARGS);
    let mut term_out = TermOut::new(&ARGS.command);

    let res = match &ARGS.command {
        Command::Compile(command) => {
            crate::compile::compile(&mut term_out, timer, command.clone())
        }
        Command::Watch(command) => {
            crate::watch::watch(&mut term_out, timer, command.clone())
        }
        Command::Query(command) => crate::query::query(command),
        Command::Fonts(command) => crate::fonts::fonts(command),
        Command::Update(command) => crate::update::update(command),
    };

    if let Err(msg) = res {
        set_failed();
        print_error(&mut term_out, &msg).expect("failed to print error");
    }

    EXIT.with(|cell| cell.get())
}

/// Ensure a failure exit code.
fn set_failed() {
    EXIT.with(|cell| cell.set(ExitCode::FAILURE));
}

/// A utility that allows users to write colored terminal output.
/// If colors are not supported by the terminal, they are disabled.
/// This type also allows for deletion of previously written lines.
#[derive(Clone)]
pub struct TermOut {
    inner: Rc<RefCell<TermOutInner>>,
}

/// The stuff that has to be shared between instances of [`TermOut`].
///
/// Sharing is necessary because the [`world::SystemWorld`] writes to the terminal,
/// while other functions do as well.
/// Write access behind a shared reference is needed due to the design
/// of the [`typst::World`] trait, whose methods take `&self`.
struct TermOutInner {
    stream: termcolor::StandardStream,
    lines_written: usize,
}

impl TermOut {
    fn new(command: &Command) -> Self {
        let color_choice = match ARGS.color {
            clap::ColorChoice::Auto if std::io::stderr().is_terminal() => color_choice,
            clap::ColorChoice::Always => ColorChoice::Always,
            _ => ColorChoice::Never,
        };

        let stream = termcolor::StandardStream::stderr(color_choice);
        TermOut {
            inner: Rc::new(RefCell::new(TermOutInner { stream, lines_written: 0 })),
        }
    }

    /// Clears everything printed so far.
    pub fn clear(&mut self) -> io::Result<()> {
        let lines = self.inner.borrow().lines_written;
        self.clear_lines(lines)?;
        Ok(())
    }

    /// Clears a given number of lines.
    pub fn clear_lines(&mut self, lines: usize) -> io::Result<()> {
        let mut inner = self.inner.borrow_mut();
        // We don't want to clear anything that is not a TTY.
        if lines != 0 && inner.stream.supports_color() {
            // First, move the cursor up `lines` lines.
            // Then, clear everything between between the cursor to end of screen.
            write!(inner.stream, "\x1B[{lines}F\x1B[0J")?;
            inner.stream.flush()?;
            inner.lines_written -= lines;
        }
        Ok(())
    }
}

impl Write for TermOut {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let mut inner = self.inner.borrow_mut();
        let n = inner.stream.write(buf)?;
        // Determine the number of lines just written.
        inner.lines_written += buf[..n].iter().filter(|&&b| b == b'\n').count();
        Ok(n)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.borrow_mut().stream.flush()
    }
}

impl WriteColor for TermOut {
    fn supports_color(&self) -> bool {
        self.inner.borrow_mut().stream.supports_color()
    }

    fn set_color(&mut self, spec: &termcolor::ColorSpec) -> io::Result<()> {
        self.inner.borrow_mut().stream.set_color(spec)
    }

    fn reset(&mut self) -> io::Result<()> {
        self.inner.borrow_mut().stream.reset()
    }
}

/// Print an application-level error (independent from a source file).
fn print_error(output: &mut TermOut, msg: &str) -> io::Result<()> {
    let styles = term::Styles::default();

    output.set_color(&styles.header_error)?;
    write!(output, "error")?;

    output.reset()?;
    writeln!(output, ": {msg}.")
}

/// Used by `args.rs`.
fn typst_version() -> &'static str {
    env!("TYPST_VERSION")
}

#[cfg(not(feature = "self-update"))]
mod update {
    use crate::args::UpdateCommand;
    use typst::diag::{bail, StrResult};

    pub fn update(_: &UpdateCommand) -> StrResult<()> {
        bail!(
            "self-updating is not enabled for this executable, \
             please update with the package manager or mechanism \
             used for initial installation"
        )
    }
}
