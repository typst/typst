use std::collections::{BTreeMap, HashMap, HashSet};

use krilla::annotation::Annotation;
use krilla::configure::{Configuration, PdfVersion, ValidationError};
use krilla::destination::{NamedDestination, XyzDestination};
use krilla::error::KrillaError;
use krilla::page::PageLabel;
use krilla::path::PathBuilder;
use krilla::surface::Surface;
use krilla::{Document, PageSettings, SerializeSettings};
use typst_library::diag::{bail, SourceResult};
use typst_library::foundations::NativeElement;
use typst_library::introspection::Location;
use typst_library::layout::{
    Abs, Frame, FrameItem, GroupItem, PagedDocument, Size, Transform,
};
use typst_library::model::HeadingElem;
use typst_library::text::{Font, Lang};
use typst_library::visualize::{Geometry, Paint};
use typst_syntax::Span;

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
    };

    let mut document = Document::new_with(settings);
    let mut gc = GlobalContext::new(
        typst_document,
        options,
        collect_named_destinations(typst_document, options),
    );

    convert_pages(&mut gc, &mut document)?;

    document.set_outline(build_outline(&gc));
    document.set_metadata(build_metadata(&gc));

    finish(document, gc)
}

fn convert_pages(gc: &mut GlobalContext, document: &mut Document) -> SourceResult<()> {
    let mut skipped_pages = 0;

    for (i, typst_page) in gc.document.pages.iter().enumerate() {
        if gc
            .options
            .page_ranges
            .as_ref()
            .is_some_and(|ranges| !ranges.includes_page_index(i))
        {
            // Don't export this page.
            skipped_pages += 1;
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
                    (skipped_pages > 0).then(|| PageLabel::arabic(i + 1))
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
    pub(crate) fonts_forward: HashMap<Font, krilla::font::Font>,
    pub(crate) fonts_backward: HashMap<krilla::font::Font, Font>,
    /// Mapping between images and their span.
    // Note: In theory, the same image can have multiple spans
    // if it appears in the document multiple times. We just store the
    // first appearance, though.
    pub(crate) image_spans: HashMap<krilla::image::Image, Span>,
    pub(crate) document: &'a PagedDocument,
    pub(crate) options: &'a PdfOptions<'a>,
    /// Mapping between locations in the document and named destinations.
    pub(crate) loc_to_named: HashMap<Location, NamedDestination>,
    /// The languages used throughout the document.
    pub(crate) languages: BTreeMap<Lang, usize>,
}

impl<'a> GlobalContext<'a> {
    pub(crate) fn new(
        document: &'a PagedDocument,
        options: &'a PdfOptions,
        loc_to_named: HashMap<Location, NamedDestination>,
    ) -> GlobalContext<'a> {
        Self {
            fonts_forward: HashMap::new(),
            fonts_backward: HashMap::new(),
            document,
            options,
            loc_to_named,
            image_spans: HashMap::new(),
            languages: BTreeMap::new(),
        }
    }

    pub(crate) fn page_excluded(&self, page_index: usize) -> bool {
        self.options
            .page_ranges
            .as_ref()
            .is_some_and(|ranges| !ranges.includes_page_index(page_index))
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
        handle_shape(fc, &shape, surface, gc)?;
    }

    for (point, item) in frame.items() {
        fc.push();
        fc.state_mut().pre_concat(Transform::translate(point.x, point.y));

        match item {
            FrameItem::Group(g) => handle_group(fc, g, surface, gc)?,
            FrameItem::Text(t) => handle_text(fc, t, surface, gc)?,
            FrameItem::Shape(s, _) => handle_shape(fc, s, surface, gc)?,
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
        surface.push_clip_path(clip_path, &krilla::path::FillRule::NonZero);
    }

    handle_frame(fc, &group.frame, None, surface, context)?;

    if clip_path.is_some() {
        surface.pop();
    }

    fc.pop();

    Ok(())
}

/// Finish a krilla document and handle export errors.
fn finish(document: Document, gc: GlobalContext) -> SourceResult<Vec<u8>> {
    let validator: krilla::configure::Validator = gc
        .options
        .validator
        .map(|v| v.into())
        .unwrap_or(krilla::configure::Validator::None);

    match document.finish() {
        Ok(r) => Ok(r),
        Err(e) => match e {
            KrillaError::FontError(f, s) => {
                let font_str = display_font(gc.fonts_backward.get(&f).unwrap());
                bail!(Span::detached(), "failed to process font {font_str} ({s})");
            }
            KrillaError::UserError(u) => {
                // This is an error which indicates misuse on the typst-pdf side.
                bail!(Span::detached(), "internal error ({u})"; hint: "please report this as a bug")
            }
            KrillaError::ValidationError(ve) => {
                // We can only produce 1 error, so just take the first one.
                let prefix =
                    format!("validated export for {} failed:", validator.as_str());

                match &ve[0] {
                    ValidationError::TooLongString => {
                        bail!(Span::detached(), "{prefix} a PDF string longer \
                        than 32767 characters";
                            hint: "make sure title and author names are short enough");
                    }
                    // Should in theory never occur, as krilla always trims font names
                    ValidationError::TooLongName => {
                        bail!(Span::detached(), "{prefix} a PDF name longer than 127 characters";
                            hint: "perhaps a font name is too long");
                    }
                    ValidationError::TooLongArray => {
                        bail!(Span::detached(), "{prefix} a PDF array longer than 8191 elements";
                            hint: "this can happen if you have a very long text in a single line");
                    }
                    ValidationError::TooLongDictionary => {
                        bail!(Span::detached(), "{prefix} a PDF dictionary had \
                        more than 4095 entries";
                            hint: "try reducing the complexity of your document");
                    }
                    ValidationError::TooLargeFloat => {
                        bail!(Span::detached(), "{prefix} a PDF float was larger than \
                        the allowed limit";
                            hint: "try exporting using a higher PDF version");
                    }
                    ValidationError::TooManyIndirectObjects => {
                        bail!(Span::detached(), "{prefix} the PDF has too many indirect objects";
                            hint: "reduce the size of your document");
                    }
                    // Can only occur if we have 27+ nested clip paths
                    ValidationError::TooHighQNestingLevel => {
                        bail!(Span::detached(), "{prefix} the PDF has too high q nesting";
                            hint: "reduce the number of nested containers");
                    }
                    ValidationError::ContainsPostScript => {
                        bail!(Span::detached(), "{prefix} the PDF contains PostScript code";
                            hint: "sweep gradients are not supported in this PDF standard");
                    }
                    ValidationError::MissingCMYKProfile => {
                        bail!(Span::detached(), "{prefix} the PDF is missing a CMYK profile";
                            hint: "CMYK colors are not yet supported in this export mode");
                    }
                    ValidationError::ContainsNotDefGlyph => {
                        bail!(Span::detached(), "{prefix} the PDF contains the .notdef glyph";
                            hint: "ensure all text can be displayed using an available font");
                    }
                    ValidationError::InvalidCodepointMapping(_, _) => {
                        bail!(Span::detached(), "{prefix} the PDF contains \
                        disallowed codepoints or is missing codepoint mappings";
                            hint: "make sure to not use the unicode characters 0x0, \
                            0xFEFF or 0xFFFE";
                            hint: "for complex scripts like indic or arabic, it might \
                            not be possible to produce a compliant document");
                    }
                    ValidationError::UnicodePrivateArea(_, _) => {
                        bail!(Span::detached(), "{prefix} the PDF contains characters from the \
                        Unicode private area";
                            hint: "remove the text containing codepoints \
                            from the Unicode private area");
                    }
                    ValidationError::Transparency => {
                        bail!(Span::detached(), "{prefix} document contains transparency";
                            hint: "remove any transparency from your \
                            document (e.g. fills with opacity)";
                            hint: "you might have to convert certain SVGs into a bitmap image if \
                            they contain transparency";
                            hint: "export using a different standard that supports transparency"
                        );
                    }
                    ValidationError::ImageInterpolation => {
                        bail!(Span::detached(), "{prefix} document contains an image with smooth interpolation";
                            hint: "such images are not supported in this export mode"
                        );
                    }
                    ValidationError::EmbeddedFile(_) => {
                        bail!(Span::detached(), "{prefix} document contains an embedded file";
                            hint: "embedded files are not supported in this export mode"
                        );
                    }

                    // The below errors cannot occur yet, only once Typst supports full PDF/A
                    // and PDF/UA.
                    // But let's still add a message just to be on the safe side.
                    ValidationError::MissingAnnotationAltText => {
                        bail!(Span::detached(), "{prefix} missing annotation alt text";
                            hint: "please report this as a bug");
                    }
                    ValidationError::MissingAltText => {
                        bail!(Span::detached(), "{prefix} missing alt text";
                            hint: "make sure your images and formulas have alt text");
                    }
                    ValidationError::NoDocumentLanguage => {
                        bail!(Span::detached(), "{prefix} missing document language";
                            hint: "set the language of the document");
                    }

                    // Needs to be set by typst-pdf.
                    ValidationError::MissingHeadingTitle => {
                        bail!(Span::detached(), "{prefix} missing heading title";
                            hint: "please report this as a bug");
                    }
                    ValidationError::MissingDocumentOutline => {
                        bail!(Span::detached(), "{prefix} missing document outline";
                            hint: "please report this as a bug");
                    }
                    ValidationError::MissingTagging => {
                        bail!(Span::detached(), "{prefix} missing document tags";
                            hint: "please report this as a bug");
                    }
                    ValidationError::NoDocumentTitle => {
                        bail!(Span::detached(), "{prefix} missing document title";
                            hint: "set the title of the document");
                    }
                }
            }
            KrillaError::ImageError(i) => {
                let span = gc.image_spans.get(&i).unwrap();
                bail!(*span, "failed to process image");
            }
        },
    }
}

fn collect_named_destinations(
    document: &PagedDocument,
    options: &PdfOptions,
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
        if options
            .page_ranges
            .as_ref()
            .is_some_and(|ranges| !ranges.includes_page_index(index))
        {
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
        (None, None) => Configuration::new_with_version(PdfVersion::Pdf17),
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
                        "export using {} instead",
                        v.recommended_version().as_str()
                    );

                    bail!(Span::detached(), "{pdf_string} is not compatible with standard {s_string}"; hint: "{h_message}");
                }
            }
        }
    };

    Ok(config)
}
