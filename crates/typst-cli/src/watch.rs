use std::collections::{HashMap, HashSet};
use std::io::{self, Write};
use std::iter;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

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

/// How long to wait for a shortly following file system event when watching.
const WATCH_TIMEOUT: Duration = Duration::from_millis(100);
/// How long file system events should be watched for before stopping
/// to compile the document.
const STARVE_DURATION: Duration = Duration::from_millis(500);

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

    // Set the poll interval to something more eager than the default.
    // That default seems a bit excessive for our purposes at around 30s.
    // Depending on feedback, some tuning might still be in order.
    // Note that this only affects a tiny number of systems.
    // Most do not use the [`notify::PollWatcher`].
    let watch_config =
        notify::Config::default().with_poll_interval(Duration::from_secs(4));
    let mut watcher = RecommendedWatcher::new(tx, watch_config)
        .map_err(|err| eco_format!("failed to setup file watching ({err})"))?;

    // Watch all the files that are used by the input file and its dependencies.
    let mut watched = HashMap::new();
    // Files that were removed but are still dependencies.
    let mut missing = HashSet::new();
    watch_dependencies(&mut world, &mut watcher, &mut watched, &mut missing)?;

    // Handle events.
    let output = command.output();
    while terminal::out().is_active() {
        let mut recompile = false;

        // Watch for file system events. If multiple events happen consecutively all within
        // a certain duration, then they are bunched up without a recompile in-between.
        // This helps against some editors' remove&move behavior.
        // Events are also only watched until a certain point, to hinder a barrage of events from
        // preventing recompilations.
        let recv_loop_start = Instant::now();
        for event in iter::from_fn(|| rx.recv_timeout(WATCH_TIMEOUT).ok())
            .take_while(|_| recv_loop_start.elapsed() <= STARVE_DURATION)
        {
            let event = event
                .map_err(|err| eco_format!("failed to watch dependencies ({err})"))?;

            // Workaround for notify-rs' implicit unwatch on remove/rename
            // (triggered by some editors when saving files) with the inotify
            // backend. By keeping track of the potentially unwatched files, we
            // can allow those we still depend on to be watched again later on.
            if matches!(
                event.kind,
                notify::EventKind::Remove(notify::event::RemoveKind::File)
                    | notify::EventKind::Modify(notify::event::ModifyKind::Name(
                        notify::event::RenameMode::From
                    ))
            ) {
                for path in &event.paths {
                    // Remove affected path from watched path map to restart
                    // watching on it later again.
                    watched.remove(path);
                    missing.insert(path.clone());
                }
            }

            recompile |= is_event_relevant(&event, &output);
        }

        // notify-rs unwatches on remove/rename, potentially breaking watches if affected files
        // are removed outside the [`WATCH_TIMEOUT`] duration.
        // So we regularly check whether it reappears.
        if !recompile {
            recompile = missing.iter().any(|path| path.exists());
        }

        if recompile {
            // Reset all dependencies.
            world.reset();

            // Recompile.
            timer
                .record(&mut world, |world| compile_once(world, &mut command, true))??;

            comemo::evict(10);

            // Adjust the file watching.
            watch_dependencies(&mut world, &mut watcher, &mut watched, &mut missing)?;
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
    missing: &mut HashSet<PathBuf>,
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
        let is_missing = missing.remove(&path);
        if is_missing || !watched.contains_key(&path) {
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
