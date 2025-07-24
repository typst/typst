use std::collections::{HashMap, HashSet};
use std::io::{self, Write};
use std::iter;
use std::path::PathBuf;
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

use codespan_reporting::term::termcolor::WriteColor;
use codespan_reporting::term::{self, termcolor};
use ecow::eco_format;
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher as _};
use same_file::is_same_file;
use typst::diag::{StrResult, bail, warning};
use typst::syntax::Span;
use typst::utils::format_duration;

use crate::args::{Input, Output, WatchCommand};
use crate::compile::{CompileConfig, compile_once, print_diagnostics};
use crate::timings::Timer;
use crate::world::{SystemWorld, WorldCreationError};
use crate::{print_error, terminal};

/// Execute a watching compilation command.
pub fn watch(timer: &mut Timer, command: &WatchCommand) -> StrResult<()> {
    let mut config = CompileConfig::watching(command)?;

    let Output::Path(output) = &config.output else {
        bail!("cannot write document to stdout in watch mode");
    };

    // Create a file system watcher.
    let mut watcher = Watcher::new(output.clone())?;

    // Create the world that serves sources, files, and fonts.
    // Additionally, if any files do not exist, wait until they do.
    let mut world = loop {
        match SystemWorld::new(
            &command.args.input,
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

/// Watches file system activity.
struct Watcher {
    /// The output file. We ignore any events for it.
    output: PathBuf,
    /// The underlying watcher.
    watcher: RecommendedWatcher,
    /// Notify event receiver.
    rx: Receiver<notify::Result<Event>>,
    /// Keeps track of which paths are watched via `watcher`. The boolean is
    /// used during updating for mark-and-sweep garbage collection of paths we
    /// should unwatch.
    watched: HashMap<PathBuf, bool>,
    /// A set of files that should be watched, but don't exist. We manually poll
    /// for those.
    missing: HashSet<PathBuf>,
}

impl Watcher {
    /// How long to wait for a shortly following file system event when
    /// watching.
    const BATCH_TIMEOUT: Duration = Duration::from_millis(100);

    /// The maximum time we spend batching events before quitting wait().
    const STARVE_TIMEOUT: Duration = Duration::from_millis(500);

    /// The interval in which we poll when falling back to poll watching
    /// due to missing files.
    const POLL_INTERVAL: Duration = Duration::from_millis(300);

    /// Create a new, blank watcher.
    fn new(output: PathBuf) -> StrResult<Self> {
        // Setup file watching.
        let (tx, rx) = std::sync::mpsc::channel();

        // Set the poll interval to something more eager than the default. That
        // default seems a bit excessive for our purposes at around 30s.
        // Depending on feedback, some tuning might still be in order. Note that
        // this only affects a tiny number of systems. Most do not use the
        // [`notify::PollWatcher`].
        let config = notify::Config::default().with_poll_interval(Self::POLL_INTERVAL);
        let watcher = RecommendedWatcher::new(tx, config)
            .map_err(|err| eco_format!("failed to setup file watching ({err})"))?;

        Ok(Self {
            output,
            rx,
            watcher,
            watched: HashMap::new(),
            missing: HashSet::new(),
        })
    }

    /// Update the watching to watch exactly the listed files.
    ///
    /// Files that are not yet watched will be watched. Files that are already
    /// watched, but don't need to be watched anymore, will be unwatched.
    fn update(&mut self, iter: impl IntoIterator<Item = PathBuf>) -> StrResult<()> {
        // Mark all files as not "seen" so that we may unwatch them if they
        // aren't in the dependency list.
        for seen in self.watched.values_mut() {
            *seen = false;
        }

        // Reset which files are missing.
        self.missing.clear();

        // Retrieve the dependencies of the last compilation and watch new paths
        // that weren't watched yet.
        for path in iter {
            // We can't watch paths that don't exist with notify-rs. Instead, we
            // add those to a `missing` set and fall back to manual poll
            // watching.
            if !path.exists() {
                self.missing.insert(path);
                continue;
            }

            // Watch the path if it's not already watched.
            if !self.watched.contains_key(&path) {
                self.watcher
                    .watch(&path, RecursiveMode::NonRecursive)
                    .map_err(|err| eco_format!("failed to watch {path:?} ({err})"))?;
            }

            // Mark the file as "seen" so that we don't unwatch it.
            self.watched.insert(path, true);
        }

        // Unwatch old paths that don't need to be watched anymore.
        self.watched.retain(|path, &mut seen| {
            if !seen {
                self.watcher.unwatch(path).ok();
            }
            seen
        });

        Ok(())
    }

    /// Wait until there is a change to a watched path.
    fn wait(&mut self) -> StrResult<()> {
        loop {
            // Wait for an initial event. If there are missing files, we need to
            // poll those regularly to check whether they are created, so we
            // wait with a smaller timeout.
            let first = self.rx.recv_timeout(if self.missing.is_empty() {
                Duration::MAX
            } else {
                Self::POLL_INTERVAL
            });

            // Watch for file system events. If multiple events happen
            // consecutively all within a certain duration, then they are
            // bunched up without a recompile in-between. This helps against
            // some editors' remove & move behavior. Events are also only
            // watched until a certain point, to hinder a barrage of events from
            // preventing recompilations.
            let mut relevant = false;
            let batch_start = Instant::now();
            for event in first
                .into_iter()
                .chain(iter::from_fn(|| self.rx.recv_timeout(Self::BATCH_TIMEOUT).ok()))
                .take_while(|_| batch_start.elapsed() <= Self::STARVE_TIMEOUT)
            {
                let event = event
                    .map_err(|err| eco_format!("failed to watch dependencies ({err})"))?;

                if !is_relevant_event_kind(&event.kind) {
                    continue;
                }

                // Workaround for notify-rs' implicit unwatch on remove/rename
                // (triggered by some editors when saving files) with the
                // inotify backend. By keeping track of the potentially
                // unwatched files, we can allow those we still depend on to be
                // watched again later on.
                if matches!(
                    event.kind,
                    notify::EventKind::Remove(notify::event::RemoveKind::File)
                        | notify::EventKind::Modify(notify::event::ModifyKind::Name(
                            notify::event::RenameMode::From
                        ))
                ) {
                    for path in &event.paths {
                        // Remove affected path from the watched map to restart
                        // watching on it later again.
                        self.watcher.unwatch(path).ok();
                        self.watched.remove(path);
                    }
                }

                // Don't recompile because the output file changed.
                // FIXME: This doesn't work properly for multifile image export.
                if event
                    .paths
                    .iter()
                    .all(|path| is_same_file(path, &self.output).unwrap_or(false))
                {
                    continue;
                }

                relevant = true;
            }

            // If we found a relevant event or if any of the missing files now
            // exists, stop waiting.
            if relevant || self.missing.iter().any(|path| path.exists()) {
                return Ok(());
            }
        }
    }
}

/// Whether a kind of watch event is relevant for compilation.
fn is_relevant_event_kind(kind: &notify::EventKind) -> bool {
    match kind {
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
        hint: "to compile once and exit, please use `typst compile` instead"
    );
    print_diagnostics(world, &[], &[warning], config.diagnostic_format)
        .map_err(|err| eco_format!("failed to print diagnostics ({err})"))
}
