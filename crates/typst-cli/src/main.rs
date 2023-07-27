mod args;
mod compile;
mod fonts;
mod package;
mod query;
mod tracing;
mod watch;
mod world;

use std::cell::Cell;
use std::env;
use std::io::{self, IsTerminal, Write};
use std::process::ExitCode;

use clap::Parser;
use codespan_reporting::term::{self, termcolor};
use termcolor::{ColorChoice, WriteColor};

use crate::args::{CliArguments, Command};

thread_local! {
    /// The CLI's exit code.
    static EXIT: Cell<ExitCode> = Cell::new(ExitCode::SUCCESS);
}

/// Entry point.
fn main() -> ExitCode {
    let arguments = CliArguments::parse();
    let _guard = match crate::tracing::setup_tracing(&arguments) {
        Ok(guard) => guard,
        Err(err) => {
            eprintln!("failed to initialize tracing {}", err);
            None
        }
    };

    let res = match arguments.command {
        Command::Compile(command) => crate::compile::compile(command),
        Command::Watch(command) => crate::watch::watch(command),
        Command::Query(command) => crate::query::query(command),
        Command::Fonts(command) => crate::fonts::fonts(command),
    };

    if let Err(msg) = res {
        set_failed();
        print_error(&msg).expect("failed to print error");
    }

    EXIT.with(|cell| cell.get())
}

/// Ensure a failure exit code.
fn set_failed() {
    EXIT.with(|cell| cell.set(ExitCode::FAILURE));
}

/// Print an application-level error (independent from a source file).
fn print_error(msg: &str) -> io::Result<()> {
    let mut w = color_stream();
    let styles = term::Styles::default();

    w.set_color(&styles.header_error)?;
    write!(w, "error")?;

    w.reset()?;
    writeln!(w, ": {msg}.")
}

/// Get stderr with color support if desirable.
fn color_stream() -> termcolor::StandardStream {
    termcolor::StandardStream::stderr(if std::io::stderr().is_terminal() {
        ColorChoice::Auto
    } else {
        ColorChoice::Never
    })
}

/// Used by `args.rs`.
fn typst_version() -> &'static str {
    env!("TYPST_VERSION")
}
