use std::ffi::OsStr;
use std::path::Path;

use chrono::{DateTime, Datelike, Timelike, Utc};
use codespan_reporting::diagnostic::{Diagnostic, Label};
use codespan_reporting::term;
use ecow::eco_format;
use parking_lot::RwLock;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use typst::WorldExt;
use typst::diag::{
    At, HintedStrResult, HintedString, Severity, SourceDiagnostic, SourceResult,
    StrResult, Warned, bail,
};
use typst::foundations::{Datetime, Smart};
use typst::layout::{Page, PageRanges, PagedDocument};
use typst::syntax::{FileId, Lines, Span};
use typst_html::HtmlDocument;
use typst_pdf::{PdfOptions, PdfStandards, Timestamp};

use crate::args::{
    CompileArgs, CompileCommand, DepsFormat, DiagnosticFormat, Input, Output,
    OutputFormat, PdfStandard, WatchCommand,
};
use crate::deps::write_deps;
#[cfg(feature = "http-server")]
use crate::server::HtmlServer;
use crate::timings::Timer;

use crate::watch::Status;
use crate::world::SystemWorld;
use crate::{set_failed, terminal};

type CodespanResult<T> = Result<T, CodespanError>;
type CodespanError = codespan_reporting::files::Error;

/// Execute a compilation command.
pub fn compile(timer: &mut Timer, command: &CompileCommand) -> HintedStrResult<()> {
    let mut config = CompileConfig::new(command)?;
    let mut world =
        SystemWorld::new(&command.args.input, &command.args.world, &command.args.process)
            .map_err(|err| eco_format!("{err}"))?;
    timer.record(&mut world, |world| compile_once(world, &mut config))?
}

/// A preprocessed `CompileCommand`.
pub struct CompileConfig {
    /// Static warnings to emit after compilation.
    pub warnings: Vec<HintedString>,
    /// Whether we are watching.
    pub watching: bool,
    /// Path to input Typst file or stdin.
    pub input: Input,
    /// Path to output file (PDF, PNG, SVG, or HTML).
    pub output: Output,
    /// The format of the output file.
    pub output_format: OutputFormat,
    /// Which pages to export.
    pub pages: Option<PageRanges>,
    /// The document's creation date formatted as a UNIX timestamp, with UTC suffix.
    pub creation_timestamp: Option<DateTime<Utc>>,
    /// The format to emit diagnostics in.
    pub diagnostic_format: DiagnosticFormat,
    /// Opens the output file with the default viewer or a specific program after
    /// compilation.
    pub open: Option<Option<String>>,
    /// A list of standards the PDF should conform to.
    pub pdf_standards: PdfStandards,
    /// Whether to write PDF (accessibility) tags.
    pub tagged: bool,
    /// A destination to write a list of dependencies to.
    pub deps: Option<Output>,
    /// The format to use for dependencies.
    pub deps_format: DepsFormat,
    /// The PPI (pixels per inch) to use for PNG export.
    pub ppi: f32,
    /// The export cache for images, used for caching output files in `typst
    /// watch` sessions with images.
    pub export_cache: ExportCache,
    /// Server for `typst watch` to HTML.
    #[cfg(feature = "http-server")]
    pub server: Option<HtmlServer>,
}

impl CompileConfig {
    /// Preprocess a `CompileCommand`, producing a compilation config.
    pub fn new(command: &CompileCommand) -> HintedStrResult<Self> {
        Self::new_impl(&command.args, None)
    }

    /// Preprocess a `WatchCommand`, producing a compilation config.
    pub fn watching(command: &WatchCommand) -> HintedStrResult<Self> {
        Self::new_impl(&command.args, Some(command))
    }

    /// The shared implementation of [`CompileConfig::new`] and
    /// [`CompileConfig::watching`].
    fn new_impl(
        args: &CompileArgs,
        watch: Option<&WatchCommand>,
    ) -> HintedStrResult<Self> {
        let mut warnings = Vec::new();
        let input = args.input.clone();

        let output_format = if let Some(specified) = args.format {
            specified
        } else if let Some(Output::Path(output)) = &args.output {
            match output.extension() {
                Some(ext) if ext.eq_ignore_ascii_case("pdf") => OutputFormat::Pdf,
                Some(ext) if ext.eq_ignore_ascii_case("png") => OutputFormat::Png,
                Some(ext) if ext.eq_ignore_ascii_case("svg") => OutputFormat::Svg,
                Some(ext) if ext.eq_ignore_ascii_case("html") => OutputFormat::Html,
                _ => bail!(
                    "could not infer output format for path {}.\n\
                     consider providing the format manually with `--format/-f`",
                    output.display()
                ),
            }
        } else {
            OutputFormat::Pdf
        };

        let output = args.output.clone().unwrap_or_else(|| {
            let Input::Path(path) = &input else {
                panic!("output must be specified when input is from stdin, as guarded by the CLI");
            };
            Output::Path(path.with_extension(
                match output_format {
                    OutputFormat::Pdf => "pdf",
                    OutputFormat::Png => "png",
                    OutputFormat::Svg => "svg",
                    OutputFormat::Html => "html",
                },
            ))
        });

        let pages = args.pages.as_ref().map(|export_ranges| {
            PageRanges::new(export_ranges.iter().map(|r| r.0.clone()).collect())
        });

        let tagged = !args.no_pdf_tags && pages.is_none();
        if output_format == OutputFormat::Pdf && pages.is_some() && !args.no_pdf_tags {
            warnings.push(
                HintedString::from("using --pages implies --no-pdf-tags").with_hints([
                    "the resulting PDF will be inaccessible".into(),
                    "add --no-pdf-tags to silence this warning".into(),
                ]),
            );
        }

        if !tagged {
            const ACCESSIBLE: &[(PdfStandard, &str)] = &[
                (PdfStandard::A_1a, "PDF/A-1a"),
                (PdfStandard::A_2a, "PDF/A-2a"),
                (PdfStandard::A_3a, "PDF/A-3a"),
                (PdfStandard::UA_1, "PDF/UA-1"),
            ];

            for (standard, name) in ACCESSIBLE {
                if args.pdf_standard.contains(standard) {
                    if args.no_pdf_tags {
                        bail!("cannot disable PDF tags when exporting a {name} document");
                    } else {
                        bail!(
                            "cannot disable PDF tags when exporting a {name} document";
                            hint: "using --pages implies --no-pdf-tags"
                        );
                    }
                }
            }
        }

        let pdf_standards = PdfStandards::new(
            &args.pdf_standard.iter().copied().map(Into::into).collect::<Vec<_>>(),
        )?;

        #[cfg(feature = "http-server")]
        let server = match watch {
            Some(command)
                if output_format == OutputFormat::Html && !command.server.no_serve =>
            {
                Some(HtmlServer::new(&input, &command.server)?)
            }
            _ => None,
        };

        let mut deps = args.deps.clone();
        let mut deps_format = args.deps_format;

        if let Some(path) = &args.make_deps
            && deps.is_none()
        {
            deps = Some(Output::Path(path.clone()));
            deps_format = DepsFormat::Make;
            warnings.push(
                HintedString::from("--make-deps is deprecated")
                    .with_hint("use --deps and --deps-format instead"),
            );
        }

        match (&output, &deps, watch) {
            (Output::Stdout, _, Some(_)) => {
                bail!("cannot write document to stdout in watch mode");
            }
            (_, Some(Output::Stdout), Some(_)) => {
                bail!("cannot write dependencies to stdout in watch mode")
            }
            (Output::Stdout, Some(Output::Stdout), _) => {
                bail!("cannot write both output and dependencies to stdout")
            }
            _ => {}
        }

        Ok(Self {
            warnings,
            watching: watch.is_some(),
            input,
            output,
            output_format,
            pages,
            pdf_standards,
            tagged,
            creation_timestamp: args.world.creation_timestamp,
            ppi: args.ppi,
            diagnostic_format: args.process.diagnostic_format,
            open: args.open.clone(),
            export_cache: ExportCache::new(),
            deps,
            deps_format,
            #[cfg(feature = "http-server")]
            server,
        })
    }
}

/// Compile a single time.
///
/// Returns whether it compiled without errors.
#[typst_macros::time(name = "compile once")]
pub fn compile_once(
    world: &mut SystemWorld,
    config: &mut CompileConfig,
) -> HintedStrResult<()> {
    let start = std::time::Instant::now();
    if config.watching {
        Status::Compiling.print(config).unwrap();
    }

    let Warned { output, mut warnings } = compile_and_export(world, config);

    // Add static warnings (for deprecated CLI flags and such).
    for warning in config.warnings.iter() {
        warnings.push(
            SourceDiagnostic::warning(Span::detached(), warning.message())
                .with_hints(warning.hints().iter().map(Into::into)),
        );
    }

    match &output {
        // Print success message and possibly warnings.
        Ok(_) => {
            let duration = start.elapsed();
            if config.watching {
                if warnings.is_empty() {
                    Status::Success(duration).print(config).unwrap();
                } else {
                    Status::PartialSuccess(duration).print(config).unwrap();
                }
            }

            print_diagnostics(world, &[], &warnings, config.diagnostic_format)
                .map_err(|err| eco_format!("failed to print diagnostics ({err})"))?;

            open_output(config)?;
        }

        // Print failure message and diagnostics.
        Err(errors) => {
            set_failed();

            if config.watching {
                Status::Error.print(config).unwrap();
            }

            print_diagnostics(world, errors, &warnings, config.diagnostic_format)
                .map_err(|err| eco_format!("failed to print diagnostics ({err})"))?;
        }
    }

    if let Some(dest) = &config.deps {
        write_deps(world, dest, config.deps_format, output.as_deref().ok())
            .map_err(|err| eco_format!("failed to create dependency file ({err})"))?;
    }

    Ok(())
}

/// Compile and then export the document.
fn compile_and_export(
    world: &mut SystemWorld,
    config: &mut CompileConfig,
) -> Warned<SourceResult<Vec<Output>>> {
    match config.output_format {
        OutputFormat::Html => {
            let Warned { output, warnings } = typst::compile::<HtmlDocument>(world);
            let result = output.and_then(|document| export_html(&document, config));
            Warned {
                output: result.map(|()| vec![config.output.clone()]),
                warnings,
            }
        }
        _ => {
            let Warned { output, warnings } = typst::compile::<PagedDocument>(world);
            let result = output.and_then(|document| export_paged(&document, config));
            Warned { output: result, warnings }
        }
    }
}

/// Export to HTML.
fn export_html(document: &HtmlDocument, config: &CompileConfig) -> SourceResult<()> {
    let html = typst_html::html(document)?;
    let result = config.output.write(html.as_bytes());

    #[cfg(feature = "http-server")]
    if let Some(server) = &config.server {
        server.update(html);
    }

    result
        .map_err(|err| eco_format!("failed to write HTML file ({err})"))
        .at(Span::detached())
}

/// Export to a paged target format.
fn export_paged(
    document: &PagedDocument,
    config: &CompileConfig,
) -> SourceResult<Vec<Output>> {
    match config.output_format {
        OutputFormat::Pdf => {
            export_pdf(document, config).map(|()| vec![config.output.clone()])
        }
        OutputFormat::Png => {
            export_image(document, config, ImageExportFormat::Png).at(Span::detached())
        }
        OutputFormat::Svg => {
            export_image(document, config, ImageExportFormat::Svg).at(Span::detached())
        }
        OutputFormat::Html => unreachable!(),
    }
}

/// Export to a PDF.
fn export_pdf(document: &PagedDocument, config: &CompileConfig) -> SourceResult<()> {
    // If the timestamp is provided through the CLI, use UTC suffix,
    // else, use the current local time and timezone.
    let timestamp = match config.creation_timestamp {
        Some(timestamp) => convert_datetime(timestamp).map(Timestamp::new_utc),
        None => {
            let local_datetime = chrono::Local::now();
            convert_datetime(local_datetime).and_then(|datetime| {
                Timestamp::new_local(
                    datetime,
                    local_datetime.offset().local_minus_utc() / 60,
                )
            })
        }
    };

    let options = PdfOptions {
        ident: Smart::Auto,
        timestamp,
        page_ranges: config.pages.clone(),
        standards: config.pdf_standards.clone(),
        tagged: config.tagged,
    };
    let buffer = typst_pdf::pdf(document, &options)?;
    config
        .output
        .write(&buffer)
        .map_err(|err| eco_format!("failed to write PDF file ({err})"))
        .at(Span::detached())?;
    Ok(())
}

/// Convert [`chrono::DateTime`] to [`Datetime`]
fn convert_datetime<Tz: chrono::TimeZone>(
    date_time: chrono::DateTime<Tz>,
) -> Option<Datetime> {
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
#[derive(Copy, Clone)]
enum ImageExportFormat {
    Png,
    Svg,
}

/// Export to one or multiple images.
fn export_image(
    document: &PagedDocument,
    config: &CompileConfig,
    fmt: ImageExportFormat,
) -> StrResult<Vec<Output>> {
    // Determine whether we have indexable templates in output
    let can_handle_multiple = match config.output {
        Output::Stdout => false,
        Output::Path(ref output) => {
            output_template::has_indexable_template(output.to_str().unwrap_or_default())
        }
    };

    let exported_pages = document
        .pages
        .iter()
        .enumerate()
        .filter(|(i, _)| {
            config.pages.as_ref().is_none_or(|exported_page_ranges| {
                exported_page_ranges.includes_page_index(*i)
            })
        })
        .collect::<Vec<_>>();

    if !can_handle_multiple && exported_pages.len() > 1 {
        let err = match config.output {
            Output::Stdout => "to stdout",
            Output::Path(_) => {
                "without a page number template ({p}, {0p}) in the output path"
            }
        };
        bail!("cannot export multiple images {err}");
    }

    // The results are collected in a `Vec<()>` which does not allocate.
    exported_pages
        .par_iter()
        .map(|(i, page)| {
            // Use output with converted path.
            let output = match &config.output {
                Output::Path(path) => {
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
                    if config.watching
                        && config.export_cache.is_cached(*i, page)
                        && path.exists()
                    {
                        return Ok(Output::Path(path.to_path_buf()));
                    }

                    Output::Path(path.to_owned())
                }
                Output::Stdout => Output::Stdout,
            };

            export_image_page(config, page, &output, fmt)?;
            Ok(output)
        })
        .collect::<StrResult<Vec<Output>>>()
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
    config: &CompileConfig,
    page: &Page,
    output: &Output,
    fmt: ImageExportFormat,
) -> StrResult<()> {
    match fmt {
        ImageExportFormat::Png => {
            let pixmap = typst_render::render(page, config.ppi / 72.0);
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
    pub fn is_cached(&self, i: usize, page: &Page) -> bool {
        let hash = typst::utils::hash128(page);

        let mut cache = self.cache.upgradable_read();
        if i >= cache.len() {
            cache.with_upgraded(|cache| cache.push(hash));
            return false;
        }

        cache.with_upgraded(|cache| std::mem::replace(&mut cache[i], hash) == hash)
    }
}

/// Opens the output if desired.
fn open_output(config: &mut CompileConfig) -> StrResult<()> {
    let Some(viewer) = config.open.take() else { return Ok(()) };

    #[cfg(feature = "http-server")]
    if let Some(server) = &config.server {
        let url = format!("http://{}", server.addr());
        return open_path(OsStr::new(&url), viewer.as_deref());
    }

    // Can't open stdout.
    let Output::Path(path) = &config.output else { return Ok(()) };

    // Some resource openers require the path to be canonicalized.
    let path = path
        .canonicalize()
        .map_err(|err| eco_format!("failed to canonicalize path ({err})"))?;

    open_path(path.as_os_str(), viewer.as_deref())
}

/// Opens the given file using:
///
/// - The default file viewer if `app` is `None`.
/// - The given viewer provided by `app` if it is `Some`.
fn open_path(path: &OsStr, viewer: Option<&str>) -> StrResult<()> {
    if let Some(viewer) = viewer {
        open::with_detached(path, viewer)
            .map_err(|err| eco_format!("failed to open file with {} ({})", viewer, err))
    } else {
        open::that_detached(path).map_err(|err| {
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
    type Source = Lines<String>;

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

impl From<PdfStandard> for typst_pdf::PdfStandard {
    fn from(standard: PdfStandard) -> Self {
        match standard {
            PdfStandard::V_1_4 => typst_pdf::PdfStandard::V_1_4,
            PdfStandard::V_1_5 => typst_pdf::PdfStandard::V_1_5,
            PdfStandard::V_1_6 => typst_pdf::PdfStandard::V_1_6,
            PdfStandard::V_1_7 => typst_pdf::PdfStandard::V_1_7,
            PdfStandard::V_2_0 => typst_pdf::PdfStandard::V_2_0,
            PdfStandard::A_1b => typst_pdf::PdfStandard::A_1b,
            PdfStandard::A_1a => typst_pdf::PdfStandard::A_1a,
            PdfStandard::A_2b => typst_pdf::PdfStandard::A_2b,
            PdfStandard::A_2u => typst_pdf::PdfStandard::A_2u,
            PdfStandard::A_2a => typst_pdf::PdfStandard::A_2a,
            PdfStandard::A_3b => typst_pdf::PdfStandard::A_3b,
            PdfStandard::A_3u => typst_pdf::PdfStandard::A_3u,
            PdfStandard::A_3a => typst_pdf::PdfStandard::A_3a,
            PdfStandard::A_4 => typst_pdf::PdfStandard::A_4,
            PdfStandard::A_4f => typst_pdf::PdfStandard::A_4f,
            PdfStandard::A_4e => typst_pdf::PdfStandard::A_4e,
            PdfStandard::UA_1 => typst_pdf::PdfStandard::Ua_1,
        }
    }
}
