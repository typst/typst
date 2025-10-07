mod args;
mod compile;
mod completions;
mod deps;
mod download;
mod fonts;
mod greet;
mod info;
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
use ecow::eco_format;
use serde::Serialize;
use typst::diag::{HintedStrResult, StrResult};

use crate::args::{CliArguments, Command, SerializationFormat};
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
    // Handle SIGPIPE
    // https://stackoverflow.com/questions/65755853/simple-word-count-rust-program-outputs-valid-stdout-but-panicks-when-piped-to-he/65760807
    sigpipe::reset();

    let res = dispatch();

    if let Err(msg) = res {
        set_failed();
        print_error(msg.message()).expect("failed to print error");
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
        Command::Info(command) => crate::info::info(command)?,
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

/// Used by `args.rs`.
fn typst_commit_sha() -> &'static str {
    env!("TYPST_COMMIT_SHA")
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

/// Serialize data to the output format and convert the error to an
/// [`EcoString`].
fn serialize(
    data: &impl Serialize,
    format: SerializationFormat,
    pretty: bool,
) -> StrResult<String> {
    match format {
        SerializationFormat::Json => {
            if pretty {
                serde_json::to_string_pretty(data).map_err(|e| eco_format!("{e}"))
            } else {
                serde_json::to_string(data).map_err(|e| eco_format!("{e}"))
            }
        }
        SerializationFormat::Yaml => {
            serde_yaml::to_string(data).map_err(|e| eco_format!("{e}"))
        }
    }
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
