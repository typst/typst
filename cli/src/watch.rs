use std::collections::HashSet;
use std::io::{self, IsTerminal, Write};
use std::path::Path;

use codespan_reporting::term::{self, termcolor};
use notify::{RecommendedWatcher, Watcher};
use same_file::is_same_file;
use termcolor::WriteColor;
use typst::diag::StrResult;

use crate::args::CompileCommand;
use crate::color_stream;
use crate::compile::compile_once;
use crate::world::SystemWorld;

/// Execute a watching compilation command.
pub fn watch(mut command: CompileCommand) -> StrResult<()> {
    // Create the world that serves sources, files, and fonts.
    let mut world = SystemWorld::new(&command)?;

    // Perform initial compilation.
    compile_once(&mut world, &mut command, true)?;

    // Setup file watching.
    let (tx, rx) = std::sync::mpsc::channel();
    let mut watcher = RecommendedWatcher::new(tx, notify::Config::default())
        .map_err(|_| "failed to setup file watching")?;

    // Watch all the files that are used by the input file and its dependencies.
    world.watch(&mut watcher, HashSet::new())?;

    // Handle events.
    let timeout = std::time::Duration::from_millis(100);
    let output = command.output();
    loop {
        let mut recompile = false;
        for event in rx
            .recv()
            .into_iter()
            .chain(std::iter::from_fn(|| rx.recv_timeout(timeout).ok()))
        {
            let event = event.map_err(|_| "failed to watch directory")?;
            recompile |= is_event_relevant(&event, &output);
        }

        if recompile {
            // Retrieve the dependencies of the last compilation.
            let dependencies = world.dependencies();

            // Recompile.
            compile_once(&mut world, &mut command, true)?;
            comemo::evict(10);

            // Adjust the watching.
            world.watch(&mut watcher, dependencies)?;
        }
    }
}

/// Whether a watch event is relevant for compilation.
fn is_event_relevant(event: &notify::Event, output: &Path) -> bool {
    // Never recompile because the output file changed.
    if event
        .paths
        .iter()
        .all(|path| is_same_file(path, output).unwrap_or(false))
    {
        return false;
    }

    match &event.kind {
        notify::EventKind::Any => true,
        notify::EventKind::Access(_) => false,
        notify::EventKind::Create(_) => true,
        notify::EventKind::Modify(kind) => match kind {
            notify::event::ModifyKind::Any => true,
            notify::event::ModifyKind::Data(_) => true,
            notify::event::ModifyKind::Metadata(_) => false,
            notify::event::ModifyKind::Name(_) => true,
            notify::event::ModifyKind::Other => false,
        },
        notify::EventKind::Remove(_) => true,
        notify::EventKind::Other => false,
    }
}

/// The status in which the watcher can be.
pub enum Status {
    Compiling,
    Success(std::time::Duration),
    Error,
}

impl Status {
    /// Clear the terminal and render the status message.
    pub fn print(&self, command: &CompileCommand) -> io::Result<()> {
        let output = command.output();
        let timestamp = chrono::offset::Local::now().format("%H:%M:%S");
        let color = self.color();

        let mut w = color_stream();
        if std::io::stderr().is_terminal() {
            // Clear the terminal.
            let esc = 27 as char;
            write!(w, "{esc}c{esc}[1;1H")?;
        }

        w.set_color(&color)?;
        write!(w, "watching")?;
        w.reset()?;
        writeln!(w, " {}", command.input.display())?;

        w.set_color(&color)?;
        write!(w, "writing to")?;
        w.reset()?;
        writeln!(w, " {}", output.display())?;

        writeln!(w)?;
        writeln!(w, "[{timestamp}] {}", self.message())?;
        writeln!(w)?;

        w.flush()
    }

    fn message(&self) -> String {
        match self {
            Self::Compiling => "compiling ...".into(),
            Self::Success(duration) => format!("compiled successfully in {duration:.2?}"),
            Self::Error => "compiled with errors".into(),
        }
    }

    fn color(&self) -> termcolor::ColorSpec {
        let styles = term::Styles::default();
        match self {
            Self::Error => styles.header_error,
            _ => styles.header_note,
        }
    }
}
