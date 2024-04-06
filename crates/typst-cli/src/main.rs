mod args;
mod compile;
mod download;
mod fonts;
mod init;
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

use args::{Input, Output};
use clap::error::ErrorKind;
use clap::{CommandFactory, Parser};
use codespan_reporting::term;
use codespan_reporting::term::termcolor::WriteColor;
use once_cell::sync::Lazy;

use crate::args::{CliArguments, Command};
use crate::timings::Timer;

thread_local! {
    /// The CLI's exit code.
    static EXIT: Cell<ExitCode> = const { Cell::new(ExitCode::SUCCESS) };
}

/// The parsed commandline arguments.
static ARGS: Lazy<CliArguments> = Lazy::new(|| {
    let args = CliArguments::parse();

    // Validate the combination of input, output, and makefile_deps. There may
    // be a more elegant way to do this once the following issue is addressed:
    // https://github.com/clap-rs/clap/issues/3008. Don't change this without
    // ensuring it won't break the use of makefile_deps in compile.rs
    let (Command::Compile(ref command) | Command::Watch(ref command)) = args.command
    else {
        return args;
    };
    if command.makefile_deps.is_none()
        || matches!(
            (&command.common.input, &command.output),
            (Input::Path(_), Some(Output::Path(_)))
        )
    {
        return args;
    }
    CliArguments::command()
        .error(
            ErrorKind::ArgumentConflict,
            "use of --makefile-deps requires INPUT and OUTPUT paths",
        )
        .exit()
});

/// Entry point.
fn main() -> ExitCode {
    let timer = Timer::new(&ARGS);

    let res = match &ARGS.command {
        Command::Compile(command) => crate::compile::compile(timer, command.clone()),
        Command::Watch(command) => crate::watch::watch(timer, command.clone()),
        Command::Init(command) => crate::init::init(command),
        Command::Query(command) => crate::query::query(command),
        Command::Fonts(command) => crate::fonts::fonts(command),
        Command::Update(command) => crate::update::update(command),
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
    writeln!(output, ": {msg}")
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
