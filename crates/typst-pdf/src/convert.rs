use ecow::{EcoVec, eco_format};
use indexmap::IndexMap;
use krilla::configure::{Configuration, ValidationError, Validator};
use krilla::destination::NamedDestination;
use krilla::embed::EmbedError;
use krilla::error::KrillaError;
use krilla::geom::PathBuilder;
use krilla::page::{PageLabel, PageSettings};
use krilla::pdf::PdfError;
use krilla::surface::Surface;
use krilla::{Document, SerializeSettings};
use krilla_svg::render_svg_glyph;
use rustc_hash::{FxBuildHasher, FxHashMap, FxHashSet};
use smallvec::SmallVec;
use typst_library::diag::{
    At, ExpectInternal, SourceDiagnostic, SourceResult, bail, error,
};
use typst_library::foundations::{NativeElement, Repr};
use typst_library::introspection::{Location, Tag};
use typst_library::layout::{
    Frame, FrameItem, GroupItem, PagedDocument, Size, Transform,
};
use typst_library::model::HeadingElem;
use typst_library::text::Font;
use typst_library::visualize::{Geometry, Paint};
use typst_syntax::Span;

use crate::PdfOptions;
use crate::attach::attach_files;
use crate::image::handle_image;
use crate::link::{LinkAnnotation, handle_link};
use crate::metadata::build_metadata;
use crate::outline::build_outline;
use crate::page::PageLabelExt;
use crate::shape::handle_shape;
use crate::tags::{self, GroupId, Tags};
use crate::text::handle_text;
use crate::util::{AbsExt, TransformExt, convert_path, display_font};

#[typst_macros::time(name = "convert document")]
pub fn convert(
    typst_document: &PagedDocument,
    options: &PdfOptions,
) -> SourceResult<Vec<u8>> {
    let settings = SerializeSettings {
        compress_content_streams: true,
        no_device_cs: true,
        ascii_compatible: false,
        xmp_metadata: true,
        cmyk_profile: None,
        configuration: options.standards.config,
        enable_tagging: options.tagged,
        render_svg_glyph_fn: render_svg_glyph,
    };

    let mut document = Document::new_with(settings);
    let page_index_converter = PageIndexConverter::new(typst_document, options);
    let named_destinations =
        collect_named_destinations(typst_document, &page_index_converter);
    let tags = tags::init(typst_document, options)?;

    let mut gc = GlobalContext::new(
        typst_document,
        options,
        named_destinations,
        page_index_converter,
        tags,
    );

    convert_pages(&mut gc, &mut document)?;
    attach_files(&gc, &mut document)?;
    let (doc_lang, tree) = tags::resolve(&mut gc)?;

    document.set_outline(build_outline(&gc));
    document.set_metadata(build_metadata(&gc, doc_lang));
    document.set_tag_tree(tree);

    finish(document, gc, options.standards.config)
}

fn convert_pages(gc: &mut GlobalContext, document: &mut Document) -> SourceResult<()> {
    for (i, typst_page) in gc.document.pages.iter().enumerate() {
        if gc.page_index_converter.pdf_page_index(i).is_none() {
            // Don't export this page.
            continue;
        }

        // PDF 1.4 upwards to 1.7 specifies a minimum page size of 3x3 units.
        // PDF 2.0 doesn't define an explicit limit, but krilla and probably
        // some viewers won't handle pages that have zero sized pages.
        let mut settings = PageSettings::from_wh(
            typst_page.frame.width().to_f32().max(3.0),
            typst_page.frame.height().to_f32().max(3.0),
        )
        .expect_internal("invalid page size")
        .at(Span::detached())?;

        if let Some(label) = typst_page
            .numbering
            .as_ref()
            .and_then(|num| PageLabel::generate(num, typst_page.number))
            .or_else(|| {
                // When some pages were ignored from export, we show a page label with
                // the correct real (not logical) page number.
                // This is for consistency with normal output when pages have no numbering
                // and all are exported: the final PDF page numbers always correspond to
                // the real (not logical) page numbers. Here, the final PDF page number
                // will differ, but we can at least use labels to indicate what was
                // the corresponding real page number in the Typst document.
                gc.page_index_converter
                    .has_skipped_pages()
                    .then(|| PageLabel::arabic((i + 1) as u64))
            })
        {
            settings = settings.with_page_label(label);
        }

        let mut page = document.start_page_with(settings);
        let mut surface = page.surface();
        let page_idx = gc.page_index_converter.pdf_page_index(i);
        let mut fc = FrameContext::new(page_idx, typst_page.frame.size());

        tags::page(gc, &mut surface, |gc, surface| {
            handle_frame(
                &mut fc,
                &typst_page.frame,
                typst_page.fill_or_transparent(),
                surface,
                gc,
            )
        })?;

        surface.finish();

        let link_annotations = fc.link_annotations.into_values().flatten();
        tags::add_link_annotations(gc, &mut page, link_annotations);
    }

    Ok(())
}

/// A state allowing us to keep track of transforms and container sizes,
/// which is mainly needed to resolve gradients and patterns correctly.
#[derive(Debug, Clone)]
pub(crate) struct State {
    /// The current transform.
    transform: Transform,
    /// The transform of first hard frame in the hierarchy.
    container_transform: Transform,
    /// The size of the first hard frame in the hierarchy.
    container_size: Size,
}

impl State {
    /// Creates a new, clean state for a given `size`.
    fn new(size: Size) -> Self {
        Self {
            transform: Transform::identity(),
            container_transform: Transform::identity(),
            container_size: size,
        }
    }

    pub(crate) fn register_container(&mut self, size: Size) {
        self.container_transform = self.transform;
        self.container_size = size;
    }

    pub(crate) fn pre_concat(&mut self, transform: Transform) {
        self.transform = self.transform.pre_concat(transform);
    }

    pub(crate) fn transform(&self) -> Transform {
        self.transform
    }

    pub(crate) fn container_transform(&self) -> Transform {
        self.container_transform
    }

    pub(crate) fn container_size(&self) -> Size {
        self.container_size
    }
}

/// Context needed for converting a single frame.
pub(crate) struct FrameContext {
    /// The logical page index. This might be `None` if the page isn't exported,
    /// of if the FrameContext has been built to convert a pattern.
    pub(crate) page_idx: Option<usize>,
    states: Vec<State>,
    /// The link annotations belonging to a Link tag.
    link_annotations: IndexMap<GroupId, SmallVec<[LinkAnnotation; 1]>, FxBuildHasher>,
}

impl FrameContext {
    pub(crate) fn new(page_idx: Option<usize>, size: Size) -> Self {
        Self {
            page_idx,
            states: vec![State::new(size)],
            link_annotations: IndexMap::default(),
        }
    }

    pub(crate) fn push(&mut self) {
        self.states.push(self.states.last().unwrap().clone());
    }

    pub(crate) fn pop(&mut self) {
        self.states.pop();
    }

    pub(crate) fn state(&self) -> &State {
        self.states.last().unwrap()
    }

    pub(crate) fn state_mut(&mut self) -> &mut State {
        self.states.last_mut().unwrap()
    }

    pub(crate) fn get_link_annotation(
        &mut self,
        id: GroupId,
    ) -> Option<&mut LinkAnnotation> {
        self.link_annotations.get_mut(&id)?.last_mut()
    }

    pub(crate) fn push_link_annotation(
        &mut self,
        id: GroupId,
        annotation: LinkAnnotation,
    ) {
        let annotations = self.link_annotations.entry(id).or_default();
        annotations.push(annotation);
    }
}

/// Globally needed context for converting a Typst document.
pub(crate) struct GlobalContext<'a> {
    /// Cache the conversion between krilla and Typst fonts (forward and backward).
    pub(crate) fonts_forward: FxHashMap<Font, krilla::text::Font>,
    pub(crate) fonts_backward: FxHashMap<krilla::text::Font, Font>,
    /// Mapping between images and their span.
    // Note: In theory, the same image can have multiple spans
    // if it appears in the document multiple times. We just store the
    // first appearance, though.
    pub(crate) image_to_spans: FxHashMap<krilla::image::Image, Span>,
    /// The spans of all images that appear in the document. We use this so
    /// we can give more accurate error messages.
    pub(crate) image_spans: FxHashSet<Span>,
    /// The document to convert.
    pub(crate) document: &'a PagedDocument,
    /// Options for PDF export.
    pub(crate) options: &'a PdfOptions<'a>,
    /// Mapping between locations in the document and named destinations.
    pub(crate) loc_to_names: FxHashMap<Location, NamedDestination>,
    pub(crate) page_index_converter: PageIndexConverter,
    /// Tagged PDF context.
    pub(crate) tags: Tags,
}

impl<'a> GlobalContext<'a> {
    pub(crate) fn new(
        document: &'a PagedDocument,
        options: &'a PdfOptions,
        loc_to_names: FxHashMap<Location, NamedDestination>,
        page_index_converter: PageIndexConverter,
        tags: Tags,
    ) -> GlobalContext<'a> {
        Self {
            fonts_forward: FxHashMap::default(),
            fonts_backward: FxHashMap::default(),
            document,
            options,
            loc_to_names,
            image_to_spans: FxHashMap::default(),
            image_spans: FxHashSet::default(),
            page_index_converter,
            tags,
        }
    }
}

#[typst_macros::time(name = "handle page")]
pub(crate) fn handle_frame(
    fc: &mut FrameContext,
    frame: &Frame,
    fill: Option<Paint>,
    surface: &mut Surface,
    gc: &mut GlobalContext,
) -> SourceResult<()> {
    fc.push();

    if frame.kind().is_hard() {
        fc.state_mut().register_container(frame.size());
    }

    if let Some(fill) = fill {
        let shape = Geometry::Rect(frame.size()).filled(fill);
        handle_shape(fc, &shape, surface, gc, Span::detached())?;
    }

    for (point, item) in frame.items() {
        fc.push();
        fc.state_mut().pre_concat(Transform::translate(point.x, point.y));

        match item {
            FrameItem::Group(g) => handle_group(fc, g, surface, gc)?,
            FrameItem::Text(t) => handle_text(fc, t, surface, gc)?,
            FrameItem::Shape(s, span) => handle_shape(fc, s, surface, gc, *span)?,
            FrameItem::Image(image, size, span) => {
                handle_image(gc, fc, image, *size, surface, *span)?
            }
            FrameItem::Link(dest, size) => handle_link(fc, gc, dest, *size)?,
            FrameItem::Tag(Tag::Start(_, flags)) => {
                if flags.tagged {
                    tags::handle_start(gc, surface);
                }
            }
            FrameItem::Tag(Tag::End(_, _, flags)) => {
                if flags.tagged {
                    tags::handle_end(gc, surface);
                }
            }
        }

        fc.pop();
    }

    fc.pop();

    Ok(())
}

pub(crate) fn handle_group(
    fc: &mut FrameContext,
    group: &GroupItem,
    surface: &mut Surface,
    gc: &mut GlobalContext,
) -> SourceResult<()> {
    fc.push();
    fc.state_mut().pre_concat(group.transform);

    tags::group(gc, surface, group.parent, |gc, surface| -> SourceResult<()> {
        let clip_path = group
            .clip
            .as_ref()
            .and_then(|p| {
                let mut builder = PathBuilder::new();
                convert_path(p, &mut builder);
                builder.finish()
            })
            .and_then(|p| p.transform(fc.state().transform.to_krilla()));

        if let Some(clip_path) = &clip_path {
            surface.push_clip_path(clip_path, &krilla::paint::FillRule::NonZero);
        }

        let res = handle_frame(fc, &group.frame, None, surface, gc);

        if clip_path.is_some() {
            surface.pop();
        }

        res
    })?;

    fc.pop();

    Ok(())
}

/// Finish a krilla document and handle export errors.
#[typst_macros::time(name = "finish export")]
fn finish(
    document: Document,
    gc: GlobalContext,
    configuration: Configuration,
) -> SourceResult<Vec<u8>> {
    let validator = configuration.validator();

    match document.finish() {
        Ok(r) => Ok(r),
        Err(e) => match e {
            KrillaError::Font(f, err) => {
                bail!(
                    Span::detached(),
                    "failed to process {} ({err})",
                    display_font(gc.fonts_backward.get(&f));
                    hint: "make sure the font is valid";
                    hint: "the used font might be unsupported by Typst";
                );
            }
            KrillaError::Validation(ve) => {
                let errors = ve
                    .iter()
                    .map(|e| convert_error(&gc, validator, e))
                    .collect::<EcoVec<_>>();
                Err(errors)
            }
            KrillaError::Image(_, loc, err) => {
                let span = to_span(loc);
                bail!(span, "failed to process image ({err})");
            }
            KrillaError::SixteenBitImage(image, _) => {
                let span = gc.image_to_spans.get(&image).unwrap();
                bail!(
                    *span, "16 bit images are not supported in this export mode";
                    hint: "convert the image to 8 bit instead";
                )
            }
            KrillaError::Pdf(_, e, loc) => {
                let span = to_span(loc);
                match e {
                    // We already validated in `typst-library` that the page index is valid.
                    PdfError::InvalidPage(_) => bail!(
                        span,
                        "invalid page number for PDF file";
                        hint: "please report this as a bug";
                    ),
                    PdfError::VersionMismatch(v) => {
                        let pdf_ver = v.as_str();
                        let config_ver = configuration.version();
                        let cur_ver = config_ver.as_str();
                        bail!(span,
                            "the version of the PDF is too high";
                            hint: "the current export target is {cur_ver}, while the PDF \
                                   has version {pdf_ver}";
                            hint: "raise the export target to {pdf_ver} or higher";
                            hint: "or preprocess the PDF to convert it to a lower version";
                        );
                    }
                }
            }
            KrillaError::DuplicateTagId(_, loc) => {
                let span = to_span(loc);
                bail!(span,
                    "duplicate tag id";
                    hint: "please report this as a bug";
                );
            }
            KrillaError::UnknownTagId(_, loc) => {
                let span = to_span(loc);
                bail!(span,
                    "unknown tag id";
                    hint: "please report this as a bug";
                );
            }
        },
    }
}

/// Converts a krilla error into a Typst error.
fn convert_error(
    gc: &GlobalContext,
    validator: Validator,
    error: &ValidationError,
) -> SourceDiagnostic {
    let prefix = eco_format!("{} error:", validator.as_str());
    match error {
        ValidationError::TooLongString => error!(
            Span::detached(),
            "{prefix} a PDF string is longer than 32767 characters";
            hint: "ensure title and author names are short enough";
        ),
        // Should in theory never occur, as krilla always trims font names.
        ValidationError::TooLongName => error!(
            Span::detached(),
            "{prefix} a PDF name is longer than 127 characters";
            hint: "perhaps a font name is too long";
        ),

        ValidationError::TooLongArray => error!(
            Span::detached(),
            "{prefix} a PDF array is longer than 8191 elements";
            hint: "this can happen if you have a very long text in a single line";
        ),
        ValidationError::TooLongDictionary => error!(
            Span::detached(),
            "{prefix} a PDF dictionary has more than 4095 entries";
            hint: "try reducing the complexity of your document";
        ),
        ValidationError::TooLargeFloat => error!(
            Span::detached(),
            "{prefix} a PDF floating point number is larger than the allowed limit";
            hint: "try exporting with a higher PDF version";
        ),
        ValidationError::TooManyIndirectObjects => error!(
            Span::detached(),
            "{prefix} the PDF has too many indirect objects";
            hint: "reduce the size of your document";
        ),
        // Can only occur if we have 27+ nested clip paths
        ValidationError::TooHighQNestingLevel => error!(
            Span::detached(),
            "{prefix} the PDF has too high q nesting";
            hint: "reduce the number of nested containers";
        ),
        ValidationError::ContainsPostScript(loc) => error!(
            to_span(*loc),
            "{prefix} the PDF contains PostScript code";
            hint: "conic gradients are not supported in this PDF standard";
        ),
        ValidationError::MissingCMYKProfile => error!(
            Span::detached(),
            "{prefix} the PDF is missing a CMYK profile";
            hint: "CMYK colors are not yet supported in this export mode";
        ),
        ValidationError::ContainsNotDefGlyph(f, loc, text) => error!(
            to_span(*loc),
            "{prefix} the text `{}` could not be displayed with {}",
            text.repr(),
            display_font(gc.fonts_backward.get(f));
            hint: "try using a different font";
        ),
        ValidationError::NoCodepointMapping(_, _, loc) => {
            let msg = if loc.is_some() {
                "the text was not mapped to a code point"
            } else {
                "the PDF contains text with missing codepoints"
            };
            error!(
                to_span(*loc),
                "{prefix} {msg}";
                hint: "for complex scripts like Arabic, it might not be \
                       possible to produce a compliant document";
            )
        }
        ValidationError::InvalidCodepointMapping(_, _, c, loc) => {
            let msg = if loc.is_some() {
                "the text contains"
            } else {
                "the PDF contains text with"
            };
            error!(
                to_span(*loc),
                "{prefix} {msg} the disallowed codepoint `{}`",
                c.repr(),
            )
        }
        ValidationError::UnicodePrivateArea(_, _, c, loc) => {
            let msg = if loc.is_some() { "the PDF" } else { "the text" };
            error!(
                to_span(*loc),
                "{prefix} {msg} contains the codepoint `{}`", c.repr();
                hint: "codepoints from the Unicode private area are \
                       forbidden in this export mode";
            )
        }
        ValidationError::RestrictedLicense(f) => error!(
            Span::detached(),
            "{prefix} license of {} is too restrictive",
            display_font(gc.fonts_backward.get(f));
            hint: "the font has specified \"Restricted License embedding\" in its \
                   metadata";
            hint: "restrictive font licenses are prohibited by {} because they limit \
                   the suitability for archival",
            validator.as_str();
        ),
        ValidationError::Transparency(loc) => {
            let span = to_span(*loc);
            let hint1 = "try exporting with a different standard that \
                         supports transparency";
            if loc.is_some() {
                if gc.image_spans.contains(&span) {
                    error!(
                        span, "{prefix} the image contains transparency";
                        hint: "{hint1}";
                        hint: "or convert the image to a non-transparent one";
                        hint: "you might have to convert SVGs into \
                               non-transparent bitmap images";
                    )
                } else {
                    error!(
                        span, "{prefix} the used fill or stroke has transparency";
                        hint: "{hint1}";
                        hint: "or don't use colors with transparency in \
                               this export mode";
                    )
                }
            } else {
                error!(
                    span, "{prefix} the PDF contains transparency";
                    hint: "{hint1}";
                )
            }
        }
        ValidationError::ImageInterpolation(loc) => {
            let span = to_span(*loc);
            if loc.is_some() {
                error!(
                    span, "{prefix} the image has smooth scaling";
                    hint: "set the `scaling` attribute to `pixelated`";
                )
            } else {
                error!(
                    span, "{prefix} an image in the PDF has smooth scaling";
                    hint: "set the `scaling` attribute of all images to `pixelated`";
                )
            }
        }
        ValidationError::EmbeddedFile(e, s) => {
            // We always set the span for attached files, so it cannot be detached.
            let span = to_span(*s);
            match e {
                EmbedError::Existence => {
                    error!(
                        span, "{prefix} document contains an attached file";
                        hint: "file attachments are not supported in this export mode";
                    )
                }
                EmbedError::MissingDate => {
                    error!(
                        span, "{prefix} document date is missing";
                        hint: "the document must have a date when attaching files";
                        hint: "`set document(date: none)` must not be used in this case";
                    )
                }
                EmbedError::MissingDescription => {
                    error!(span, "{prefix} the file description is missing")
                }
                EmbedError::MissingMimeType => {
                    error!(span, "{prefix} the file mime type is missing")
                }
            }
        }
        // The below errors cannot occur yet, only once Typst supports full PDF/A
        // and PDF/UA. But let's still add a message just to be on the safe side.
        ValidationError::MissingAnnotationAltText(loc) => {
            let span = to_span(*loc);
            error!(
                span, "{prefix} missing annotation alt text";
                hint: "please report this as a bug";
            )
        }
        ValidationError::MissingAltText(loc) => {
            let span = to_span(*loc);
            error!(
                span, "{prefix} missing alt text";
                hint: "make sure your images and equations have alt text";
            )
        }
        ValidationError::NoDocumentLanguage => error!(
            Span::detached(),
            "{prefix} missing document language";
            hint: "set the language of the document";
        ),
        // Needs to be set by typst-pdf.
        ValidationError::MissingHeadingTitle => error!(
            Span::detached(),
            "{prefix} missing heading title";
            hint: "please report this as a bug";
        ),
        ValidationError::MissingDocumentOutline => error!(
            Span::detached(),
            "{prefix} missing document outline";
            hint: "please report this as a bug";
        ),
        ValidationError::MissingTagging => error!(
            Span::detached(),
            "{prefix} missing document tags";
            hint: "please report this as a bug";
        ),
        ValidationError::NoDocumentTitle => error!(
            Span::detached(),
            "{prefix} missing document title";
            hint: "set the title with `set document(title: [...])`";
        ),
        ValidationError::MissingDocumentDate => error!(
            Span::detached(),
            "{prefix} missing document date";
            hint: "set the date of the document";
        ),
        ValidationError::EmbeddedPDF(loc) => {
            error!(
                to_span(*loc),
                "embedding PDFs is currently not supported in this export mode";
                hint: "try converting the PDF to an SVG before embedding it";
            )
        }
    }
}

/// Convert a krilla location to a span.
pub(crate) fn to_span(loc: Option<krilla::surface::Location>) -> Span {
    loc.map(Span::from_raw).unwrap_or(Span::detached())
}

fn collect_named_destinations(
    document: &PagedDocument,
    pic: &PageIndexConverter,
) -> FxHashMap<Location, NamedDestination> {
    let mut locs_to_names = FxHashMap::default();

    // Find all headings that have a label and are the first among other
    // headings with the same label.
    let matches: Vec<_> = {
        let mut seen = FxHashSet::default();
        document
            .introspector
            .query(&HeadingElem::ELEM.select())
            .iter()
            .filter_map(|elem| elem.location().zip(elem.label()))
            .filter(|&(_, label)| seen.insert(label))
            .collect()
    };

    for (loc, label) in matches {
        // Only add named destination if page belonging to the position is exported.
        let pos = document.introspector.position(loc);
        if let Some(dest) = crate::link::pos_to_xyz(pic, pos) {
            let named = NamedDestination::new(label.resolve().to_string(), dest);
            locs_to_names.insert(loc, named);
        }
    }

    locs_to_names
}

pub(crate) struct PageIndexConverter {
    page_indices: FxHashMap<usize, usize>,
    skipped_pages: usize,
}

impl PageIndexConverter {
    pub fn new(document: &PagedDocument, options: &PdfOptions) -> Self {
        let mut page_indices = FxHashMap::default();
        let mut skipped_pages = 0;

        for i in 0..document.pages.len() {
            if options
                .page_ranges
                .as_ref()
                .is_some_and(|ranges| !ranges.includes_page_index(i))
            {
                skipped_pages += 1;
            } else {
                page_indices.insert(i, i - skipped_pages);
            }
        }

        Self { page_indices, skipped_pages }
    }

    pub(crate) fn has_skipped_pages(&self) -> bool {
        self.skipped_pages > 0
    }

    /// Get the PDF page index of a page index, if it's not excluded.
    pub(crate) fn pdf_page_index(&self, page_index: usize) -> Option<usize> {
        self.page_indices.get(&page_index).copied()
    }
}
