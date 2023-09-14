use std::fs::File;
use std::io::{self, BufReader, BufWriter, Seek, SeekFrom};
use std::path::PathBuf;

use inferno::flamegraph::Options;
use tracing::metadata::LevelFilter;
use tracing_error::ErrorLayer;
use tracing_flame::{FlameLayer, FlushGuard};
use tracing_subscriber::fmt;
use tracing_subscriber::prelude::*;

use crate::args::{CliArguments, Command};

/// Initializes the tracing system and returns a guard that will flush the
/// flamegraph to disk when dropped.
pub fn setup_tracing(args: &CliArguments) -> io::Result<Option<impl Drop>> {
    let flamegraph = match &args.command {
        Command::Compile(command) => command.flamegraph.as_ref(),
        Command::Watch(command) if command.flamegraph.is_some() => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "cannot use --flamegraph with watch command",
            ));
        }
        _ => None,
    };

    // Short circuit if we don't need to initialize flamegraph or debugging.
    if flamegraph.is_none() && args.verbosity == 0 {
        tracing_subscriber::fmt()
            .without_time()
            .with_max_level(level_filter(args))
            .init();

        return Ok(None);
    }

    // Build the FMT layer printing to the console.
    let fmt_layer = fmt::Layer::default().without_time().with_filter(level_filter(args));

    // Error layer for building backtraces
    let error_layer = ErrorLayer::default();

    // Build the registry.
    let registry = tracing_subscriber::registry().with(fmt_layer).with(error_layer);

    let Some(path) = flamegraph else {
        registry.init();
        return Ok(None);
    };

    // Create a temporary file to store the flamegraph data.
    let temp_file = tempfile::tempfile()?;
    let writer = BufWriter::new(temp_file.try_clone()?);

    // Build the flamegraph layer.
    let flame_layer = FlameLayer::new(writer)
        .with_empty_samples(false)
        .with_threads_collapsed(true)
        .with_module_path(false)
        .with_file_and_line(true);
    let flush_guard = flame_layer.flush_on_drop();

    // Build the subscriber.
    registry.with(flame_layer).init();

    tracing::warn!(
        "Flamegraph is enabled, this can create a large temporary \
         file and slow down the compilation process."
    );

    Ok(Some(TracingGuard {
        flush_guard: Some(flush_guard),
        temp_file,
        output_svg: path.clone().unwrap_or_else(|| "flamegraph.svg".into()),
    }))
}

/// Returns the log level filter for the given verbosity level.
fn level_filter(args: &CliArguments) -> LevelFilter {
    match args.verbosity {
        0 => LevelFilter::OFF,
        1 => LevelFilter::WARN,
        2 => LevelFilter::INFO,
        3 => LevelFilter::DEBUG,
        _ => LevelFilter::TRACE,
    }
}

/// Will flush the flamegraph to disk when dropped.
struct TracingGuard {
    flush_guard: Option<FlushGuard<BufWriter<File>>>,
    temp_file: File,
    output_svg: PathBuf,
}

impl TracingGuard {
    fn finish(&mut self) -> io::Result<()> {
        if self.flush_guard.is_none() {
            return Ok(());
        }

        tracing::info!("Flushing tracing flamegraph...");

        // At this point, we're done tracing, so we can drop the guard.
        // This will flush the tracing output to disk.
        // We can then read the file and generate the flamegraph.
        drop(self.flush_guard.take());

        // Reset the file pointer to the beginning.
        self.temp_file.seek(SeekFrom::Start(0))?;

        // Create the readers and writers.
        let reader = BufReader::new(&mut self.temp_file);
        let output = BufWriter::new(File::create(&self.output_svg)?);

        // Create the options: default in flame chart mode
        let mut options = Options::default();
        options.flame_chart = true;

        inferno::flamegraph::from_reader(&mut options, reader, output)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        Ok(())
    }
}

impl Drop for TracingGuard {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            if let Err(err) = self.finish() {
                // Since we are finished, we cannot rely on tracing to log the
                // error.
                eprintln!("failed to flush tracing flamegraph ({err})");
            }
        }
    }
}
