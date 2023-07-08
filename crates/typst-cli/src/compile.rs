use std::fs;
use std::path::Path;

use codespan_reporting::diagnostic::{Diagnostic, Label};
use codespan_reporting::term::{self, termcolor};
use termcolor::{ColorChoice, StandardStream};
use typst::diag::{bail, SourceError, StrResult};
use typst::doc::Document;
use typst::eval::eco_format;
use typst::file::FileId;
use typst::geom::Color;
use typst::syntax::Source;
use typst::World;

use crate::args::{CompileCommand, DiagnosticFormat};
use crate::watch::Status;
use crate::world::SystemWorld;
use crate::{color_stream, set_failed};

type CodespanResult<T> = Result<T, CodespanError>;
type CodespanError = codespan_reporting::files::Error;

/// Execute a compilation command.
pub fn compile(mut command: CompileCommand) -> StrResult<()> {
    let mut world = SystemWorld::new(&command)?;
    compile_once(&mut world, &mut command, false)?;
    Ok(())
}

/// Compile a single time.
///
/// Returns whether it compiled without errors.
#[tracing::instrument(skip_all)]
pub fn compile_once(
    world: &mut SystemWorld,
    command: &mut CompileCommand,
    watching: bool,
) -> StrResult<()> {
    tracing::info!("Starting compilation");

    let start = std::time::Instant::now();
    if watching {
        Status::Compiling.print(command).unwrap();
    }

    // Reset everything and ensure that the main file is still present.
    world.reset();
    world.source(world.main()).map_err(|err| err.to_string())?;

    let result = typst::compile(world);
    let duration = start.elapsed();

    match result {
        // Export the PDF / PNG.
        Ok(document) => {
            export(&document, command)?;

            tracing::info!("Compilation succeeded in {duration:?}");
            if watching {
                Status::Success(duration).print(command).unwrap();
            }

            if let Some(open) = command.open.take() {
                open_file(open.as_deref(), &command.output())?;
            }
        }

        // Print diagnostics.
        Err(errors) => {
            set_failed();
            tracing::info!("Compilation failed");

            if watching {
                Status::Error.print(command).unwrap();
            }

            print_diagnostics(world, *errors, command.diagnostic_format)
                .map_err(|_| "failed to print diagnostics")?;
        }
    }

    Ok(())
}

/// Export into the target format.
fn export(document: &Document, command: &CompileCommand) -> StrResult<()> {
    match command.output().extension() {
        Some(ext) if ext.eq_ignore_ascii_case("png") => export_png(document, command),
        _ => export_pdf(document, command),
    }
}

/// Export to a PDF.
fn export_pdf(document: &Document, command: &CompileCommand) -> StrResult<()> {
    let output = command.output();
    let buffer = typst::export::pdf(document);
    fs::write(output, buffer).map_err(|_| "failed to write PDF file")?;
    Ok(())
}

/// Export to one or multiple PNGs.
fn export_png(document: &Document, command: &CompileCommand) -> StrResult<()> {
    // Determine whether we have a `{n}` numbering.
    let output = command.output();
    let string = output.to_str().unwrap_or_default();
    let numbered = string.contains("{n}");
    if !numbered && document.pages.len() > 1 {
        bail!("cannot export multiple PNGs without `{{n}}` in output path");
    }

    // Find a number width that accommodates all pages. For instance, the
    // first page should be numbered "001" if there are between 100 and
    // 999 pages.
    let width = 1 + document.pages.len().checked_ilog10().unwrap_or(0) as usize;
    let mut storage;

    for (i, frame) in document.pages.iter().enumerate() {
        let pixmap = typst::export::render(frame, command.ppi / 72.0, Color::WHITE);
        let path = if numbered {
            storage = string.replace("{n}", &format!("{:0width$}", i + 1));
            Path::new(&storage)
        } else {
            output.as_path()
        };
        pixmap.save_png(path).map_err(|_| "failed to write PNG file")?;
    }

    Ok(())
}

/// Opens the given file using:
/// - The default file viewer if `open` is `None`.
/// - The given viewer provided by `open` if it is `Some`.
fn open_file(open: Option<&str>, path: &Path) -> StrResult<()> {
    if let Some(app) = open {
        open::with_in_background(path, app);
    } else {
        open::that_in_background(path);
    }

    Ok(())
}

/// Print diagnostic messages to the terminal.
fn print_diagnostics(
    world: &SystemWorld,
    errors: Vec<SourceError>,
    diagnostic_format: DiagnosticFormat,
) -> Result<(), codespan_reporting::files::Error> {
    let mut w = match diagnostic_format {
        DiagnosticFormat::Human => color_stream(),
        DiagnosticFormat::Short => StandardStream::stderr(ColorChoice::Never),
    };

    let mut config = term::Config { tab_width: 2, ..Default::default() };
    if diagnostic_format == DiagnosticFormat::Short {
        config.display_style = term::DisplayStyle::Short;
    }

    for error in errors {
        // The main diagnostic.
        let diag = Diagnostic::error()
            .with_message(error.message)
            .with_notes(
                error
                    .hints
                    .iter()
                    .map(|e| (eco_format!("hint: {e}")).into())
                    .collect(),
            )
            .with_labels(vec![Label::primary(error.span.id(), error.span.range(world))]);

        term::emit(&mut w, &config, world, &diag)?;

        // Stacktrace-like helper diagnostics.
        for point in error.trace {
            let message = point.v.to_string();
            let help = Diagnostic::help().with_message(message).with_labels(vec![
                Label::primary(point.span.id(), point.span.range(world)),
            ]);

            term::emit(&mut w, &config, world, &help)?;
        }
    }

    Ok(())
}

impl<'a> codespan_reporting::files::Files<'a> for SystemWorld {
    type FileId = FileId;
    type Name = FileId;
    type Source = Source;

    fn name(&'a self, id: FileId) -> CodespanResult<Self::Name> {
        Ok(id)
    }

    fn source(&'a self, id: FileId) -> CodespanResult<Self::Source> {
        Ok(self.lookup(id))
    }

    fn line_index(&'a self, id: FileId, given: usize) -> CodespanResult<usize> {
        let source = self.lookup(id);
        source
            .byte_to_line(given)
            .ok_or_else(|| CodespanError::IndexTooLarge {
                given,
                max: source.len_bytes(),
            })
    }

    fn line_range(
        &'a self,
        id: FileId,
        given: usize,
    ) -> CodespanResult<std::ops::Range<usize>> {
        let source = self.lookup(id);
        source
            .line_to_range(given)
            .ok_or_else(|| CodespanError::LineTooLarge { given, max: source.len_lines() })
    }

    fn column_number(
        &'a self,
        id: FileId,
        _: usize,
        given: usize,
    ) -> CodespanResult<usize> {
        let source = self.lookup(id);
        source.byte_to_column(given).ok_or_else(|| {
            let max = source.len_bytes();
            if given <= max {
                CodespanError::InvalidCharBoundary { given }
            } else {
                CodespanError::IndexTooLarge { given, max }
            }
        })
    }
}
