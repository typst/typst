use std::process::ExitCode;

use typst::diag::HintedStrResult;

use typst_cli::{ARGS, EXIT, print_error, set_failed};
use typst_cli::args::Command;
use typst_cli::timings::Timer;

/// Entry point.
fn main() -> ExitCode {
    let res = dispatch();

    if let Err(msg) = res {
        set_failed();
        print_error(msg.message()).expect("failed to print error");
    }

    EXIT.with(|cell| cell.get())
}

/// Execute the requested command.
fn dispatch() -> HintedStrResult<()> {
    let timer = Timer::new(&ARGS);

    match &ARGS.command {
        Command::Compile(command) => typst_cli::compile::compile(timer, command.clone())?,
        Command::Watch(command) => typst_cli::watch::watch(timer, command.clone())?,
        Command::Init(command) => typst_cli::init::init(command)?,
        Command::Query(command) => typst_cli::query::query(command)?,
        Command::Fonts(command) => typst_cli::fonts::fonts(command)?,
        Command::Update(command) => typst_cli::update::update(command)?,
    }

    Ok(())
}
