use std::fs::File;
use std::io::BufWriter;
use std::path::{Path, PathBuf};

use typst::diag::{bail, StrResult};
use typst::syntax::Span;
use typst::World;

use crate::args::{CliArguments, Command};
use crate::world::SystemWorld;

/// Initializes the tracing system and returns a guard that will flush the
/// recorder to disk when dropped.
pub fn setup(args: &CliArguments) -> TimignHandle {
    let record = match &args.command {
        Command::Compile(command) => command.timings.clone(),
        Command::Watch(command) => command.timings.clone(),
        _ => None,
    };

    // Enable event collection.
    if record.is_some() {
        typst_timing::enable();
    }

    TimignHandle {
        record: record
            .map(|path| path.unwrap_or_else(|| PathBuf::from("record-{n}.json"))),
        index: 0,
    }
}

/// Will flush the flamegraph to disk when dropped.
pub struct TimignHandle {
    /// Where to save the recorded trace of each compilation step.
    record: Option<PathBuf>,
    /// The current trace iteration.
    index: usize,
}

impl TimignHandle {
    /// Record all traces in `f`.
    pub fn record<O>(
        &mut self,
        world: &mut SystemWorld,
        f: impl FnOnce(&mut SystemWorld) -> O,
    ) -> StrResult<O> {
        let Some(record) = &self.record else {
            return Ok(f(world));
        };

        typst_timing::clear();

        let string = record.to_str().unwrap_or_default();
        let numbered = string.contains("{n}");
        if !numbered && self.index > 0 {
            bail!("cannot export multiple recordings without `{{n}}` in path");
        }

        let storage;
        let path = if numbered {
            storage = string.replace("{n}", &self.index.to_string());
            Path::new(&storage)
        } else {
            record.as_path()
        };

        let output = f(world);
        self.index += 1;

        let file =
            File::create(path).map_err(|e| format!("failed to create file: {e}"))?;
        let writer = BufWriter::with_capacity(1 << 20, file);

        typst_timing::export_json(writer, |span| {
            resolve_span(world, span).unwrap_or_else(|| ("unknown".to_string(), 0))
        })?;

        Ok(output)
    }
}

/// Turns a span into a (file, line) pair.
fn resolve_span(world: &SystemWorld, span: Span) -> Option<(String, u32)> {
    let id = span.id()?;
    let source = world.source(id).ok()?;
    let range = source.range(span)?;
    let line = source.byte_to_line(range.start)?;
    Some((format!("{id:?}"), line as u32 + 1))
}
