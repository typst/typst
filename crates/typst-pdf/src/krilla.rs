use crate::image::handle_image;
use crate::link::handle_link;
use crate::metadata::build_metadata;
use crate::outline::build_outline;
use crate::page::PageLabelExt;
use crate::util::{build_path, display_font, AbsExt, PointExt, SizeExt, TransformExt};
use crate::{paint, PdfOptions};
use bytemuck::TransparentWrapper;
use krilla::action::{Action, LinkAction};
use krilla::annotation::{LinkAnnotation, Target};
use krilla::destination::{NamedDestination, XyzDestination};
use krilla::error::KrillaError;
use krilla::font::{GlyphId, GlyphUnits};
use krilla::geom::Rect;
use krilla::page::PageLabel;
use krilla::path::PathBuilder;
use krilla::surface::Surface;
use krilla::validation::ValidationError;
use krilla::version::PdfVersion;
use krilla::{Document, PageSettings, SerializeSettings, SvgSettings};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::ops::Range;
use std::sync::Arc;
use typst_library::diag::{bail, SourceResult};
use typst_library::foundations::NativeElement;
use typst_library::introspection::Location;
use typst_library::layout::{
    Abs, Frame, FrameItem, GroupItem, PagedDocument, Point, Size, Transform,
};
use typst_library::model::{Destination, HeadingElem};
use typst_library::text::{Font, Glyph, Lang, TextItem};
use typst_library::visualize::{
    FillRule, Geometry, Image, ImageKind, Paint, Path, PathItem, Shape,
};
use typst_syntax::Span;

#[derive(Debug, Clone)]
pub(crate) struct State {
    /// The full transform chain
    transform_chain: Transform,
    /// The transform of the current item.
    pub(crate) transform: Transform,
    /// The transform of first hard frame in the hierarchy.
    container_transform_chain: Transform,
    /// The size of the first hard frame in the hierarchy.
    size: Size,
}

impl State {
    /// Creates a new, clean state for a given `size`.
    fn new(
        size: Size,
        transform_chain: Transform,
        container_transform_chain: Transform,
    ) -> Self {
        Self {
            transform_chain,
            transform: Transform::identity(),
            container_transform_chain,
            size,
        }
    }

    pub fn size(&mut self, size: Size) {
        self.size = size;
    }

    pub fn transform(&mut self, transform: Transform) {
        self.transform = self.transform.pre_concat(transform);
        self.transform_chain = self.transform_chain.pre_concat(transform);
    }

    fn set_container_transform(&mut self) {
        self.container_transform_chain = self.transform_chain;
    }

    /// Creates the [`Transforms`] structure for the current item.
    pub fn transforms(&self, size: Size) -> Transforms {
        Transforms {
            transform_chain_: self.transform_chain,
            container_transform_chain: self.container_transform_chain,
            container_size: self.size,
            size,
        }
    }
}

pub(crate) struct FrameContext {
    states: Vec<State>,
    pub(crate) annotations: Vec<krilla::annotation::Annotation>,
}

impl FrameContext {
    pub fn new(size: Size) -> Self {
        Self {
            states: vec![State::new(size, Transform::identity(), Transform::identity())],
            annotations: vec![],
        }
    }

    pub fn push(&mut self) {
        self.states.push(self.states.last().unwrap().clone());
    }

    pub fn pop(&mut self) {
        self.states.pop();
    }

    pub fn state(&self) -> &State {
        self.states.last().unwrap()
    }

    pub fn state_mut(&mut self) -> &mut State {
        self.states.last_mut().unwrap()
    }
}

/// Subset of the state used to calculate the transform of gradients and patterns.
#[derive(Debug, Clone, Copy)]
pub(super) struct Transforms {
    /// The full transform chain.
    pub transform_chain_: Transform,
    /// The transform of first hard frame in the hierarchy.
    pub container_transform_chain: Transform,
    /// The size of the first hard frame in the hierarchy.
    pub container_size: Size,
    /// The size of the item.
    pub size: Size,
}

#[derive(TransparentWrapper)]
#[repr(transparent)]
struct PdfGlyph(Glyph);

impl krilla::font::Glyph for PdfGlyph {
    fn glyph_id(&self) -> GlyphId {
        GlyphId::new(self.0.id as u32)
    }

    fn text_range(&self) -> Range<usize> {
        self.0.range.start as usize..self.0.range.end as usize
    }

    fn x_advance(&self) -> f32 {
        self.0.x_advance.get() as f32
    }

    fn x_offset(&self) -> f32 {
        self.0.x_offset.get() as f32
    }

    fn y_offset(&self) -> f32 {
        0.0
    }

    fn y_advance(&self) -> f32 {
        0.0
    }
}

pub struct GlobalContext<'a> {
    /// Cache the conversion between krilla and Typst fonts (forward and backward).
    fonts_forward: HashMap<Font, krilla::font::Font>,
    fonts_backward: HashMap<krilla::font::Font, Font>,
    // Note: In theory, the same image can have multiple spans
    // if it appears in the document multiple times. We just store the
    // first appearance, though.
    /// Mapping between images and their span.
    pub(crate) image_spans: HashMap<krilla::image::Image, Span>,
    pub(crate) document: &'a PagedDocument,
    pub(crate) options: &'a PdfOptions<'a>,
    /// Mapping between locations in the document and named destinations.
    pub(crate) loc_to_named: HashMap<Location, NamedDestination>,
    /// The languages used throughout the document.
    pub(crate) languages: BTreeMap<Lang, usize>,
}

impl<'a> GlobalContext<'a> {
    pub fn new(
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

// TODO: Change rustybuzz cluster behavior so it works with ActualText

#[typst_macros::time(name = "write pdf")]
pub fn pdf(
    typst_document: &PagedDocument,
    options: &PdfOptions,
) -> SourceResult<Vec<u8>> {
    let version = get_version(options)?;

    let settings = SerializeSettings {
        compress_content_streams: true,
        no_device_cs: true,
        ascii_compatible: false,
        xmp_metadata: true,
        cmyk_profile: None,
        validator: options.validator,
        enable_tagging: false,
        pdf_version: version,
    };

    let mut locs_to_names = HashMap::new();
    let mut seen = HashSet::new();

    // Find all headings that have a label and are the first among other
    // headings with the same label.
    let mut matches: Vec<_> = typst_document
        .introspector
        .query(&HeadingElem::elem().select())
        .iter()
        .filter_map(|elem| elem.location().zip(elem.label()))
        .filter(|&(_, label)| seen.insert(label))
        .collect();

    // Named destinations must be sorted by key.
    matches.sort_by_key(|&(_, label)| label.resolve());

    for (loc, label) in matches {
        let pos = typst_document.introspector.position(loc);
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

    let mut document = Document::new_with(settings);
    let mut gc = GlobalContext::new(&typst_document, options, locs_to_names);

    let mut skipped_pages = 0;

    for (i, typst_page) in typst_document.pages.iter().enumerate() {
        if options
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
            process_frame(
                &mut fc,
                &typst_page.frame,
                typst_page.fill_or_transparent(),
                &mut surface,
                &mut gc,
            )?;
            surface.finish();

            for annotation in fc.annotations {
                page.add_annotation(annotation);
            }
        }
    }

    document.set_outline(build_outline(&gc));
    document.set_metadata(build_metadata(&gc));

    finish(document, gc)
}

/// Finish a krilla document and handle export errors.
fn finish(document: Document, gc: GlobalContext) -> SourceResult<Vec<u8>> {
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
                let prefix = format!(
                    "validated export for {} failed:",
                    gc.options.validator.as_str()
                );
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
                        bail!(Span::detached(), "{prefix} the PDF contains the \
                        disallowed codepoints";
                            hint: "make sure to not use the Unicode characters 0x0, \
                            0xFEFF or 0xFFFE");
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
                    // Needs to be set by Typst.
                    ValidationError::MissingHeadingTitle => {
                        bail!(Span::detached(), "{prefix} missing heading title";
                            hint: "please report this as a bug");
                    }
                    // Needs to be set by Typst.
                    ValidationError::MissingDocumentOutline => {
                        bail!(Span::detached(), "{prefix} missing document outline";
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

fn get_version(options: &PdfOptions) -> SourceResult<PdfVersion> {
    match options.pdf_version {
        None => Ok(options.validator.recommended_version()),
        Some(v) => {
            if !options.validator.compatible_with_version(v) {
                let v_string = v.as_str();
                let s_string = options.validator.as_str();
                let h_message = format!(
                    "export using {} instead",
                    options.validator.recommended_version().as_str()
                );
                bail!(Span::detached(), "{v_string} is not compatible with standard {s_string}"; hint: "{h_message}");
            } else {
                Ok(v)
            }
        }
    }
}

pub fn process_frame(
    fc: &mut FrameContext,
    frame: &Frame,
    fill: Option<Paint>,
    surface: &mut Surface,
    gc: &mut GlobalContext,
) -> SourceResult<()> {
    fc.push();

    if frame.kind().is_hard() {
        fc.state_mut().set_container_transform();
        fc.state_mut().size(frame.size());
    }

    if let Some(fill) = fill {
        let shape = Geometry::Rect(frame.size()).filled(fill);
        handle_shape(fc, &shape, surface, gc)?;
    }

    for (point, item) in frame.items() {
        fc.push();
        fc.state_mut().transform(Transform::translate(point.x, point.y));

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

pub fn handle_group(
    fc: &mut FrameContext,
    group: &GroupItem,
    surface: &mut Surface,
    context: &mut GlobalContext,
) -> SourceResult<()> {
    fc.push();
    fc.state_mut().transform(group.transform);

    let clip_path = group
        .clip_path
        .as_ref()
        .and_then(|p| {
            let mut builder = PathBuilder::new();
            build_path(p, &mut builder);
            builder.finish()
        })
        .and_then(|p| p.transform(fc.state().transform.to_krilla()));

    if let Some(clip_path) = &clip_path {
        surface.push_clip_path(clip_path, &krilla::path::FillRule::NonZero);
    }

    process_frame(fc, &group.frame, None, surface, context)?;

    if clip_path.is_some() {
        surface.pop();
    }

    fc.pop();

    Ok(())
}

pub fn handle_text(
    fc: &mut FrameContext,
    t: &TextItem,
    surface: &mut Surface,
    gc: &mut GlobalContext,
) -> SourceResult<()> {
    let typst_font = t.font.clone();

    let krilla_font = if let Some(font) = gc.fonts_forward.get(&typst_font) {
        font.clone()
    } else {
        let font = match krilla::font::Font::new(
            Arc::new(typst_font.data().clone()),
            typst_font.index(),
            true,
        ) {
            None => {
                let font_str = display_font(&typst_font);
                bail!(Span::detached(), "failed to process font {font_str}");
            }
            Some(f) => f,
        };

        gc.fonts_forward.insert(typst_font.clone(), font.clone());
        gc.fonts_backward.insert(font.clone(), typst_font.clone());

        font
    };

    *gc.languages.entry(t.lang).or_insert(0) += t.glyphs.len();

    let fill = paint::fill(
        gc,
        &t.fill,
        FillRule::NonZero,
        true,
        surface,
        fc.state().transforms(Size::zero()),
    )?;
    let text = t.text.as_str();
    let size = t.size;

    let glyphs: &[PdfGlyph] = TransparentWrapper::wrap_slice(t.glyphs.as_slice());

    surface.push_transform(&fc.state().transform.to_krilla());

    surface.fill_glyphs(
        krilla::geom::Point::from_xy(0.0, 0.0),
        fill,
        &glyphs,
        krilla_font.clone(),
        text,
        size.to_f32(),
        GlyphUnits::Normalized,
        false,
    );

    if let Some(stroke) = t
        .stroke
        .as_ref()
        .map(|s| paint::stroke(gc, s, true, surface, fc.state().transforms(Size::zero())))
    {
        let stroke = stroke?;

        surface.stroke_glyphs(
            krilla::geom::Point::from_xy(0.0, 0.0),
            stroke,
            &glyphs,
            krilla_font.clone(),
            text,
            size.to_f32(),
            GlyphUnits::Normalized,
            true,
        );
    }

    surface.pop();

    Ok(())
}

pub fn handle_shape(
    fc: &mut FrameContext,
    shape: &Shape,
    surface: &mut Surface,
    gc: &mut GlobalContext,
) -> SourceResult<()> {
    let mut path_builder = PathBuilder::new();

    match &shape.geometry {
        Geometry::Line(l) => {
            path_builder.move_to(0.0, 0.0);
            path_builder.line_to(l.x.to_f32(), l.y.to_f32());
        }
        Geometry::Rect(size) => {
            let w = size.x.to_f32();
            let h = size.y.to_f32();
            let rect = if w < 0.0 || h < 0.0 {
                // Skia doesn't normally allow for negative dimensions, but
                // Typst supports them, so we apply a transform if needed
                // Because this operation is expensive according to tiny-skia's
                // docs, we prefer to not apply it if not needed
                let transform =
                    krilla::geom::Transform::from_scale(w.signum(), h.signum());
                Rect::from_xywh(0.0, 0.0, w.abs(), h.abs())
                    .and_then(|rect| rect.transform(transform))
            } else {
                Rect::from_xywh(0.0, 0.0, w, h)
            };

            if let Some(rect) = rect {
                path_builder.push_rect(rect);
            }
        }
        Geometry::Path(p) => {
            build_path(p, &mut path_builder);
        }
    }

    surface.push_transform(&fc.state().transform.to_krilla());

    if let Some(path) = path_builder.finish() {
        if let Some(paint) = &shape.fill {
            let fill = paint::fill(
                gc,
                &paint,
                shape.fill_rule,
                false,
                surface,
                fc.state().transforms(shape.geometry.bbox_size()),
            )?;
            surface.fill_path(&path, fill);
        }

        let stroke = shape.stroke.as_ref().and_then(|stroke| {
            if stroke.thickness.to_f32() > 0.0 {
                Some(stroke)
            } else {
                None
            }
        });

        if let Some(stroke) = &stroke {
            let stroke = paint::stroke(
                gc,
                stroke,
                false,
                surface,
                fc.state().transforms(shape.geometry.bbox_size()),
            )?;
            surface.stroke_path(&path, stroke);
        }
    }

    surface.pop();

    Ok(())
}
