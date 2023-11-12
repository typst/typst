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
use std::process::ExitCode;

use clap::Parser;
use ecow::eco_format;
use once_cell::sync::Lazy;
use terminal::TermOut;

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
    let mut term_out = TermOut::new();
    term_out.init_exit_handler();

    let mut res = match &ARGS.command {
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

    // Leave the alternate screen if it was opened. This operation occurs here
    // so that it is executed prior to printing the final error.
    res = res.or(term_out
        .leave_alternate_screen()
        .map_err(|err| eco_format!("failed to leave alternate screen ({err})")));

    if let Err(msg) = res {
        set_failed();
        terminal::print_error(&mut term_out, &msg).expect("failed to print error");
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
