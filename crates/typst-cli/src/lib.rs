pub mod args;
pub mod compile;
pub mod download;
pub mod fonts;
pub mod init;
pub mod package;
pub mod query;
pub mod terminal;
pub mod timings;
#[cfg(feature = "self-update")]
pub mod update;
pub mod watch;
pub mod world;

use std::cell::Cell;
use std::io::{self, Write};
use std::process::ExitCode;

use clap::Parser;
use codespan_reporting::term;
use codespan_reporting::term::termcolor::WriteColor;
use once_cell::sync::Lazy;

use crate::args::CliArguments;

/// Ensure a failure exit code.
pub fn set_failed() {
    EXIT.with(|cell| cell.set(ExitCode::FAILURE));
}


thread_local! {
    /// The CLI's exit code.
    pub static EXIT: Cell<ExitCode> = const { Cell::new(ExitCode::SUCCESS) };
}

/// The parsed commandline arguments.
pub static ARGS: Lazy<CliArguments> = Lazy::new(CliArguments::parse);

/// Used by `args.rs`.
pub fn typst_version() -> &'static str {
    env!("TYPST_VERSION")
}

/// Print an application-level error (independent from a source file).
pub fn print_error(msg: &str) -> io::Result<()> {
    let styles = term::Styles::default();

    let mut output = terminal::out();
    output.set_color(&styles.header_error)?;
    write!(output, "error")?;

    output.reset()?;
    writeln!(output, ": {msg}")
}

#[cfg(not(feature = "self-update"))]
pub mod update {
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
