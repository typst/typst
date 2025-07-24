mod args;
mod compile;
mod completions;
mod download;
mod fonts;
mod greet;
mod init;
mod package;
mod query;
#[cfg(feature = "http-server")]
mod server;
mod terminal;
mod timings;
#[cfg(feature = "self-update")]
mod update;
mod watch;
mod world;

use std::cell::Cell;
use std::io::{self, Write};
use std::process::ExitCode;
use std::sync::LazyLock;

use clap::Parser;
use clap::error::ErrorKind;
use codespan_reporting::term;
use codespan_reporting::term::termcolor::WriteColor;
use typst::diag::HintedStrResult;

use crate::args::{CliArguments, Command};
use crate::timings::Timer;

thread_local! {
    /// The CLI's exit code.
    static EXIT: Cell<ExitCode> = const { Cell::new(ExitCode::SUCCESS) };
}

/// The parsed command line arguments.
static ARGS: LazyLock<CliArguments> = LazyLock::new(|| {
    CliArguments::try_parse().unwrap_or_else(|error| {
        if error.kind() == ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand {
            crate::greet::greet();
        }
        error.exit();
    })
});

/// Entry point.
fn main() -> ExitCode {
    let mut prescient = false;
    if let Ok(file) = std::fs::File::open("comemo-sink") {
        let mmap = Box::leak(Box::new(unsafe { memmap2::Mmap::map(&file).unwrap() }));
        comemo::put_prescience(&*mmap);
        prescient = true;
    }

    // Handle SIGPIPE
    // https://stackoverflow.com/questions/65755853/simple-word-count-rust-program-outputs-valid-stdout-but-panicks-when-piped-to-he/65760807
    sigpipe::reset();

    let res = dispatch();

    if let Err(msg) = res {
        set_failed();
        print_error(msg.message()).expect("failed to print error");
    }

    if !prescient {
        let file = std::fs::File::create("comemo-sink").unwrap();
        let sink = std::io::BufWriter::new(file);
        comemo::write_prescience(sink);
    }

    EXIT.with(|cell| cell.get())
}

/// Execute the requested command.
fn dispatch() -> HintedStrResult<()> {
    let mut timer = Timer::new(&ARGS);

    match &ARGS.command {
        Command::Compile(command) => crate::compile::compile(&mut timer, command)?,
        Command::Watch(command) => crate::watch::watch(&mut timer, command)?,
        Command::Init(command) => crate::init::init(command)?,
        Command::Query(command) => crate::query::query(command)?,
        Command::Fonts(command) => crate::fonts::fonts(command),
        Command::Update(command) => crate::update::update(command)?,
        Command::Completions(command) => crate::completions::completions(command),
    }

    Ok(())
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
    use typst::diag::{StrResult, bail};

    use crate::args::UpdateCommand;

    pub fn update(_: &UpdateCommand) -> StrResult<()> {
        bail!(
            "self-updating is not enabled for this executable, \
             please update with the package manager or mechanism \
             used for initial installation"
        )
    }
}
