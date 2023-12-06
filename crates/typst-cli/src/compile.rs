use std::fs;
use std::path::{Path, PathBuf};

use chrono::{Datelike, Timelike};
use codespan_reporting::diagnostic::{Diagnostic, Label};
use codespan_reporting::term::{self, termcolor};
use ecow::eco_format;
use termcolor::{ColorChoice, StandardStream};
use typst::diag::{bail, At, Severity, SourceDiagnostic, StrResult};
use typst::eval::Tracer;
use typst::foundations::Datetime;
use typst::model::Document;
use typst::syntax::{FileId, Source, Span};
use typst::visualize::Color;
use typst::{World, WorldExt};

use crate::args::{CompileCommand, DiagnosticFormat, OutputFormat};
use crate::watch::Status;
use crate::world::SystemWorld;
use crate::{color_stream, set_failed};

type CodespanResult<T> = Result<T, CodespanError>;
type CodespanError = codespan_reporting::files::Error;

impl CompileCommand {
    /// The output path.
    pub fn output(&self) -> PathBuf {
        self.output.clone().unwrap_or_else(|| {
            self.common.input.with_extension(
                match self.output_format().unwrap_or(OutputFormat::Pdf) {
                    OutputFormat::Pdf => "pdf",
                    OutputFormat::Png => "png",
                    OutputFormat::Svg => "svg",
                },
            )
        })
    }

    /// The format to use for generated output, either specified by the user or inferred from the extension.
    ///
    /// Will return `Err` if the format was not specified and could not be inferred.
    pub fn output_format(&self) -> StrResult<OutputFormat> {
        Ok(if let Some(specified) = self.format {
            specified
        } else if let Some(output) = &self.output {
            match output.extension() {
                Some(ext) if ext.eq_ignore_ascii_case("pdf") => OutputFormat::Pdf,
                Some(ext) if ext.eq_ignore_ascii_case("png") => OutputFormat::Png,
                Some(ext) if ext.eq_ignore_ascii_case("svg") => OutputFormat::Svg,
                _ => bail!("could not infer output format for path {}.\nconsider providing the format manually with `--format/-f`", output.display()),
            }
        } else {
            OutputFormat::Pdf
        })
    }
}

/// Execute a compilation command.
pub fn compile(mut command: CompileCommand) -> StrResult<()> {
    let mut world = SystemWorld::new(&command.common)?;
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

    // Check if main file can be read and opened.
    if let Err(errors) = world.source(world.main()).at(Span::detached()) {
        set_failed();
        tracing::info!("Failed to open and decode main file");

        if watching {
            Status::Error.print(command).unwrap();
        }

        print_diagnostics(world, &errors, &[], command.common.diagnostic_format)
            .map_err(|err| eco_format!("failed to print diagnostics ({err})"))?;

        return Ok(());
    }

    let mut tracer = Tracer::new();
    let result = typst::compile(world, &mut tracer);
    let warnings = tracer.warnings();

    match result {
        // Export the PDF / PNG.
        Ok(document) => {
            export(world, &document, command, watching)?;
            let duration = start.elapsed();

            tracing::info!("Compilation succeeded in {duration:?}");
            if watching {
                if warnings.is_empty() {
                    Status::Success(duration).print(command).unwrap();
                } else {
                    Status::PartialSuccess(duration).print(command).unwrap();
                }
            }

            print_diagnostics(world, &[], &warnings, command.common.diagnostic_format)
                .map_err(|err| eco_format!("failed to print diagnostics ({err})"))?;

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

            print_diagnostics(
                world,
                &errors,
                &warnings,
                command.common.diagnostic_format,
            )
            .map_err(|err| eco_format!("failed to print diagnostics ({err})"))?;
        }
    }

    Ok(())
}

/// Export into the target format.
fn export(
    world: &mut SystemWorld,
    document: &Document,
    command: &CompileCommand,
    watching: bool,
) -> StrResult<()> {
    match command.output_format()? {
        OutputFormat::Png => {
            export_image(world, document, command, watching, ImageExportFormat::Png)
        }
        OutputFormat::Svg => {
            export_image(world, document, command, watching, ImageExportFormat::Svg)
        }
        OutputFormat::Pdf => export_pdf(document, command, world),
    }
}

/// Export to a PDF.
fn export_pdf(
    document: &Document,
    command: &CompileCommand,
    world: &SystemWorld,
) -> StrResult<()> {
    let ident = world.input().to_string_lossy();
    let buffer = typst_pdf::pdf(document, Some(&ident), now());
    let output = command.output();
    fs::write(output, buffer)
        .map_err(|err| eco_format!("failed to write PDF file ({err})"))?;
    Ok(())
}

/// Get the current date and time in UTC.
fn now() -> Option<Datetime> {
    let now = chrono::Local::now().naive_utc();
    Datetime::from_ymd_hms(
        now.year(),
        now.month().try_into().ok()?,
        now.day().try_into().ok()?,
        now.hour().try_into().ok()?,
        now.minute().try_into().ok()?,
        now.second().try_into().ok()?,
    )
}

/// An image format to export in.
enum ImageExportFormat {
    Png,
    Svg,
}

/// Export to one or multiple PNGs.
fn export_image(
    world: &mut SystemWorld,
    document: &Document,
    command: &CompileCommand,
    watching: bool,
    fmt: ImageExportFormat,
) -> StrResult<()> {
    // Determine whether we have a `{n}` numbering.
    let output = command.output();
    let string = output.to_str().unwrap_or_default();
    let numbered = string.contains("{n}");
    if !numbered && document.pages.len() > 1 {
        bail!("cannot export multiple images without `{{n}}` in output path");
    }

    // Find a number width that accommodates all pages. For instance, the
    // first page should be numbered "001" if there are between 100 and
    // 999 pages.
    let width = 1 + document.pages.len().checked_ilog10().unwrap_or(0) as usize;
    let mut storage;

    let cache = world.export_cache();
    for (i, frame) in document.pages.iter().enumerate() {
        let path = if numbered {
            storage = string.replace("{n}", &format!("{:0width$}", i + 1));
            Path::new(&storage)
        } else {
            output.as_path()
        };

        // If we are not watching, don't use the cache.
        // If the frame is in the cache, skip it.
        // If the file does not exist, always create it.
        if watching && cache.is_cached(i, frame) && path.exists() {
            continue;
        }

        match fmt {
            ImageExportFormat::Png => {
                let pixmap =
                    typst_render::render(frame, command.ppi / 72.0, Color::WHITE);
                pixmap
                    .save_png(path)
                    .map_err(|err| eco_format!("failed to write PNG file ({err})"))?;
            }
            ImageExportFormat::Svg => {
                let svg = typst_svg::svg(frame);
                fs::write(path, svg.as_bytes())
                    .map_err(|err| eco_format!("failed to write SVG file ({err})"))?;
            }
        }
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
pub fn print_diagnostics(
    world: &SystemWorld,
    errors: &[SourceDiagnostic],
    warnings: &[SourceDiagnostic],
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

    for diagnostic in warnings.iter().chain(errors) {
        let diag = match diagnostic.severity {
            Severity::Error => Diagnostic::error(),
            Severity::Warning => Diagnostic::warning(),
        }
        .with_message(diagnostic.message.clone())
        .with_notes(
            diagnostic
                .hints
                .iter()
                .map(|e| (eco_format!("hint: {e}")).into())
                .collect(),
        )
        .with_labels(label(world, diagnostic.span).into_iter().collect());

        term::emit(&mut w, &config, world, &diag)?;

        // Stacktrace-like helper diagnostics.
        for point in &diagnostic.trace {
            let message = point.v.to_string();
            let help = Diagnostic::help()
                .with_message(message)
                .with_labels(label(world, point.span).into_iter().collect());

            term::emit(&mut w, &config, world, &help)?;
        }
    }

    Ok(())
}

/// Create a label for a span.
fn label(world: &SystemWorld, span: Span) -> Option<Label<FileId>> {
    Some(Label::primary(span.id()?, world.range(span)?))
}

impl<'a> codespan_reporting::files::Files<'a> for SystemWorld {
    type FileId = FileId;
    type Name = String;
    type Source = Source;

    fn name(&'a self, id: FileId) -> CodespanResult<Self::Name> {
        let vpath = id.vpath();
        Ok(if let Some(package) = id.package() {
            format!("{package}{}", vpath.as_rooted_path().display())
        } else {
            // Try to express the path relative to the working directory.
            vpath
                .resolve(self.root())
                .and_then(|abs| pathdiff::diff_paths(abs, self.workdir()))
                .as_deref()
                .unwrap_or_else(|| vpath.as_rootless_path())
                .to_string_lossy()
                .into()
        })
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
