use std::collections::{BTreeMap, HashMap, HashSet};
use std::num::NonZeroU64;

use ecow::EcoVec;
use krilla::annotation::Annotation;
use krilla::configure::{Configuration, ValidationError};
use krilla::destination::{NamedDestination, XyzDestination};
use krilla::embed::EmbedError;
use krilla::error::KrillaError;
use krilla::geom::PathBuilder;
use krilla::page::{PageLabel, PageSettings};
use krilla::surface::Surface;
use krilla::{Document, SerializeSettings};
use krilla_svg::render_svg_glyph;
use typst_library::diag::{bail, error, SourceResult};
use typst_library::foundations::NativeElement;
use typst_library::introspection::Location;
use typst_library::layout::{
    Abs, Frame, FrameItem, GroupItem, PagedDocument, Size, Transform,
};
use typst_library::model::HeadingElem;
use typst_library::text::{Font, Lang};
use typst_library::visualize::{Geometry, Paint};
use typst_syntax::Span;

use crate::embed::embed_files;
use crate::image::handle_image;
use crate::link::handle_link;
use crate::metadata::build_metadata;
use crate::outline::build_outline;
use crate::page::PageLabelExt;
use crate::shape::handle_shape;
use crate::text::handle_text;
use crate::util::{convert_path, display_font, AbsExt, TransformExt};
use crate::PdfOptions;

pub fn convert(
    typst_document: &PagedDocument,
    options: &PdfOptions,
) -> SourceResult<Vec<u8>> {
    let configuration = get_configuration(options)?;

    let settings = SerializeSettings {
        compress_content_streams: true,
        no_device_cs: true,
        ascii_compatible: false,
        xmp_metadata: true,
        cmyk_profile: None,
        configuration,
        enable_tagging: false,
        render_svg_glyph_fn: render_svg_glyph,
    };

    let mut document = Document::new_with(settings);
    let page_index_converter = PageIndexConverter::new(typst_document, options);
    let named_destinations =
        collect_named_destinations(typst_document, &page_index_converter);
    let mut gc = GlobalContext::new(
        typst_document,
        options,
        named_destinations,
        page_index_converter,
    );

    convert_pages(&mut gc, &mut document)?;
    embed_files(typst_document, &mut document)?;

    document.set_outline(build_outline(&gc));
    document.set_metadata(build_metadata(&gc));

    finish(document, gc, configuration)
}

fn convert_pages(gc: &mut GlobalContext, document: &mut Document) -> SourceResult<()> {
    for (i, typst_page) in gc.document.pages.iter().enumerate() {
        if gc.page_index_converter.pdf_page_index(i).is_none() {
            // Don't export this page.
            continue;
        } else {
            let mut settings = PageSettings::new(
                typst_page.frame.width().to_f32(),
                typst_page.frame.height().to_f32(),
            );

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
                        .then(|| PageLabel::arabic(i + 1))
                })
            {
                settings = settings.with_page_label(label);
            }

            let mut page = document.start_page_with(settings);
            let mut surface = page.surface();
            let mut fc = FrameContext::new(typst_page.frame.size());

            handle_frame(
                &mut fc,
                &typst_page.frame,
                typst_page.fill_or_transparent(),
                &mut surface,
                gc,
            )?;

            surface.finish();

            for annotation in fc.annotations {
                page.add_annotation(annotation);
            }
        }
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
    states: Vec<State>,
    annotations: Vec<Annotation>,
}

impl FrameContext {
    pub(crate) fn new(size: Size) -> Self {
        Self {
            states: vec![State::new(size)],
            annotations: vec![],
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

    pub(crate) fn push_annotation(&mut self, annotation: Annotation) {
        self.annotations.push(annotation);
    }
}

/// Globally needed context for converting a typst document.
pub(crate) struct GlobalContext<'a> {
    /// Cache the conversion between krilla and Typst fonts (forward and backward).
    pub(crate) fonts_forward: HashMap<Font, krilla::text::Font>,
    pub(crate) fonts_backward: HashMap<krilla::text::Font, Font>,
    /// Mapping between images and their span.
    // Note: In theory, the same image can have multiple spans
    // if it appears in the document multiple times. We just store the
    // first appearance, though.
    pub(crate) image_to_spans: HashMap<krilla::image::Image, Span>,
    /// The spans of all images that appear in the document. We use this so
    /// we can give more accurate error messages.
    pub(crate) image_spans: HashSet<Span>,
    /// The document to convert.
    pub(crate) document: &'a PagedDocument,
    /// Options for PDF export.
    pub(crate) options: &'a PdfOptions<'a>,
    /// Mapping between locations in the document and named destinations.
    pub(crate) loc_to_names: HashMap<Location, NamedDestination>,
    /// The languages used throughout the document.
    pub(crate) languages: BTreeMap<Lang, usize>,
    pub(crate) page_index_converter: PageIndexConverter,
}

impl<'a> GlobalContext<'a> {
    pub(crate) fn new(
        document: &'a PagedDocument,
        options: &'a PdfOptions,
        loc_to_names: HashMap<Location, NamedDestination>,
        page_index_converter: PageIndexConverter,
    ) -> GlobalContext<'a> {
        Self {
            fonts_forward: HashMap::new(),
            fonts_backward: HashMap::new(),
            document,
            options,
            loc_to_names,
            image_to_spans: HashMap::new(),
            image_spans: HashSet::new(),
            languages: BTreeMap::new(),
            page_index_converter,
        }
    }
}

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
            FrameItem::Link(d, s) => handle_link(fc, gc, d, *s),
            FrameItem::Tag(_) => {}
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
    context: &mut GlobalContext,
) -> SourceResult<()> {
    fc.push();
    fc.state_mut().pre_concat(group.transform);

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

    handle_frame(fc, &group.frame, None, surface, context)?;

    if clip_path.is_some() {
        surface.pop();
    }

    fc.pop();

    Ok(())
}

/// Finish a krilla document and handle export errors.
fn finish(
    document: Document,
    gc: GlobalContext,
    configuration: Configuration,
) -> SourceResult<Vec<u8>> {
    let validator = configuration.validator();

    match document.finish() {
        Ok(r) => Ok(r),
        Err(e) => match e {
            KrillaError::Font(f, s) => {
                let font_str = display_font(gc.fonts_backward.get(&f).unwrap());
                bail!(Span::detached(), "failed to process font {font_str}: {s}";
                    hint: "make sure the font is valid";
                    hint: "the used font might be unsupported by Typst"
                );
            }
            KrillaError::Validation(ve) => {
                let prefix = format!("{} error:", validator.as_str());

                let get_span = |loc: Option<krilla::surface::Location>| {
                    loc.map(|l| Span::from_raw(NonZeroU64::new(l).unwrap()))
                        .unwrap_or(Span::detached())
                };

                let errors = ve.iter().map(|e| {
                    match e {
                        ValidationError::TooLongString => {
                            error!(Span::detached(), "{prefix} a PDF string is longer than \
                            32767 characters";
                            hint: "ensure title and author names are short enough")
                        }
                        // Should in theory never occur, as krilla always trims font names.
                        ValidationError::TooLongName => {
                            error!(Span::detached(), "{prefix} a PDF name is longer than \
                            127 characters";
                            hint: "perhaps a font name is too long")
                        }
                        ValidationError::TooLongArray => {
                            error!(Span::detached(), "{prefix} a PDF array is longer than \
                            8191 elements";
                            hint: "this can happen if you have a very long text in a single line")
                        }
                        ValidationError::TooLongDictionary => {
                            error!(Span::detached(), "{prefix} a PDF dictionary has more than \
                            4095 entries";
                            hint: "try reducing the complexity of your document")
                        }
                        ValidationError::TooLargeFloat => {
                            error!(Span::detached(), "{prefix} a PDF floating point number is larger \
                            than the allowed limit";
                            hint: "try exporting using a higher PDF version")
                        }
                        ValidationError::TooManyIndirectObjects => {
                            error!(Span::detached(), "{prefix} the PDF has too many indirect objects";
                            hint: "reduce the size of your document")
                        }
                        // Can only occur if we have 27+ nested clip paths
                        ValidationError::TooHighQNestingLevel => {
                            error!(Span::detached(), "{prefix} the PDF has too high q nesting";
                            hint: "reduce the number of nested containers")
                        }
                        ValidationError::ContainsPostScript(loc) => {
                            error!(get_span(*loc), "{prefix} the PDF contains PostScript code";
                            hint: "conic gradients are not supported in this PDF standard")
                        }
                        ValidationError::MissingCMYKProfile => {
                            error!(Span::detached(), "{prefix} the PDF is missing a CMYK profile";
                            hint: "CMYK colors are not yet supported in this export mode")
                        }
                        ValidationError::ContainsNotDefGlyph(f, loc, text) => {
                            let span = get_span(*loc);
                            let font_str = display_font(gc.fonts_backward.get(f).unwrap());

                            error!(span, "{prefix} the text '{text}' cannot be displayed \
                            using {font_str}";
                                hint: "try using a different font"
                            )

                        }
                        ValidationError::InvalidCodepointMapping(_, _, cp, loc) => {
                            let code_point = cp.map(|c| format!("{:#06x}", c as u32));
                            if let Some(cp) = code_point {
                                let msg = if loc.is_some() {
                                    "the PDF contains text with" 
                                } else {
                                    "the text contains" 
                                };
                                error!(get_span(*loc), "{prefix} {msg} the disallowed \
                                codepoint {cp}")
                            }   else {
                                // I think this code path is in theory unreachable, 
                                // but just to be safe.
                                let msg = if loc.is_some() { "the PDF contains text with missing codepoints" } else { "the text was not mapped to a code point" };
                                error!(get_span(*loc), "{prefix} {msg}";
                                hint: "for complex scripts like indic or arabic, it might \
                                not be possible to produce a compliant document")
                            }
                        }
                        ValidationError::UnicodePrivateArea(_, _, c, loc) => {
                            let code_point = format!("{:#06x}", *c as u32);
                            let msg = if loc.is_some() { "the PDF" } else { "the text" };

                            error!(get_span(*loc), "{prefix} {msg} contains the codepoint \
                                {code_point}";
                                hint: "codepoints from the Unicode private area are \
                                forbidden in this export mode")
                        }
                        ValidationError::Transparency(loc) => {
                            let span = get_span(*loc);
                            let is_img = gc.image_spans.contains(&span);
                            let hint1 = "export using a different standard \
                            that supports transparency";

                            if loc.is_some() {
                                if is_img {
                                    error!(get_span(*loc), "{prefix} the image contains transparency";
                                        hint: "convert the image to a non-transparent one";
                                        hint: "you might have to convert SVGs into a \
                                        non-transparent bitmap image";
                                        hint: "{hint1}"
                                    )
                                }   else {
                                    error!(get_span(*loc), "{prefix} the used fill or stroke has \
                                    transparency";
                                        hint: "don't use colors with transparency in \
                                        this export mode";
                                        hint: "{hint1}"
                                    )
                                }
                            }   else {
                                error!(get_span(*loc), "{prefix} the PDF contains transparency";
                                        hint: "convert any images with transparency into \
                                        non-transparent ones";
                                        hint: "don't use fills or strokes with transparent colors";
                                        hint: "{hint1}"
                                    )
                            }
                        }
                        ValidationError::ImageInterpolation(loc) => {
                            let span = get_span(*loc);

                            if loc.is_some() {
                                error!(span, "{prefix} the image has smooth scaling";
                                hint: "set the `scaling` attribute to `pixelated`")
                            }   else {
                                error!(span, "{prefix} an image in the PDF has smooth scaling";
                                hint: "set the `scaling` attribute of all images \
                                to `pixelated`")
                            }
                        }
                        ValidationError::EmbeddedFile(e, s) => {
                            // We always set the span for embedded files, so it cannot be detached.
                            let span = get_span(*s);
                            match e {
                                EmbedError::Existence => {
                                    error!(span, "{prefix} document contains an embedded file";
                                        hint: "embedded files are not supported in this \
                                        export mode"
                                    )
                                }
                                EmbedError::MissingDate => {
                                    error!(span, "{prefix} document date is missing";
                                        hint: "the document date needs to be set when \
                                        embedding files"
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
                        // and PDF/UA.
                        // But let's still add a message just to be on the safe side.
                        ValidationError::MissingAnnotationAltText => {
                            error!(Span::detached(), "{prefix} missing annotation alt text";
                                hint: "please report this as a bug"
                            )
                        }
                        ValidationError::MissingAltText => {
                            error!(Span::detached(), "{prefix} missing alt text";
                                hint: "make sure your images and formulas have alt text"
                            )
                        }
                        ValidationError::NoDocumentLanguage => {
                            error!(Span::detached(), "{prefix} missing document language";
                                hint: "set the language of the document"
                            )
                        }
                        // Needs to be set by typst-pdf.
                        ValidationError::MissingHeadingTitle => {
                            error!(Span::detached(), "{prefix} missing heading title";
                                hint: "please report this as a bug"
                            )
                        }
                        ValidationError::MissingDocumentOutline => {
                            error!(Span::detached(), "{prefix} missing document outline";
                                hint: "please report this as a bug"
                            )
                        }
                        ValidationError::MissingTagging => {
                            error!(Span::detached(), "{prefix} missing document tags";
                                hint: "please report this as a bug"
                            )
                        }
                        ValidationError::NoDocumentTitle => {
                            error!(Span::detached(), "{prefix} missing document title";
                                hint: "set the title of the document"
                            )
                        }
                        ValidationError::MissingDocumentDate => {
                            error!(Span::detached(), "{prefix} missing document date";
                                hint: "set the date of the document"
                            )
                        }
                    }
                })
                    .collect::<EcoVec<_>>();

                Err(errors)
            }
            KrillaError::Image(i) => {
                let span = gc.image_to_spans.get(&i).unwrap();
                bail!(*span, "failed to process image");
            }
            KrillaError::SixteenBitImage(image, _) => {
                let span = gc.image_to_spans.get(&image).unwrap();
                bail!(*span, "16 bit images are not supported in this export mode";
                    hint: "convert the image to 8 bit instead")
            }
        },
    }
}

fn collect_named_destinations(
    document: &PagedDocument,
    pic: &PageIndexConverter,
) -> HashMap<Location, NamedDestination> {
    let mut locs_to_names = HashMap::new();

    // Find all headings that have a label and are the first among other
    // headings with the same label.
    let matches: Vec<_> = {
        let mut seen = HashSet::new();
        document
            .introspector
            .query(&HeadingElem::elem().select())
            .iter()
            .filter_map(|elem| elem.location().zip(elem.label()))
            .filter(|&(_, label)| seen.insert(label))
            .collect()
    };

    for (loc, label) in matches {
        let pos = document.introspector.position(loc);
        let index = pos.page.get() - 1;
        // We are subtracting 10 because the position of links e.g. to headings is always at the
        // baseline and if you link directly to it, the text will not be visible
        // because it is right above.
        let y = (pos.point.y - Abs::pt(10.0)).max(Abs::zero());

        // Only add named destination if page belonging to the position is exported.
        if let Some(index) = pic.pdf_page_index(index) {
            let named = NamedDestination::new(
                label.resolve().to_string(),
                XyzDestination::new(
                    index,
                    krilla::geom::Point::from_xy(pos.point.x.to_f32(), y.to_f32()),
                ),
            );
            locs_to_names.insert(loc, named);
        }
    }

    locs_to_names
}

fn get_configuration(options: &PdfOptions) -> SourceResult<Configuration> {
    let config = match (options.pdf_version, options.validator) {
        (None, None) => {
            Configuration::new_with_version(krilla::configure::PdfVersion::Pdf17)
        }
        (Some(pdf), None) => Configuration::new_with_version(pdf.into()),
        (None, Some(v)) => Configuration::new_with_validator(v.into()),
        (Some(pdf), Some(v)) => {
            let pdf = pdf.into();
            let v = v.into();

            match Configuration::new_with(v, pdf) {
                Some(c) => c,
                None => {
                    let pdf_string = pdf.as_str();
                    let s_string = v.as_str();

                    let h_message = format!(
                        "export using version {} instead",
                        v.recommended_version().as_str()
                    );

                    bail!(Span::detached(), "{pdf_string} is not compatible with {s_string}"; hint: "{h_message}");
                }
            }
        }
    };

    Ok(config)
}

pub(crate) struct PageIndexConverter {
    page_indices: HashMap<usize, usize>,
    skipped_pages: usize,
}

impl PageIndexConverter {
    pub fn new(document: &PagedDocument, options: &PdfOptions) -> Self {
        let mut page_indices = HashMap::new();
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
