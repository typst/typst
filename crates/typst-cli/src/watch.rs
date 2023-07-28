use std::collections::HashSet;
use std::io::{self, IsTerminal, Write};
use std::path::{Path, PathBuf};

use codespan_reporting::term::{self, termcolor};
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use same_file::is_same_file;
use termcolor::WriteColor;
use typst::diag::StrResult;
use typst::eval::eco_format;

use crate::args::CompileCommand;
use crate::color_stream;
use crate::compile::compile_once;
use crate::world::SystemWorld;

/// Execute a watching compilation command.
pub fn watch(mut command: CompileCommand) -> StrResult<()> {
    // Create the world that serves sources, files, and fonts.
    let mut world = SystemWorld::new(&command.common)?;

    // Perform initial compilation.
    compile_once(&mut world, &mut command, true)?;

    // Setup file watching.
    let (tx, rx) = std::sync::mpsc::channel();
    let mut watcher = RecommendedWatcher::new(tx, notify::Config::default())
        .map_err(|_| "failed to setup file watching")?;

    // Watch all the files that are used by the input file and its dependencies.
    watch_dependencies(&mut world, &mut watcher, HashSet::new())?;

    // Handle events.
    let timeout = std::time::Duration::from_millis(100);
    let output = command.output();
    loop {
        let mut removed = HashSet::new();
        let mut recompile = false;
        for event in rx
            .recv()
            .into_iter()
            .chain(std::iter::from_fn(|| rx.recv_timeout(timeout).ok()))
        {
            let event = event.map_err(|_| "failed to watch directory")?;

            // Workaround for notify-rs' implicit unwatch on remove/rename
            // (triggered by some editors when saving files) with the inotify
            // backend. By keeping track of the removed files, we can allow
            // those we still depend on to be watched again later on.
            if matches!(
                event.kind,
                notify::EventKind::Remove(notify::event::RemoveKind::File)
            ) {
                let path = &event.paths[0];
                removed.insert(path.clone());

                // Remove the watch in case it still exists.
                watcher.unwatch(path).ok();
            }

            recompile |= is_event_relevant(&event, &output);
        }

        if recompile {
            // Retrieve the dependencies of the last compilation.
            let previous: HashSet<PathBuf> = world
                .dependencies()
                .filter(|path| !removed.contains(*path))
                .map(ToOwned::to_owned)
                .collect();

            // Recompile.
            compile_once(&mut world, &mut command, true)?;
            comemo::evict(10);

            // Adjust the watching.
            watch_dependencies(&mut world, &mut watcher, previous)?;
        }
    }
}

/// Adjust the file watching. Watches all new dependencies and unwatches
/// all `previous` dependencies that are not relevant anymore.
#[tracing::instrument(skip_all)]
fn watch_dependencies(
    world: &mut SystemWorld,
    watcher: &mut dyn Watcher,
    mut previous: HashSet<PathBuf>,
) -> StrResult<()> {
    // Watch new paths that weren't watched yet.
    for path in world.dependencies() {
        let watched = previous.remove(path);
        if path.exists() && !watched {
            tracing::info!("Watching {}", path.display());
            watcher
                .watch(path, RecursiveMode::NonRecursive)
                .map_err(|_| eco_format!("failed to watch {path:?}"))?;
        }
    }

    // Unwatch old paths that don't need to be watched anymore.
    for path in previous {
        tracing::info!("Unwatching {}", path.display());
        watcher.unwatch(&path).ok();
    }

    Ok(())
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
    PartialSuccess(std::time::Duration),
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
        writeln!(w, " {}", command.common.input.display())?;

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
            Self::PartialSuccess(duration) => {
                format!("compiled with warnings in {duration:.2?}")
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
