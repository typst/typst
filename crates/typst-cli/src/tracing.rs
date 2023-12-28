use std::io;
use std::path::PathBuf;

use typst::World;
use typst::diag::{StrResult, bail};
use typst::syntax::Span;

use crate::args::{CliArguments, Command};

/// Initializes the tracing system and returns a guard that will flush the
/// recorder to disk when dropped.
pub fn setup_tracing(args: &CliArguments) -> io::Result<TracingHandle> {
    let cmd = match &args.command {
        Command::Compile(command) => command,
        Command::Watch(command) => command,
        _ => return Ok(TracingHandle {
            incremental: false,
            record: None,
            index: 0,
        }),
    };

    // Enable tracing
    typst_trace::enable();

    Ok(TracingHandle {
        incremental: matches!(&args.command, Command::Watch(_)),
        record: cmd
            .record
            .clone()
            .map(|x| x.unwrap_or_else(|| PathBuf::from("record-{n}.json"))),
        index: 0,
    })
}

/// Will flush the flamegraph to disk when dropped.
pub struct TracingHandle {
    /// Whether Typst is running in incremental mode.
    incremental: bool,
    /// Whether to produce the recorder trace of each compilation step.
    record: Option<PathBuf>,
    /// The current iteration.
    index: u64,
}

impl TracingHandle {
    pub fn record<O, W>(
        &mut self,
        world: &mut W,
        fun: impl FnOnce(&mut W) -> O,
        mut source: impl FnMut(&mut W, Span) -> (String, u32)
    ) -> StrResult<O> where W: World {
        if self.record.is_some() {
            let handle = RecordHandle::new(self)?;
            let output = fun(world);
            handle.finish(|span| source(world, span))?;
            Ok(output)
        } else {
            Ok(fun(world))
        }
    }
}

pub struct RecordHandle<'a> {
    _tracer: &'a mut TracingHandle,
    record: Option<PathBuf>,
}

impl<'a> RecordHandle<'a> {
    fn new(tracer: &'a mut TracingHandle) -> StrResult<Self> {
        typst_trace::clear();

        // Create the path to the record.
        let record = if let Some(path) = &tracer.record {
            let string = path.to_str().unwrap_or_default();
            let numbered = string.contains("{n}");
            if !numbered && tracer.incremental {
                bail!("cannot export multiple images without `{{n}}` in output path");
            }

            let string = if numbered {
                string.replace("{n}", &tracer.index.to_string())
            } else {
                string.into()
            };

            tracer.index += 1;
            Some(PathBuf::from(string))
        } else {
            None
        };

        Ok(Self { _tracer: tracer, record })
    }

    pub fn finish(self, source: impl FnMut(Span) -> (String, u32)) -> StrResult<()> {
        if let Some(path) = self.record {
            typst_trace::export_json(path, source)?;
        }

        Ok(())
    }
}
