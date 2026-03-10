use std::io::{self, Write};

use codespan_reporting::term::termcolor::WriteColor;
use codespan_reporting::term::{self, termcolor};
use ecow::eco_format;
use typst::diag::{HintedStrResult, StrResult, bail, warning};
use typst::syntax::Span;
use typst::utils::format_duration;
use typst_kit::watcher::Watcher;

use crate::args::{Input, Output, WatchCommand};
use crate::compile::{CompileConfig, compile_once, print_diagnostics};
use crate::timings::Timer;
use crate::world::{SystemWorld, WorldCreationError};
use crate::{print_error, terminal};

/// Execute a watching compilation command.
pub fn watch(timer: &mut Timer, command: &'static WatchCommand) -> HintedStrResult<()> {
    let mut config = CompileConfig::watching(command)?;

    let Output::Path(output) = &config.output else {
        bail!("cannot write document to stdout in watch mode");
    };

    // Create a file system watcher.
    let mut watcher = Watcher::new(Some(output.clone()))?;

    // Create the world that serves sources, files, and fonts.
    // Additionally, if any files do not exist, wait until they do.
    let mut world = loop {
        match SystemWorld::new(
            Some(&command.args.input),
            &command.args.world,
            &command.args.process,
        ) {
            Ok(world) => break world,
            Err(
                ref err @ (WorldCreationError::InputNotFound(ref path)
                | WorldCreationError::RootNotFound(ref path)),
            ) => {
                watcher.update([path.clone()])?;
                Status::Error.print(&config).unwrap();
                print_error(&err.to_string()).unwrap();
                watcher.wait()?;
            }
            Err(err) => return Err(err.into()),
        }
    };

    // Eagerly scan fonts if we expect to need them so that it's not counted as
    // part of the displayed compilation time. The duration of font scanning is
    // heavily system-dependant, so it could result in confusion why compilation
    // is so much faster/slower.
    if config.output_format.is_paged() {
        world.scan_fonts();
    }

    // Perform initial compilation.
    timer.record(&mut world, |world| compile_once(world, &mut config))??;

    // Print warning when trying to watch stdin.
    if matches!(&config.input, Input::Stdin) {
        warn_watching_std(&world, &config)?;
    }

    // Recompile whenever something relevant happens.
    loop {
        // Watch all dependencies of the most recent compilation.
        watcher.update(world.dependencies())?;

        // Wait until anything relevant happens.
        watcher.wait()?;

        // Reset all dependencies.
        world.reset();

        // Recompile.
        timer.record(&mut world, |world| compile_once(world, &mut config))??;

        // Evict the cache.
        comemo::evict(10);
    }
}

/// The status in which the watcher can be.
pub enum Status {
    Compiling,
    Success(std::time::Duration),
    PartialSuccess(std::time::Duration),
    Error,
}

impl Status {
    /// Clear the terminal and render the status message.
    pub fn print(&self, config: &CompileConfig) -> io::Result<()> {
        let timestamp = chrono::offset::Local::now().format("%H:%M:%S");
        let color = self.color();

        let mut out = terminal::out();
        out.clear_screen()?;

        out.set_color(&color)?;
        write!(out, "watching")?;
        out.reset()?;
        match &config.input {
            Input::Stdin => writeln!(out, " <stdin>"),
            Input::Path(path) => writeln!(out, " {}", path.display()),
        }?;

        out.set_color(&color)?;
        write!(out, "writing to")?;
        out.reset()?;
        writeln!(out, " {}", config.output)?;

        #[cfg(feature = "http-server")]
        if let Some(server) = &config.server {
            out.set_color(&color)?;
            write!(out, "serving at")?;
            out.reset()?;
            writeln!(out, " http://{}", server.addr())?;
        }

        writeln!(out)?;
        writeln!(out, "[{timestamp}] {}", self.message())?;
        writeln!(out)?;

        out.flush()
    }

    fn message(&self) -> String {
        match *self {
            Self::Compiling => "compiling ...".into(),
            Self::Success(duration) => {
                format!("compiled successfully in {}", format_duration(duration))
            }
            Self::PartialSuccess(duration) => {
                format!("compiled with warnings in {}", format_duration(duration))
            }
            Self::Error => "compiled with errors".into(),
        }
    }

    fn color(&self) -> termcolor::ColorSpec {
        let styles = term::Styles::default();
        match self {
            Self::Error => styles.header_error,
            Self::PartialSuccess(_) => styles.header_warning,
            _ => styles.header_note,
        }
    }
}

/// Emits a warning when trying to watch stdin.
fn warn_watching_std(world: &SystemWorld, config: &CompileConfig) -> StrResult<()> {
    let warning = warning!(
        Span::detached(),
        "cannot watch changes for stdin";
        hint: "to recompile on changes, watch a regular file instead";
        hint: "to compile once and exit, please use `typst compile` instead";
    );
    print_diagnostics(world, &[], &[warning], config.diagnostic_format)
        .map_err(|err| eco_format!("failed to print diagnostics ({err})"))
}
