mod args;
mod compile;
mod download;
mod fonts;
mod package;
mod query;
mod terminal;
mod timings;
#[cfg(feature = "self-update")]
mod update;
mod watch;
mod world;

use std::cell::Cell;
use std::io::{self, Write};
use std::process::ExitCode;

use clap::Parser;
use codespan_reporting::term;
use codespan_reporting::term::termcolor::WriteColor;
use ecow::eco_format;
use once_cell::sync::Lazy;

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

    let res = match &ARGS.command {
        Command::Compile(command) => crate::compile::compile(timer, command.clone()),
        Command::Watch(command) => crate::watch::watch(timer, command.clone()),
        Command::Query(command) => crate::query::query(command),
        Command::Fonts(command) => crate::fonts::fonts(command),
        Command::Update(command) => crate::update::update(command),
    };

    // Leave the alternate screen if it was opened. This operation is done here
    // so that it is executed prior to printing the final error.
    let res_leave = terminal::out()
        .leave_alternate_screen()
        .map_err(|err| eco_format!("failed to leave alternate screen ({err})"));

    if let Some(msg) = res.err().or(res_leave.err()) {
        set_failed();
        print_error(&msg).expect("failed to print error");
    }

    EXIT.with(|cell| cell.get())
}

/// Ensure a failure exit code.
fn set_failed() {
    EXIT.with(|cell| cell.set(ExitCode::FAILURE));
}

/// Used by `args.rs`.
fn typst_version() -> &'static str {
    env!("TYPST_VERSION")
}

/// Print an application-level error (independent from a source file).
fn print_error(msg: &str) -> io::Result<()> {
    let styles = term::Styles::default();

    let mut output = terminal::out();
    output.set_color(&styles.header_error)?;
    write!(output, "error")?;

    output.reset()?;
    writeln!(output, ": {msg}.")
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
