use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use chrono::{Datelike, Timelike};
use codespan_reporting::diagnostic::{Diagnostic, Label};
use codespan_reporting::term;
use ecow::{eco_format, EcoString};
use parking_lot::RwLock;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use typst::diag::{
    bail, At, Severity, SourceDiagnostic, SourceResult, StrResult, Warned,
};
use typst::foundations::{Datetime, Smart};
use typst::layout::{Frame, Page, PageRanges};
use typst::model::Document;
use typst::syntax::{FileId, Source, Span};
use typst::WorldExt;
use typst_pdf::{PdfOptions, PdfStandards};

use crate::args::{
    CompileCommand, DiagnosticFormat, Input, Output, OutputFormat, PageRangeArgument,
    PdfStandard,
};
use crate::timings::Timer;
use crate::watch::Status;
use crate::world::SystemWorld;
use crate::{set_failed, terminal};

type CodespanResult<T> = Result<T, CodespanError>;
type CodespanError = codespan_reporting::files::Error;

impl CompileCommand {
    /// The output path.
    pub fn output(&self) -> Output {
        self.output.clone().unwrap_or_else(|| {
            let Input::Path(path) = &self.common.input else {
                panic!("output must be specified when input is from stdin, as guarded by the CLI");
            };
            Output::Path(path.with_extension(
                match self.output_format().unwrap_or(OutputFormat::Pdf) {
                    OutputFormat::Pdf => "pdf",
                    OutputFormat::Png => "png",
                    OutputFormat::Svg => "svg",
                },
            ))
        })
    }

    /// The format to use for generated output, either specified by the user or inferred from the extension.
    ///
    /// Will return `Err` if the format was not specified and could not be inferred.
    pub fn output_format(&self) -> StrResult<OutputFormat> {
        Ok(if let Some(specified) = self.format {
            specified
        } else if let Some(Output::Path(output)) = &self.output {
            match output.extension() {
                Some(ext) if ext.eq_ignore_ascii_case("pdf") => OutputFormat::Pdf,
                Some(ext) if ext.eq_ignore_ascii_case("png") => OutputFormat::Png,
                Some(ext) if ext.eq_ignore_ascii_case("svg") => OutputFormat::Svg,
                _ => bail!(
                    "could not infer output format for path {}.\n\
                     consider providing the format manually with `--format/-f`",
                    output.display()
                ),
            }
        } else {
            OutputFormat::Pdf
        })
    }

    /// The ranges of the pages to be exported as specified by the user.
    ///
    /// This returns `None` if all pages should be exported.
    pub fn exported_page_ranges(&self) -> Option<PageRanges> {
        self.pages.as_ref().map(|export_ranges| {
            PageRanges::new(
                export_ranges.iter().map(PageRangeArgument::to_range).collect(),
            )
        })
    }

    /// The PDF standards to try to conform with.
    pub fn pdf_standards(&self) -> StrResult<PdfStandards> {
        if self.pdf_standard.contains(&PdfStandard::A_2b)
            && self.pdf_standard.contains(&PdfStandard::A_3b)
        {
            bail!("PDF can not conform to A-2B and A-3B at the same time")
        }

        let list = self
            .pdf_standard
            .iter()
            .map(|standard| match standard {
                PdfStandard::V_1_7 => typst_pdf::PdfStandard::V_1_7,
                PdfStandard::A_2b => typst_pdf::PdfStandard::A_2b,
                PdfStandard::A_3b => typst_pdf::PdfStandard::A_3b,
            })
            .collect::<Vec<_>>();
        PdfStandards::new(&list)
    }
}

/// Execute a compilation command.
pub fn compile(mut timer: Timer, mut command: CompileCommand) -> StrResult<()> {
    // Only meant for input validation
    _ = command.output_format()?;

    let mut world =
        SystemWorld::new(&command.common).map_err(|err| eco_format!("{err}"))?;
    timer.record(&mut world, |world| compile_once(world, &mut command, false))??;
    Ok(())
}

/// Compile a single time.
///
/// Returns whether it compiled without errors.
#[typst_macros::time(name = "compile once")]
pub fn compile_once(
    world: &mut SystemWorld,
    command: &mut CompileCommand,
    watching: bool,
) -> StrResult<()> {
    let start = std::time::Instant::now();
    if watching {
        Status::Compiling.print(command).unwrap();
    }

    let Warned { output, warnings } = typst::compile(world);
    let result = output.and_then(|document| export(world, &document, command, watching));

    match result {
        // Export the PDF / PNG.
        Ok(()) => {
            let duration = start.elapsed();

            if watching {
                if warnings.is_empty() {
                    Status::Success(duration).print(command).unwrap();
                } else {
                    Status::PartialSuccess(duration).print(command).unwrap();
                }
            }

            print_diagnostics(world, &[], &warnings, command.common.diagnostic_format)
                .map_err(|err| eco_format!("failed to print diagnostics ({err})"))?;

            write_make_deps(world, command)?;

            if let Some(open) = command.open.take() {
                if let Output::Path(file) = command.output() {
                    open_file(open.as_deref(), &file)?;
                }
            }
        }

        // Print diagnostics.
        Err(errors) => {
            set_failed();

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
) -> SourceResult<()> {
    match command.output_format().at(Span::detached())? {
        OutputFormat::Png => {
            export_image(world, document, command, watching, ImageExportFormat::Png)
                .at(Span::detached())
        }
        OutputFormat::Svg => {
            export_image(world, document, command, watching, ImageExportFormat::Svg)
                .at(Span::detached())
        }
        OutputFormat::Pdf => export_pdf(document, command),
    }
}

/// Export to a PDF.
fn export_pdf(document: &Document, command: &CompileCommand) -> SourceResult<()> {
    let options = PdfOptions {
        ident: Smart::Auto,
        timestamp: convert_datetime(
            command.common.creation_timestamp.unwrap_or_else(chrono::Utc::now),
        ),
        page_ranges: command.exported_page_ranges(),
        standards: command.pdf_standards().at(Span::detached())?,
    };
    let buffer = typst_pdf::pdf(document, &options)?;
    command
        .output()
        .write(&buffer)
        .map_err(|err| eco_format!("failed to write PDF file ({err})"))
        .at(Span::detached())?;
    Ok(())
}

/// Convert [`chrono::DateTime`] to [`Datetime`]
fn convert_datetime(date_time: chrono::DateTime<chrono::Utc>) -> Option<Datetime> {
    Datetime::from_ymd_hms(
        date_time.year(),
        date_time.month().try_into().ok()?,
        date_time.day().try_into().ok()?,
        date_time.hour().try_into().ok()?,
        date_time.minute().try_into().ok()?,
        date_time.second().try_into().ok()?,
    )
}

/// An image format to export in.
#[derive(Clone, Copy)]
enum ImageExportFormat {
    Png,
    Svg,
}

/// Export to one or multiple images.
fn export_image(
    world: &mut SystemWorld,
    document: &Document,
    command: &CompileCommand,
    watching: bool,
    fmt: ImageExportFormat,
) -> StrResult<()> {
    let output = command.output();
    // Determine whether we have indexable templates in output
    let can_handle_multiple = match output {
        Output::Stdout => false,
        Output::Path(ref output) => {
            output_template::has_indexable_template(output.to_str().unwrap_or_default())
        }
    };

    let exported_page_ranges = command.exported_page_ranges();

    let exported_pages = document
        .pages
        .iter()
        .enumerate()
        .filter(|(i, _)| {
            exported_page_ranges.as_ref().map_or(true, |exported_page_ranges| {
                exported_page_ranges.includes_page_index(*i)
            })
        })
        .collect::<Vec<_>>();

    if !can_handle_multiple && exported_pages.len() > 1 {
        let err = match output {
            Output::Stdout => "to stdout",
            Output::Path(_) => {
                "without a page number template ({p}, {0p}) in the output path"
            }
        };
        bail!("cannot export multiple images {err}");
    }

    let cache = world.export_cache();

    // The results are collected in a `Vec<()>` which does not allocate.
    exported_pages
        .par_iter()
        .map(|(i, page)| {
            // Use output with converted path.
            let output = match output {
                Output::Path(ref path) => {
                    let storage;
                    let path = if can_handle_multiple {
                        storage = output_template::format(
                            path.to_str().unwrap_or_default(),
                            i + 1,
                            document.pages.len(),
                        );
                        Path::new(&storage)
                    } else {
                        path
                    };

                    // If we are not watching, don't use the cache.
                    // If the frame is in the cache, skip it.
                    // If the file does not exist, always create it.
                    if watching && cache.is_cached(*i, &page.frame) && path.exists() {
                        return Ok(());
                    }

                    Output::Path(path.to_owned())
                }
                Output::Stdout => Output::Stdout,
            };

            export_image_page(command, page, &output, fmt)?;
            Ok(())
        })
        .collect::<Result<Vec<()>, EcoString>>()?;

    Ok(())
}

mod output_template {
    const INDEXABLE: [&str; 3] = ["{p}", "{0p}", "{n}"];

    pub fn has_indexable_template(output: &str) -> bool {
        INDEXABLE.iter().any(|template| output.contains(template))
    }

    pub fn format(output: &str, this_page: usize, total_pages: usize) -> String {
        // Find the base 10 width of number `i`
        fn width(i: usize) -> usize {
            1 + i.checked_ilog10().unwrap_or(0) as usize
        }

        let other_templates = ["{t}"];
        INDEXABLE.iter().chain(other_templates.iter()).fold(
            output.to_string(),
            |out, template| {
                let replacement = match *template {
                    "{p}" => format!("{this_page}"),
                    "{0p}" | "{n}" => format!("{:01$}", this_page, width(total_pages)),
                    "{t}" => format!("{total_pages}"),
                    _ => unreachable!("unhandled template placeholder {template}"),
                };
                out.replace(template, replacement.as_str())
            },
        )
    }
}

/// Export single image.
fn export_image_page(
    command: &CompileCommand,
    page: &Page,
    output: &Output,
    fmt: ImageExportFormat,
) -> StrResult<()> {
    match fmt {
        ImageExportFormat::Png => {
            let pixmap = typst_render::render(page, command.ppi / 72.0);
            let buf = pixmap
                .encode_png()
                .map_err(|err| eco_format!("failed to encode PNG file ({err})"))?;
            output
                .write(&buf)
                .map_err(|err| eco_format!("failed to write PNG file ({err})"))?;
        }
        ImageExportFormat::Svg => {
            let svg = typst_svg::svg(page);
            output
                .write(svg.as_bytes())
                .map_err(|err| eco_format!("failed to write SVG file ({err})"))?;
        }
    }
    Ok(())
}

impl Output {
    fn write(&self, buffer: &[u8]) -> StrResult<()> {
        match self {
            Output::Stdout => std::io::stdout().write_all(buffer),
            Output::Path(path) => fs::write(path, buffer),
        }
        .map_err(|err| eco_format!("{err}"))
    }
}

/// Caches exported files so that we can avoid re-exporting them if they haven't
/// changed.
///
/// This is done by having a list of size `files.len()` that contains the hashes
/// of the last rendered frame in each file. If a new frame is inserted, this
/// will invalidate the rest of the cache, this is deliberate as to decrease the
/// complexity and memory usage of such a cache.
pub struct ExportCache {
    /// The hashes of last compilation's frames.
    pub cache: RwLock<Vec<u128>>,
}

impl ExportCache {
    /// Creates a new export cache.
    pub fn new() -> Self {
        Self { cache: RwLock::new(Vec::with_capacity(32)) }
    }

    /// Returns true if the entry is cached and appends the new hash to the
    /// cache (for the next compilation).
    pub fn is_cached(&self, i: usize, frame: &Frame) -> bool {
        let hash = typst::utils::hash128(frame);

        let mut cache = self.cache.upgradable_read();
        if i >= cache.len() {
            cache.with_upgraded(|cache| cache.push(hash));
            return false;
        }

        cache.with_upgraded(|cache| std::mem::replace(&mut cache[i], hash) == hash)
    }
}

/// Writes a Makefile rule describing the relationship between the output and
/// its dependencies to the path specified by the --make-deps argument, if it
/// was provided.
fn write_make_deps(world: &mut SystemWorld, command: &CompileCommand) -> StrResult<()> {
    let Some(ref make_deps_path) = command.make_deps else { return Ok(()) };
    let Output::Path(output_path) = command.output() else {
        bail!("failed to create make dependencies file because output was stdout")
    };
    let Ok(output_path) = output_path.into_os_string().into_string() else {
        bail!("failed to create make dependencies file because output path was not valid unicode")
    };

    // Based on `munge` in libcpp/mkdeps.cc from the GCC source code. This isn't
    // perfect as some special characters can't be escaped.
    fn munge(s: &str) -> String {
        let mut res = String::with_capacity(s.len());
        let mut slashes = 0;
        for c in s.chars() {
            match c {
                '\\' => slashes += 1,
                '$' => {
                    res.push('$');
                    slashes = 0;
                }
                ' ' | '\t' => {
                    // `munge`'s source contains a comment here that says: "A
                    // space or tab preceded by 2N+1 backslashes represents N
                    // backslashes followed by space..."
                    for _ in 0..slashes + 1 {
                        res.push('\\');
                    }
                    slashes = 0;
                }
                '#' => {
                    res.push('\\');
                    slashes = 0;
                }
                _ => slashes = 0,
            };
            res.push(c);
        }
        res
    }

    fn write(
        make_deps_path: &Path,
        output_path: String,
        root: PathBuf,
        dependencies: impl Iterator<Item = PathBuf>,
    ) -> io::Result<()> {
        let mut file = File::create(make_deps_path)?;

        file.write_all(munge(&output_path).as_bytes())?;
        file.write_all(b":")?;
        for dependency in dependencies {
            let Some(dependency) =
                dependency.strip_prefix(&root).unwrap_or(&dependency).to_str()
            else {
                // Silently skip paths that aren't valid unicode so we still
                // produce a rule that will work for the other paths that can be
                // processed.
                continue;
            };

            file.write_all(b" ")?;
            file.write_all(munge(dependency).as_bytes())?;
        }
        file.write_all(b"\n")?;

        Ok(())
    }

    write(make_deps_path, output_path, world.root().to_owned(), world.dependencies())
        .map_err(|err| {
            eco_format!("failed to create make dependencies file due to IO error ({err})")
        })
}

/// Opens the given file using:
/// - The default file viewer if `open` is `None`.
/// - The given viewer provided by `open` if it is `Some`.
///
/// If the file could not be opened, an error is returned.
fn open_file(open: Option<&str>, path: &Path) -> StrResult<()> {
    // Some resource openers require the path to be canonicalized.
    let path = path
        .canonicalize()
        .map_err(|err| eco_format!("failed to canonicalize path ({err})"))?;
    if let Some(app) = open {
        open::with_detached(&path, app)
            .map_err(|err| eco_format!("failed to open file with {} ({})", app, err))
    } else {
        open::that_detached(&path).map_err(|err| {
            let openers = open::commands(path)
                .iter()
                .map(|command| command.get_program().to_string_lossy())
                .collect::<Vec<_>>()
                .join(", ");
            eco_format!(
                "failed to open file with any of these resource openers: {} ({})",
                openers,
                err,
            )
        })
    }
}

/// Print diagnostic messages to the terminal.
pub fn print_diagnostics(
    world: &SystemWorld,
    errors: &[SourceDiagnostic],
    warnings: &[SourceDiagnostic],
    diagnostic_format: DiagnosticFormat,
) -> Result<(), codespan_reporting::files::Error> {
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

        term::emit(&mut terminal::out(), &config, world, &diag)?;

        // Stacktrace-like helper diagnostics.
        for point in &diagnostic.trace {
            let message = point.v.to_string();
            let help = Diagnostic::help()
                .with_message(message)
                .with_labels(label(world, point.span).into_iter().collect());

            term::emit(&mut terminal::out(), &config, world, &help)?;
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
