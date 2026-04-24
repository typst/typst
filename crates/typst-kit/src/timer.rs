//! Recording and writing of performance timing files.
//!
//! This can be used to record performance events via [`typst_timing`] and write
//! them to disk.

#![cfg(feature = "timer")]

use std::fs::File;
use std::io::BufWriter;
use std::path::{Path, PathBuf};

use typst_library::World;
use typst_library::diag::{StrResult, bail};
use typst_syntax::Span;

/// Allows to record timings of function executions.
pub struct Timer {
    /// Where to save the recorded timings of each compilation step.
    path: Option<PathBuf>,
    /// The current watch iteration.
    iter: usize,
}

impl Timer {
    /// Creates a timer that can be used to record timings for a specific
    /// function invocation.
    ///
    /// Will also internally enable event collection in [`typst_timing`].
    ///
    /// If the path contains the string `{n}`, it is replaced with a per
    /// recording index. If recording multiple events, the path _must_ contain
    /// this string.
    pub fn new(path: PathBuf) -> Self {
        // Enable event collection.
        typst_timing::enable();
        Self { path: Some(path), iter: 0 }
    }

    /// Returns a placeholder that does not record any actual timings. This can
    /// be useful to have uniform code paths with `timer.record` regardless of
    /// whether timings are enabled at runtime.
    pub fn placeholder() -> Self {
        Self { path: None, iter: 0 }
    }

    /// Creates a proper timer if the `path` is `Some(_)` or a placeholder timer
    /// if the path is `None`.
    pub fn new_or_placeholder(path: Option<PathBuf>) -> Self {
        match path {
            Some(path) => Self::new(path),
            None => Self::placeholder(),
        }
    }

    /// Records all timings in `f` and writes them to disk as JSON compatible
    /// with Chrome's tracing tool.
    pub fn record<W, T>(
        &mut self,
        world: &mut W,
        f: impl FnOnce(&mut W) -> T,
    ) -> StrResult<T>
    where
        W: World,
    {
        let Some(path) = &self.path else {
            return Ok(f(world));
        };

        typst_timing::clear();

        let string = path.to_str().unwrap_or_default();
        let numbered = string.contains("{n}");
        if !numbered && self.iter > 0 {
            bail!("cannot export multiple recordings without `{{n}}` in path");
        }

        let storage;
        let path = if numbered {
            storage = string.replace("{n}", &self.iter.to_string());
            Path::new(&storage)
        } else {
            path.as_path()
        };

        let output = f(world);
        self.iter += 1;

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
fn resolve_span<W>(world: &W, span: Span) -> Option<(String, u32)>
where
    W: World,
{
    let id = span.id()?;
    let line = match span.range() {
        Some(range) => {
            let file = world.file(id).ok()?;
            let lines = file.lines().ok()?;
            lines.byte_to_line(range.start)?
        }
        None => {
            let source = world.source(id).ok()?;
            let range = source.range(span)?;
            source.lines().byte_to_line(range.start)?
        }
    };
    Some((format!("{id:?}"), line as u32 + 1))
}
