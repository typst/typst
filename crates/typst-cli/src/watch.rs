use std::collections::HashMap;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use codespan_reporting::term::termcolor::WriteColor;
use codespan_reporting::term::{self, termcolor};
use ecow::eco_format;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use same_file::is_same_file;
use typst::diag::StrResult;

use crate::args::{CompileCommand, Input};
use crate::compile::compile_once;
use crate::terminal;
use crate::timings::Timer;
use crate::world::SystemWorld;

/// Execute a watching compilation command.
pub fn watch(mut timer: Timer, mut command: CompileCommand) -> StrResult<()> {
    // Enter the alternate screen and handle Ctrl-C ourselves.
    terminal::out().init_exit_handler()?;
    terminal::out()
        .enter_alternate_screen()
        .map_err(|err| eco_format!("failed to enter alternate screen ({err})"))?;

    // Create the world that serves sources, files, and fonts.
    let mut world = SystemWorld::new(&command.common)?;

    // Perform initial compilation.
    timer.record(&mut world, |world| compile_once(world, &mut command, true))??;

    // Setup file watching.
    let (tx, rx) = std::sync::mpsc::channel();
    let mut watcher = RecommendedWatcher::new(tx, notify::Config::default())
        .map_err(|err| eco_format!("failed to setup file watching ({err})"))?;

    // Watch all the files that are used by the input file and its dependencies.
    let mut watched = HashMap::new();
    watch_dependencies(&mut world, &mut watcher, &mut watched)?;

    // Handle events.
    let timeout = std::time::Duration::from_millis(100);
    let output = command.output();
    while terminal::out().is_active() {
        let mut recompile = false;
        if let Ok(event) = rx.recv_timeout(timeout) {
            let event =
                event.map_err(|err| eco_format!("failed to watch directory ({err})"))?;

            // Workaround for notify-rs' implicit unwatch on remove/rename
            // (triggered by some editors when saving files) with the inotify
            // backend. By keeping track of the potentially unwatched files, we
            // can allow those we still depend on to be watched again later on.
            if matches!(
                event.kind,
                notify::EventKind::Remove(notify::event::RemoveKind::File)
            ) {
                // Mark the file as unwatched and remove the watch in case it
                // still exists.
                let path = &event.paths[0];
                watched.remove(path);
                watcher.unwatch(path).ok();
            }

            recompile |= is_event_relevant(&event, &output);
        }

        if recompile {
            // Reset all dependencies.
            world.reset();

            // Recompile.
            timer
                .record(&mut world, |world| compile_once(world, &mut command, true))??;

            comemo::evict(10);

            // Adjust the file watching.
            watch_dependencies(&mut world, &mut watcher, &mut watched)?;
        }
    }
    Ok(())
}

/// Adjust the file watching. Watches all new dependencies and unwatches
/// all previously `watched` files that are no relevant anymore.
fn watch_dependencies(
    world: &mut SystemWorld,
    watcher: &mut dyn Watcher,
    watched: &mut HashMap<PathBuf, bool>,
) -> StrResult<()> {
    // Mark all files as not "seen" so that we may unwatch them if they aren't
    // in the dependency list.
    for seen in watched.values_mut() {
        *seen = false;
    }

    // Retrieve the dependencies of the last compilation and watch new paths
    // that weren't watched yet. We can't watch paths that don't exist yet
    // unfortunately, so we filter those out.
    for path in world.dependencies().filter(|path| path.exists()) {
        if !watched.contains_key(&path) {
            watcher
                .watch(&path, RecursiveMode::NonRecursive)
                .map_err(|err| eco_format!("failed to watch {path:?} ({err})"))?;
        }

        // Mark the file as "seen" so that we don't unwatch it.
        watched.insert(path, true);
    }

    // Unwatch old paths that don't need to be watched anymore.
    watched.retain(|path, &mut seen| {
        if !seen {
            watcher.unwatch(path).ok();
        }
        seen
    });

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

        let mut term_out = terminal::out();
        term_out.clear_screen()?;

        term_out.set_color(&color)?;
        write!(term_out, "watching")?;
        term_out.reset()?;
        match &command.common.input {
            Input::Stdin => writeln!(term_out, " <stdin>"),
            Input::Path(path) => writeln!(term_out, " {}", path.display()),
        }?;

        term_out.set_color(&color)?;
        write!(term_out, "writing to")?;
        term_out.reset()?;
        writeln!(term_out, " {}", output.display())?;

        writeln!(term_out)?;
        writeln!(term_out, "[{timestamp}] {}", self.message())?;
        writeln!(term_out)?;

        term_out.flush()
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
