//! Diagnostic pretty-printing.

#![cfg(feature = "emit-diagnostics")]

use std::collections::HashMap;
use std::io;
use std::ops::Range;

use codespan_reporting::diagnostic::{Diagnostic, Label};
use codespan_reporting::files::Files;
use codespan_reporting::term;
use ecow::eco_format;
use termcolor::{Color, ColorSpec, WriteColor};
use typst_library::World;
use typst_library::diag::{FileError, Severity, SourceDiagnostic, Tracepoint};
use typst_syntax::{FileId, Lines, Source, Span, Spanned};

type CodespanResult<T> = Result<T, CodespanError>;
type CodespanError = codespan_reporting::files::Error;

pub use term::termcolor;

/// Extends the [`World`] for diagnostic printing.
pub trait DiagnosticWorld: World {
    /// Formats a file ID for user-facing display.
    ///
    /// In the CLI, this formats as a path relative to the working directory.
    fn name(&self, id: FileId) -> String;
}

/// Which format to use for diagnostics.
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum DiagnosticFormat {
    /// Displays a richly formatted message showing the source code and context.
    #[default]
    Human,
    /// Displays a short single-line diagnostic.
    Short,
}

/// Emits diagnostic messages to a writable, colorized output.
pub fn emit<'a>(
    dest: &mut dyn WriteColor,
    world: &dyn DiagnosticWorld,
    diagnostics: impl IntoIterator<Item = &'a SourceDiagnostic>,
    format: DiagnosticFormat,
) -> Result<(), codespan_reporting::files::Error> {
    let mut files = WorldFiles { world, sources: HashMap::new() };

    let mut config = term::Config { tab_width: 2, ..Default::default() };
    if format == DiagnosticFormat::Short {
        config.display_style = term::DisplayStyle::Short;
    }

    for diagnostic in diagnostics {
        let diag = match diagnostic.severity {
            Severity::Error => Diagnostic::error(),
            Severity::Warning => Diagnostic::warning(),
        }
        .with_message(diagnostic.message.clone())
        .with_notes(
            diagnostic
                .hints
                .iter()
                .filter(|s| s.span.is_detached())
                .map(|s| (eco_format!("hint: {}", s.v)).into())
                .collect(),
        )
        .with_labels(
            diagnostic
                .span
                .id()
                .and_then(|id| {
                    let range = files.range(diagnostic.span)?;
                    Some(Label::primary(id, range))
                })
                .into_iter()
                .chain(diagnostic.hints.iter().filter_map(|hint| {
                    let id = hint.span.id()?;
                    let range = files.range(hint.span)?;
                    Some(Label::secondary(id, range).with_message(&hint.v))
                }))
                .collect(),
        );

        term::emit(dest, &config, &files, &diag)?;

        // Stacktrace-like helper diagnostics.
        let mut traced = false;
        for point in &diagnostic.trace {
            emit_trace(dest, &mut files, point)?;
            traced = true;
        }

        if traced {
            writeln!(dest)?;
        }
    }

    Ok(())
}

/// Emits a tracepoint.
fn emit_trace(
    dest: &mut dyn WriteColor,
    files: &mut WorldFiles,
    point: &Spanned<Tracepoint>,
) -> Result<(), codespan_reporting::files::Error> {
    let Some(id) = point.span.id() else { return Ok(()) };
    let Some(range) = files.range(point.span) else { return Ok(()) };
    let lines = files.lines(id)?;

    let name = files.name(id)?;
    let line_index = files.line_index(id, range.start)?;
    let line = files.line_number(id, line_index)?;
    let column = files.column_number(id, line_index, range.start)?;
    let text = &lines.text()[range];

    // Displays what kind of tracepoint we have and where.
    write!(dest, "  {} at ", point.v)?;
    dest.set_color(ColorSpec::new().set_underline(true))?;
    write!(dest, "{name}:{line}:{column}")?;
    dest.reset()?;
    writeln!(dest)?;

    // Displays the context in the source in a single line.
    let mut lines = text.lines();
    write!(dest, "    ")?;
    dest.set_color(ColorSpec::new().set_fg(Some(Color::Ansi256(248))))?;
    if let Some(first) = lines.next() {
        write!(dest, "{first}")?;
    }
    if let Some(last) = lines.next_back()
        && let Some(last_char) = last.chars().next_back()
        && !last_char.is_whitespace()
    {
        // If the traced source text is multi-line, try to display it
        // with inner ellipses followed by the last character.
        write!(dest, "…{last_char}")?;
    }
    dest.reset()?;
    writeln!(dest)?;

    Ok(())
}

/// Provides file contents and metadata to `codespan-reporting`.
struct WorldFiles<'a> {
    world: &'a dyn DiagnosticWorld,
    sources: HashMap<FileId, Source>,
}

impl WorldFiles<'_> {
    /// Determine the byte range of a span, also remembering the source file
    /// for future line / column lookups.
    fn range(&mut self, span: Span) -> Option<Range<usize>> {
        span.range().or_else(|| {
            let id = span.id()?;
            let source = self.world.source(id).ok()?;
            let range = source.range(span);
            self.sources.entry(id).or_insert(source);
            range
        })
    }

    /// Lookup line metadata for a file by id. If a source file was remembered,
    /// it will be used. Otherwise, we load as a file and compute line metadata.
    fn lines(&self, id: FileId) -> CodespanResult<Lines<String>> {
        match self.sources.get(&id) {
            Some(source) => Ok(source.lines().clone()),
            None => self
                .world
                .file(id)
                .and_then(|file| file.lines().map_err(Into::into))
                .map_err(|err| match err {
                    FileError::NotFound(_) => CodespanError::FileMissing,
                    other => CodespanError::Io(io::Error::other(other)),
                }),
        }
    }
}

impl<'a> Files<'a> for WorldFiles<'_> {
    type FileId = FileId;
    type Name = String;
    type Source = Lines<String>;

    fn name(&'a self, id: FileId) -> CodespanResult<Self::Name> {
        Ok(self.world.name(id))
    }

    fn source(&'a self, id: FileId) -> CodespanResult<Self::Source> {
        self.lines(id)
    }

    fn line_index(&'a self, id: FileId, given: usize) -> CodespanResult<usize> {
        let lines = self.lines(id)?;
        lines
            .byte_to_line(given)
            .ok_or_else(|| CodespanError::IndexTooLarge { given, max: lines.len_bytes() })
    }

    fn line_range(
        &'a self,
        id: FileId,
        given: usize,
    ) -> CodespanResult<std::ops::Range<usize>> {
        let lines = self.lines(id)?;
        lines
            .line_to_range(given)
            .ok_or_else(|| CodespanError::LineTooLarge { given, max: lines.len_lines() })
    }

    fn column_number(
        &'a self,
        id: FileId,
        _: usize,
        given: usize,
    ) -> CodespanResult<usize> {
        let lines = self.lines(id)?;
        lines.byte_to_column(given).ok_or_else(|| {
            let max = lines.len_bytes();
            if given <= max {
                CodespanError::InvalidCharBoundary { given }
            } else {
                CodespanError::IndexTooLarge { given, max }
            }
        })
    }
}
