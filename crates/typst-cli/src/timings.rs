use std::fs::File;
use std::io::BufWriter;
use std::path::{Path, PathBuf};

use typst::diag::{bail, StrResult};
use typst::syntax::Span;
use typst::World;

use crate::args::{CliArguments, Command};
use crate::world::SystemWorld;

/// Allows to record timings of function executions.
pub struct Timer {
    /// Where to save the recorded timings of each compilation step.
    path: Option<PathBuf>,
    /// The current watch iteration.
    index: usize,
}

impl Timer {
    /// Initializes the timing system and returns a timer that can be used to
    /// record timings for a specific function invocation.
    pub fn new(args: &CliArguments) -> Self {
        let record = match &args.command {
            Command::Compile(command) => command.args.timings.clone(),
            Command::Watch(command) => command.args.timings.clone(),
            _ => None,
        };

        // Enable event collection.
        if record.is_some() {
            typst_timing::enable();
        }

        let path =
            record.map(|path| path.unwrap_or_else(|| PathBuf::from("record-{n}.json")));

        Self { path, index: 0 }
    }

    /// Records all timings in `f` and writes them to disk.
    pub fn record<T>(
        &mut self,
        world: &mut SystemWorld,
        f: impl FnOnce(&mut SystemWorld) -> T,
    ) -> StrResult<T> {
        let Some(path) = &self.path else {
            return Ok(f(world));
        };

        typst_timing::clear();

        let string = path.to_str().unwrap_or_default();
        let numbered = string.contains("{n}");
        if !numbered && self.index > 0 {
            bail!("cannot export multiple recordings without `{{n}}` in path");
        }

        let storage;
        let path = if numbered {
            storage = string.replace("{n}", &self.index.to_string());
            Path::new(&storage)
        } else {
            path.as_path()
        };

        let output = f(world);
        self.index += 1;

        let file =
            File::create(path).map_err(|e| format!("failed to create file: {e}"))?;
        let writer = BufWriter::with_capacity(1 << 20, file);

        typst_timing::export_json(writer, |span| {
            resolve_span(world, Span::from_raw(span))
                .unwrap_or_else(|| ("unknown".to_string(), 0))
        })?;

        Ok(output)
    }
}

/// Turns a span into a (file, line) pair.
fn resolve_span(world: &SystemWorld, span: Span) -> Option<(String, u32)> {
    let id = span.id()?;
    let source = world.source(id).ok()?;
    let range = source.range(span)?;
    let line = source.lines().byte_to_line(range.start)?;
    Some((format!("{id:?}"), line as u32 + 1))
}
