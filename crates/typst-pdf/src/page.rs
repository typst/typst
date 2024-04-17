use std::collections::HashMap;
use std::num::NonZeroUsize;

use crate::color::PaintEncode;
use crate::extg::ExtGState;
use crate::image::deferred_image;
use crate::{deflate_deferred, AbsExt, EmExt, PdfContext};
use ecow::{eco_format, EcoString};
use pdf_writer::types::{
    ActionType, AnnotationFlags, AnnotationType, ColorSpaceOperand, LineCapStyle,
    LineJoinStyle, NumberingStyle, TextRenderingMode,
};
use pdf_writer::writers::{PageLabel, Resources};
use pdf_writer::{Content, Filter, Finish, Name, Rect, Ref, Str, TextStr};
use typst::introspection::Meta;
use typst::layout::{
    Abs, Em, Frame, FrameItem, GroupItem, Page, Point, Ratio, Size, Transform,
};
use typst::model::{Destination, Numbering};
use typst::text::color::is_color_glyph;
use typst::text::{Case, Font, TextItem, TextItemView};
use typst::util::{Deferred, Numeric, SliceExt};
use typst::visualize::{
    FixedStroke, Geometry, Image, LineCap, LineJoin, Paint, Path, PathItem, Shape,
};

/// Construct page objects.
#[typst_macros::time(name = "construct pages")]
pub(crate) fn construct_pages(ctx: &mut PdfContext, pages: &[Page]) {
    for page in pages {
        let (page_ref, mut encoded) = construct_page(ctx, &page.frame);
        encoded.label = page
            .numbering
            .as_ref()
            .and_then(|num| PdfPageLabel::generate(num, page.number));
        ctx.page_refs.push(page_ref);
        ctx.pages.push(encoded);
    }
}

/// Construct a page object.
#[typst_macros::time(name = "construct page")]
pub(crate) fn construct_page(ctx: &mut PdfContext, frame: &Frame) -> (Ref, EncodedPage) {
    let page_ref = ctx.alloc.bump();

    let size = frame.size();
    let mut ctx = PageContext::new(ctx, size);

    // Make the coordinate system start at the top-left.
    ctx.bottom = size.y.to_f32();
    ctx.transform(Transform {
        sx: Ratio::one(),
        ky: Ratio::zero(),
        kx: Ratio::zero(),
        sy: Ratio::new(-1.0),
        tx: Abs::zero(),
        ty: size.y,
    });

    // Encode the page into the content stream.
    write_frame(&mut ctx, frame);

    let page = EncodedPage {
        size,
        content: deflate_deferred(ctx.content.finish()),
        id: page_ref,
        uses_opacities: ctx.uses_opacities,
        links: ctx.links,
        label: None,
        resources: ctx.resources,
    };

    (page_ref, page)
}

/// Write the page tree.
pub(crate) fn write_page_tree(ctx: &mut PdfContext) {
    for i in 0..ctx.pages.len() {
        write_page(ctx, i);
    }

    ctx.pdf
        .pages(ctx.page_tree_ref)
        .count(ctx.page_refs.len() as i32)
        .kids(ctx.page_refs.iter().copied());
}

/// Write the global resource dictionary that will be referenced by all pages.
///
/// We add a reference to this dictionary to each page individually instead of
/// to the root node of the page tree because using the resource inheritance
/// feature breaks PDF merging with Apple Preview.
pub(crate) fn write_global_resources(ctx: &mut PdfContext) {
    let images_ref = ctx.alloc.bump();
    let patterns_ref = ctx.alloc.bump();
    let ext_gs_states_ref = ctx.alloc.bump();
    let color_spaces_ref = ctx.alloc.bump();

    let mut images = ctx.pdf.indirect(images_ref).dict();
    for (image_ref, im) in ctx.image_map.pdf_indices(&ctx.image_refs) {
        let name = eco_format!("Im{}", im);
        images.pair(Name(name.as_bytes()), image_ref);
    }
    images.finish();

    let mut patterns = ctx.pdf.indirect(patterns_ref).dict();
    for (gradient_ref, gr) in ctx.gradient_map.pdf_indices(&ctx.gradient_refs) {
        let name = eco_format!("Gr{}", gr);
        patterns.pair(Name(name.as_bytes()), gradient_ref);
    }

    for (pattern_ref, p) in ctx.pattern_map.pdf_indices(&ctx.pattern_refs) {
        let name = eco_format!("P{}", p);
        patterns.pair(Name(name.as_bytes()), pattern_ref);
    }
    patterns.finish();

    let mut ext_gs_states = ctx.pdf.indirect(ext_gs_states_ref).dict();
    for (gs_ref, gs) in ctx.extg_map.pdf_indices(&ctx.ext_gs_refs) {
        let name = eco_format!("Gs{}", gs);
        ext_gs_states.pair(Name(name.as_bytes()), gs_ref);
    }
    ext_gs_states.finish();

    let color_spaces = ctx.pdf.indirect(color_spaces_ref).dict();
    ctx.colors.write_color_spaces(color_spaces, &mut ctx.alloc);

    let mut resources = ctx.pdf.indirect(ctx.global_resources_ref).start::<Resources>();
    resources.pair(Name(b"XObject"), images_ref);
    resources.pair(Name(b"Pattern"), patterns_ref);
    resources.pair(Name(b"ExtGState"), ext_gs_states_ref);
    resources.pair(Name(b"ColorSpace"), color_spaces_ref);

    let mut fonts = resources.fonts();
    for (font_ref, f) in ctx.font_map.pdf_indices(&ctx.font_refs) {
        let name = eco_format!("F{}", f);
        fonts.pair(Name(name.as_bytes()), font_ref);
    }

    for font in &ctx.color_font_map.all_refs {
        let name = eco_format!("Cf{}", font.get());
        fonts.pair(Name(name.as_bytes()), font);
    }
    fonts.finish();

    resources.finish();

    // Also write the resources for Type3 fonts, that only contains images,
    // color spaces and regular fonts (COLR glyphs depend on them).
    if !ctx.color_font_map.all_refs.is_empty() {
        let mut resources =
            ctx.pdf.indirect(ctx.type3_font_resources_ref).start::<Resources>();
        resources.pair(Name(b"XObject"), images_ref);
        resources.pair(Name(b"Pattern"), patterns_ref);
        resources.pair(Name(b"ExtGState"), ext_gs_states_ref);
        resources.pair(Name(b"ColorSpace"), color_spaces_ref);

        let mut fonts = resources.fonts();
        for (font_ref, f) in ctx.font_map.pdf_indices(&ctx.font_refs) {
            let name = eco_format!("F{}", f);
            fonts.pair(Name(name.as_bytes()), font_ref);
        }
        fonts.finish();

        resources.finish();
    }

    // Write all of the functions used by the document.
    ctx.colors.write_functions(&mut ctx.pdf);
}

/// Write a page tree node.
fn write_page(ctx: &mut PdfContext, i: usize) {
    let page = &ctx.pages[i];
    let content_id = ctx.alloc.bump();

    let mut page_writer = ctx.pdf.page(page.id);
    page_writer.parent(ctx.page_tree_ref);

    let w = page.size.x.to_f32();
    let h = page.size.y.to_f32();
    page_writer.media_box(Rect::new(0.0, 0.0, w, h));
    page_writer.contents(content_id);
    page_writer.pair(Name(b"Resources"), ctx.global_resources_ref);

    if page.uses_opacities {
        page_writer
            .group()
            .transparency()
            .isolated(false)
            .knockout(false)
            .color_space()
            .srgb();
    }

    let mut annotations = page_writer.annotations();
    for (dest, rect) in &page.links {
        let mut annotation = annotations.push();
        annotation.subtype(AnnotationType::Link).rect(*rect);
        annotation.border(0.0, 0.0, 0.0, None).flags(AnnotationFlags::PRINT);

        let pos = match dest {
            Destination::Url(uri) => {
                annotation
                    .action()
                    .action_type(ActionType::Uri)
                    .uri(Str(uri.as_bytes()));
                continue;
            }
            Destination::Position(pos) => *pos,
            Destination::Location(loc) => {
                if let Some(key) = ctx.loc_to_dest.get(loc) {
                    annotation
                        .action()
                        .action_type(ActionType::GoTo)
                        // `key` must be a `Str`, not a `Name`.
                        .pair(Name(b"D"), Str(key.as_str().as_bytes()));
                    continue;
                } else {
                    ctx.document.introspector.position(*loc)
                }
            }
        };

        let index = pos.page.get() - 1;
        let y = (pos.point.y - Abs::pt(10.0)).max(Abs::zero());

        if let Some(page) = ctx.pages.get(index) {
            annotation
                .action()
                .action_type(ActionType::GoTo)
                .destination()
                .page(page.id)
                .xyz(pos.point.x.to_f32(), (page.size.y - y).to_f32(), None);
        }
    }

    annotations.finish();
    page_writer.finish();

    ctx.pdf
        .stream(content_id, page.content.wait())
        .filter(Filter::FlateDecode);
}

/// Write the page labels.
pub(crate) fn write_page_labels(ctx: &mut PdfContext) -> Vec<(NonZeroUsize, Ref)> {
    // If there is no page labeled, we skip the writing
    if !ctx.pages.iter().any(|p| {
        p.label
            .as_ref()
            .is_some_and(|l| l.prefix.is_some() || l.style.is_some())
    }) {
        return Vec::new();
    }

    let mut result = vec![];
    let empty_label = PdfPageLabel::default();
    let mut prev: Option<&PdfPageLabel> = None;

    for (i, page) in ctx.pages.iter().enumerate() {
        let nr = NonZeroUsize::new(1 + i).unwrap();
        // If there are pages with empty labels between labeled pages, we must
        // write empty PageLabel entries.
        let label = page.label.as_ref().unwrap_or(&empty_label);

        if let Some(pre) = prev {
            if label.prefix == pre.prefix
                && label.style == pre.style
                && label.offset == pre.offset.map(|n| n.saturating_add(1))
            {
                prev = Some(label);
                continue;
            }
        }

        let id = ctx.alloc.bump();
        let mut entry = ctx.pdf.indirect(id).start::<PageLabel>();

        // Only add what is actually provided. Don't add empty prefix string if
        // it wasn't given for example.
        if let Some(prefix) = &label.prefix {
            entry.prefix(TextStr(prefix));
        }

        if let Some(style) = label.style {
            entry.style(to_pdf_numbering_style(style));
        }

        if let Some(offset) = label.offset {
            entry.offset(offset.get() as i32);
        }

        result.push((nr, id));
        prev = Some(label);
    }

    result
}

/// Specification for a PDF page label.
#[derive(Debug, Clone, PartialEq, Hash, Default)]
struct PdfPageLabel {
    /// Can be any string or none. Will always be prepended to the numbering style.
    prefix: Option<EcoString>,
    /// Based on the numbering pattern.
    ///
    /// If `None` or numbering is a function, the field will be empty.
    style: Option<PdfPageLabelStyle>,
    /// Offset for the page label start.
    ///
    /// Describes where to start counting from when setting a style.
    /// (Has to be greater or equal than 1)
    offset: Option<NonZeroUsize>,
}

/// A PDF page label number style.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
enum PdfPageLabelStyle {
    /// Decimal arabic numerals (1, 2, 3).
    Arabic,
    /// Lowercase roman numerals (i, ii, iii).
    LowerRoman,
    /// Uppercase roman numerals (I, II, III).
    UpperRoman,
    /// Lowercase letters (`a` to `z` for the first 26 pages,
    /// `aa` to `zz` and so on for the next).
    LowerAlpha,
    /// Uppercase letters (`A` to `Z` for the first 26 pages,
    /// `AA` to `ZZ` and so on for the next).
    UpperAlpha,
}

impl PdfPageLabel {
    /// Create a new `PdfNumbering` from a `Numbering` applied to a page
    /// number.
    fn generate(numbering: &Numbering, number: usize) -> Option<PdfPageLabel> {
        let Numbering::Pattern(pat) = numbering else {
            return None;
        };

        let (prefix, kind, case) = pat.pieces.first()?;

        // If there is a suffix, we cannot use the common style optimisation,
        // since PDF does not provide a suffix field.
        let mut style = None;
        if pat.suffix.is_empty() {
            use {typst::model::NumberingKind as Kind, PdfPageLabelStyle as Style};
            match (kind, case) {
                (Kind::Arabic, _) => style = Some(Style::Arabic),
                (Kind::Roman, Case::Lower) => style = Some(Style::LowerRoman),
                (Kind::Roman, Case::Upper) => style = Some(Style::UpperRoman),
                (Kind::Letter, Case::Lower) if number <= 26 => {
                    style = Some(Style::LowerAlpha)
                }
                (Kind::Letter, Case::Upper) if number <= 26 => {
                    style = Some(Style::UpperAlpha)
                }
                _ => {}
            }
        }

        // Prefix and offset depend on the style: If it is supported by the PDF
        // spec, we use the given prefix and an offset. Otherwise, everything
        // goes into prefix.
        let prefix = if style.is_none() {
            Some(pat.apply(&[number]))
        } else {
            (!prefix.is_empty()).then(|| prefix.clone())
        };

        let offset = style.and(NonZeroUsize::new(number));
        Some(PdfPageLabel { prefix, style, offset })
    }
}

/// Data for an exported page.
pub struct EncodedPage {
    /// The indirect object id of the page.
    pub id: Ref,
    /// The page's dimensions.
    pub size: Size,
    /// The page's content stream.
    pub content: Deferred<Vec<u8>>,
    /// Whether the page uses opacities.
    pub uses_opacities: bool,
    /// Links in the PDF coordinate system.
    pub links: Vec<(Destination, Rect)>,
    /// The page's used resources
    pub resources: HashMap<PageResource, usize>,
    /// The page's PDF label.
    label: Option<PdfPageLabel>,
}

/// Represents a resource being used in a PDF page by its name.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct PageResource {
    kind: ResourceKind,
    name: EcoString,
}

impl PageResource {
    pub fn new(kind: ResourceKind, name: EcoString) -> Self {
        Self { kind, name }
    }
}

/// A kind of resource being used in a PDF page.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum ResourceKind {
    XObject,
    Font,
    Gradient,
    Pattern,
    ExtGState,
}

impl PageResource {
    /// Returns the name of the resource.
    pub fn name(&self) -> Name<'_> {
        Name(self.name.as_bytes())
    }

    /// Returns whether the resource is an XObject.
    pub fn is_x_object(&self) -> bool {
        matches!(self.kind, ResourceKind::XObject)
    }

    /// Returns whether the resource is a font.
    pub fn is_font(&self) -> bool {
        matches!(self.kind, ResourceKind::Font)
    }

    /// Returns whether the resource is a gradient.
    pub fn is_gradient(&self) -> bool {
        matches!(self.kind, ResourceKind::Gradient)
    }

    /// Returns whether the resource is a pattern.
    pub fn is_pattern(&self) -> bool {
        matches!(self.kind, ResourceKind::Pattern)
    }

    /// Returns whether the resource is an external graphics state.
    pub fn is_ext_g_state(&self) -> bool {
        matches!(self.kind, ResourceKind::ExtGState)
    }
}

/// An exporter for the contents of a single PDF page.
pub struct PageContext<'a, 'b> {
    pub(crate) parent: &'a mut PdfContext<'b>,
    pub content: Content,
    state: State,
    saves: Vec<State>,
    pub bottom: f32,
    uses_opacities: bool,
    links: Vec<(Destination, Rect)>,
    /// Keep track of the resources being used in the page.
    pub resources: HashMap<PageResource, usize>,
}

impl<'a, 'b> PageContext<'a, 'b> {
    pub fn new(parent: &'a mut PdfContext<'b>, size: Size) -> Self {
        PageContext {
            parent,
            uses_opacities: false,
            content: Content::new(),
            state: State::new(size),
            saves: vec![],
            bottom: 0.0,
            links: vec![],
            resources: HashMap::default(),
        }
    }
}

/// A simulated graphics state used to deduplicate graphics state changes and
/// keep track of the current transformation matrix for link annotations.
#[derive(Debug, Clone)]
struct State {
    /// The transform of the current item.
    transform: Transform,
    /// The transform of first hard frame in the hierarchy.
    container_transform: Transform,
    /// The size of the first hard frame in the hierarchy.
    size: Size,
    font: Option<(Font, Abs)>,
    fill: Option<Paint>,
    fill_space: Option<Name<'static>>,
    external_graphics_state: Option<ExtGState>,
    stroke: Option<FixedStroke>,
    stroke_space: Option<Name<'static>>,
    text_rendering_mode: TextRenderingMode,
}

impl State {
    /// Creates a new, clean state for a given page `size`.
    pub fn new(size: Size) -> Self {
        Self {
            transform: Transform::identity(),
            container_transform: Transform::identity(),
            size,
            font: None,
            fill: None,
            fill_space: None,
            external_graphics_state: None,
            stroke: None,
            stroke_space: None,
            text_rendering_mode: TextRenderingMode::Fill,
        }
    }

    /// Creates the [`Transforms`] structure for the current item.
    pub fn transforms(&self, size: Size, pos: Point) -> Transforms {
        Transforms {
            transform: self.transform.pre_concat(Transform::translate(pos.x, pos.y)),
            container_transform: self.container_transform,
            container_size: self.size,
            size,
        }
    }
}

/// Subset of the state used to calculate the transform of gradients and patterns.
#[derive(Debug, Clone, Copy)]
pub(super) struct Transforms {
    /// The transform of the current item.
    pub transform: Transform,
    /// The transform of first hard frame in the hierarchy.
    pub container_transform: Transform,
    /// The size of the first hard frame in the hierarchy.
    pub container_size: Size,
    /// The size of the item.
    pub size: Size,
}

impl PageContext<'_, '_> {
    fn save_state(&mut self) {
        self.saves.push(self.state.clone());
        self.content.save_state();
    }

    fn restore_state(&mut self) {
        self.content.restore_state();
        self.state = self.saves.pop().expect("missing state save");
    }

    fn set_external_graphics_state(&mut self, graphics_state: &ExtGState) {
        let current_state = self.state.external_graphics_state.as_ref();
        if current_state != Some(graphics_state) {
            let index = self.parent.extg_map.insert(*graphics_state);
            let name = eco_format!("Gs{index}");
            self.content.set_parameters(Name(name.as_bytes()));
            self.resources
                .insert(PageResource::new(ResourceKind::ExtGState, name), index);

            if graphics_state.uses_opacities() {
                self.uses_opacities = true;
            }
        }
    }

    fn set_opacities(&mut self, stroke: Option<&FixedStroke>, fill: Option<&Paint>) {
        let stroke_opacity = stroke
            .map(|stroke| {
                let color = match &stroke.paint {
                    Paint::Solid(color) => *color,
                    Paint::Gradient(_) | Paint::Pattern(_) => return 255,
                };

                color.alpha().map_or(255, |v| (v * 255.0).round() as u8)
            })
            .unwrap_or(255);
        let fill_opacity = fill
            .map(|paint| {
                let color = match paint {
                    Paint::Solid(color) => *color,
                    Paint::Gradient(_) | Paint::Pattern(_) => return 255,
                };

                color.alpha().map_or(255, |v| (v * 255.0).round() as u8)
            })
            .unwrap_or(255);
        self.set_external_graphics_state(&ExtGState { stroke_opacity, fill_opacity });
    }

    pub fn transform(&mut self, transform: Transform) {
        let Transform { sx, ky, kx, sy, tx, ty } = transform;
        self.state.transform = self.state.transform.pre_concat(transform);
        if self.state.container_transform.is_identity() {
            self.state.container_transform = self.state.transform;
        }
        self.content.transform([
            sx.get() as _,
            ky.get() as _,
            kx.get() as _,
            sy.get() as _,
            tx.to_f32(),
            ty.to_f32(),
        ]);
    }

    fn group_transform(&mut self, transform: Transform) {
        self.state.container_transform =
            self.state.container_transform.pre_concat(transform);
    }

    fn set_font(&mut self, font: &Font, size: Abs) {
        if self.state.font.as_ref().map(|(f, s)| (f, *s)) != Some((font, size)) {
            let index = self.parent.font_map.insert(font.clone());
            let name = eco_format!("F{index}");
            self.content.set_font(Name(name.as_bytes()), size.to_f32());
            self.resources
                .insert(PageResource::new(ResourceKind::Font, name), index);
            self.state.font = Some((font.clone(), size));
        }
    }

    fn size(&mut self, size: Size) {
        self.state.size = size;
    }

    fn set_fill(&mut self, fill: &Paint, on_text: bool, transforms: Transforms) {
        if self.state.fill.as_ref() != Some(fill)
            || matches!(self.state.fill, Some(Paint::Gradient(_)))
        {
            fill.set_as_fill(self, on_text, transforms);
            self.state.fill = Some(fill.clone());
        }
    }

    pub fn set_fill_color_space(&mut self, space: Name<'static>) {
        if self.state.fill_space != Some(space) {
            self.content.set_fill_color_space(ColorSpaceOperand::Named(space));
            self.state.fill_space = Some(space);
        }
    }

    pub fn reset_fill_color_space(&mut self) {
        self.state.fill_space = None;
    }

    fn set_stroke(
        &mut self,
        stroke: &FixedStroke,
        on_text: bool,
        transforms: Transforms,
    ) {
        if self.state.stroke.as_ref() != Some(stroke)
            || matches!(
                self.state.stroke.as_ref().map(|s| &s.paint),
                Some(Paint::Gradient(_))
            )
        {
            let FixedStroke { paint, thickness, cap, join, dash, miter_limit } = stroke;
            paint.set_as_stroke(self, on_text, transforms);

            self.content.set_line_width(thickness.to_f32());
            if self.state.stroke.as_ref().map(|s| &s.cap) != Some(cap) {
                self.content.set_line_cap(to_pdf_line_cap(*cap));
            }
            if self.state.stroke.as_ref().map(|s| &s.join) != Some(join) {
                self.content.set_line_join(to_pdf_line_join(*join));
            }
            if self.state.stroke.as_ref().map(|s| &s.dash) != Some(dash) {
                if let Some(pattern) = dash {
                    self.content.set_dash_pattern(
                        pattern.array.iter().map(|l| l.to_f32()),
                        pattern.phase.to_f32(),
                    );
                } else {
                    self.content.set_dash_pattern([], 0.0);
                }
            }
            if self.state.stroke.as_ref().map(|s| &s.miter_limit) != Some(miter_limit) {
                self.content.set_miter_limit(miter_limit.get() as f32);
            }
            self.state.stroke = Some(stroke.clone());
        }
    }

    pub fn set_stroke_color_space(&mut self, space: Name<'static>) {
        if self.state.stroke_space != Some(space) {
            self.content.set_stroke_color_space(ColorSpaceOperand::Named(space));
            self.state.stroke_space = Some(space);
        }
    }

    pub fn reset_stroke_color_space(&mut self) {
        self.state.stroke_space = None;
    }

    fn set_text_rendering_mode(&mut self, mode: TextRenderingMode) {
        if self.state.text_rendering_mode != mode {
            self.content.set_text_rendering_mode(mode);
            self.state.text_rendering_mode = mode;
        }
    }
}

/// Encode a frame into the content stream.
pub(crate) fn write_frame(ctx: &mut PageContext, frame: &Frame) {
    for &(pos, ref item) in frame.items() {
        let x = pos.x.to_f32();
        let y = pos.y.to_f32();
        match item {
            FrameItem::Group(group) => write_group(ctx, pos, group),
            FrameItem::Text(text) => write_text(ctx, pos, text),
            FrameItem::Shape(shape, _) => write_shape(ctx, pos, shape),
            FrameItem::Image(image, size, _) => write_image(ctx, x, y, image, *size),
            FrameItem::Meta(meta, size) => match meta {
                Meta::Link(dest) => write_link(ctx, pos, dest, *size),
                Meta::Elem(_) => {}
                Meta::Hide => {}
            },
        }
    }
}

/// Encode a group into the content stream.
fn write_group(ctx: &mut PageContext, pos: Point, group: &GroupItem) {
    let translation = Transform::translate(pos.x, pos.y);

    ctx.save_state();

    if group.frame.kind().is_hard() {
        ctx.group_transform(
            ctx.state
                .transform
                .post_concat(ctx.state.container_transform.invert().unwrap())
                .pre_concat(translation)
                .pre_concat(group.transform),
        );
        ctx.size(group.frame.size());
    }

    ctx.transform(translation.pre_concat(group.transform));
    if let Some(clip_path) = &group.clip_path {
        write_path(ctx, 0.0, 0.0, clip_path);
        ctx.content.clip_nonzero();
        ctx.content.end_path();
    }

    write_frame(ctx, &group.frame);
    ctx.restore_state();
}

/// Encode a text run into the content stream.
fn write_text(ctx: &mut PageContext, pos: Point, text: &TextItem) {
    let ttf = text.font.ttf();
    let tables = ttf.tables();

    // If the text run contains either only color glyphs (used for emojis for
    // example) or normal text we can render it directly
    let has_color_glyphs = tables.sbix.is_some()
        || tables.cbdt.is_some()
        || tables.svg.is_some()
        || tables.colr.is_some();
    if !has_color_glyphs {
        write_normal_text(ctx, pos, TextItemView::all_of(text));
        return;
    }

    let color_glyph_count =
        text.glyphs.iter().filter(|g| is_color_glyph(&text.font, g)).count();

    if color_glyph_count == text.glyphs.len() {
        write_color_glyphs(ctx, pos, TextItemView::all_of(text));
    } else if color_glyph_count == 0 {
        write_normal_text(ctx, pos, TextItemView::all_of(text));
    } else {
        // Otherwise we need to split it in smaller text runs
        let mut offset = 0;
        let mut position_in_run = Abs::zero();
        for (color, sub_run) in
            text.glyphs.group_by_key(|g| is_color_glyph(&text.font, g))
        {
            let end = offset + sub_run.len();

            // Build a sub text-run
            let text_item_view = TextItemView::from_glyph_range(text, offset..end);

            // Adjust the position of the run on the line
            let pos = pos + Point::new(position_in_run, Abs::zero());
            position_in_run += text_item_view.width();
            offset = end;
            // Actually write the sub text-run
            if color {
                write_color_glyphs(ctx, pos, text_item_view);
            } else {
                write_normal_text(ctx, pos, text_item_view);
            }
        }
    }
}

// Encodes a text run (without any color glyph) into the content stream.
fn write_normal_text(ctx: &mut PageContext, pos: Point, text: TextItemView) {
    let x = pos.x.to_f32();
    let y = pos.y.to_f32();

    *ctx.parent.languages.entry(text.item.lang).or_insert(0) += text.glyph_range.len();

    let glyph_set = ctx.parent.glyph_sets.entry(text.item.font.clone()).or_default();
    for g in text.glyphs() {
        let t = text.text();
        let segment = &t[g.range()];
        glyph_set.entry(g.id).or_insert_with(|| segment.into());
    }

    let fill_transform = ctx.state.transforms(Size::zero(), pos);
    ctx.set_fill(&text.item.fill, true, fill_transform);

    let stroke = text.item.stroke.as_ref().and_then(|stroke| {
        if stroke.thickness.to_f32() > 0.0 {
            Some(stroke)
        } else {
            None
        }
    });

    if let Some(stroke) = stroke {
        ctx.set_stroke(stroke, true, fill_transform);
        ctx.set_text_rendering_mode(TextRenderingMode::FillStroke);
    } else {
        ctx.set_text_rendering_mode(TextRenderingMode::Fill);
    }

    ctx.set_font(&text.item.font, text.item.size);
    ctx.set_opacities(text.item.stroke.as_ref(), Some(&text.item.fill));
    ctx.content.begin_text();

    // Position the text.
    ctx.content.set_text_matrix([1.0, 0.0, 0.0, -1.0, x, y]);

    let mut positioned = ctx.content.show_positioned();
    let mut items = positioned.items();
    let mut adjustment = Em::zero();
    let mut encoded = vec![];

    // Write the glyphs with kerning adjustments.
    for glyph in text.glyphs() {
        adjustment += glyph.x_offset;

        if !adjustment.is_zero() {
            if !encoded.is_empty() {
                items.show(Str(&encoded));
                encoded.clear();
            }

            items.adjust(-adjustment.to_font_units());
            adjustment = Em::zero();
        }

        let cid = crate::font::glyph_cid(&text.item.font, glyph.id);
        encoded.push((cid >> 8) as u8);
        encoded.push((cid & 0xff) as u8);

        if let Some(advance) = text.item.font.advance(glyph.id) {
            adjustment += glyph.x_advance - advance;
        }

        adjustment -= glyph.x_offset;
    }

    if !encoded.is_empty() {
        items.show(Str(&encoded));
    }

    items.finish();
    positioned.finish();
    ctx.content.end_text();
}

// Encodes a text run made only of color glyphs into the content stream
fn write_color_glyphs(ctx: &mut PageContext, pos: Point, text: TextItemView) {
    let x = pos.x.to_f32();
    let y = pos.y.to_f32();

    let mut last_font = None;

    ctx.content.begin_text();
    ctx.content.set_text_matrix([1.0, 0.0, 0.0, -1.0, x, y]);
    // So that the next call to ctx.set_font() will change the font to one that
    // displays regular glyphs and not color glyphs.
    ctx.state.font = None;

    let glyph_set = ctx.parent.glyph_sets.entry(text.item.font.clone()).or_default();

    for glyph in text.glyphs() {
        // Retrieve the Type3 font reference and the glyph index in the font.
        let (font, index) = ctx.parent.color_font_map.get(
            &mut ctx.parent.alloc,
            &text.item.font,
            glyph.id,
        );

        if last_font != Some(font.get()) {
            ctx.content.set_font(
                Name(eco_format!("Cf{}", font.get()).as_bytes()),
                text.item.size.to_f32(),
            );
            last_font = Some(font.get());
        }

        ctx.content.show(Str(&[index]));

        glyph_set
            .entry(glyph.id)
            .or_insert_with(|| text.text()[glyph.range()].into());
    }
    ctx.content.end_text();
}

/// Encode a geometrical shape into the content stream.
fn write_shape(ctx: &mut PageContext, pos: Point, shape: &Shape) {
    let x = pos.x.to_f32();
    let y = pos.y.to_f32();

    let stroke = shape.stroke.as_ref().and_then(|stroke| {
        if stroke.thickness.to_f32() > 0.0 {
            Some(stroke)
        } else {
            None
        }
    });

    if shape.fill.is_none() && stroke.is_none() {
        return;
    }

    if let Some(fill) = &shape.fill {
        ctx.set_fill(fill, false, ctx.state.transforms(shape.geometry.bbox_size(), pos));
    }

    if let Some(stroke) = stroke {
        ctx.set_stroke(
            stroke,
            false,
            ctx.state.transforms(shape.geometry.bbox_size(), pos),
        );
    }

    ctx.set_opacities(stroke, shape.fill.as_ref());

    match shape.geometry {
        Geometry::Line(target) => {
            let dx = target.x.to_f32();
            let dy = target.y.to_f32();
            ctx.content.move_to(x, y);
            ctx.content.line_to(x + dx, y + dy);
        }
        Geometry::Rect(size) => {
            let w = size.x.to_f32();
            let h = size.y.to_f32();
            if w.abs() > f32::EPSILON && h.abs() > f32::EPSILON {
                ctx.content.rect(x, y, w, h);
            }
        }
        Geometry::Path(ref path) => {
            write_path(ctx, x, y, path);
        }
    }

    match (&shape.fill, stroke) {
        (None, None) => unreachable!(),
        (Some(_), None) => ctx.content.fill_nonzero(),
        (None, Some(_)) => ctx.content.stroke(),
        (Some(_), Some(_)) => ctx.content.fill_nonzero_and_stroke(),
    };
}

/// Encode a bezier path into the content stream.
fn write_path(ctx: &mut PageContext, x: f32, y: f32, path: &Path) {
    for elem in &path.0 {
        match elem {
            PathItem::MoveTo(p) => {
                ctx.content.move_to(x + p.x.to_f32(), y + p.y.to_f32())
            }
            PathItem::LineTo(p) => {
                ctx.content.line_to(x + p.x.to_f32(), y + p.y.to_f32())
            }
            PathItem::CubicTo(p1, p2, p3) => ctx.content.cubic_to(
                x + p1.x.to_f32(),
                y + p1.y.to_f32(),
                x + p2.x.to_f32(),
                y + p2.y.to_f32(),
                x + p3.x.to_f32(),
                y + p3.y.to_f32(),
            ),
            PathItem::ClosePath => ctx.content.close_path(),
        };
    }
}

/// Encode a vector or raster image into the content stream.
fn write_image(ctx: &mut PageContext, x: f32, y: f32, image: &Image, size: Size) {
    let index = ctx.parent.image_map.insert(image.clone());
    ctx.parent
        .image_deferred_map
        .entry(index)
        .or_insert_with(|| deferred_image(image.clone()));

    let name = eco_format!("Im{index}");
    let w = size.x.to_f32();
    let h = size.y.to_f32();
    ctx.content.save_state();
    ctx.content.transform([w, 0.0, 0.0, -h, x, y + h]);

    if let Some(alt) = image.alt() {
        let mut image_span =
            ctx.content.begin_marked_content_with_properties(Name(b"Span"));
        let mut image_alt = image_span.properties();
        image_alt.pair(Name(b"Alt"), pdf_writer::Str(alt.as_bytes()));
        image_alt.finish();
        image_span.finish();

        ctx.content.x_object(Name(name.as_bytes()));
        ctx.content.end_marked_content();
    } else {
        ctx.content.x_object(Name(name.as_bytes()));
    }

    ctx.resources
        .insert(PageResource::new(ResourceKind::XObject, name.clone()), index);
    ctx.content.restore_state();
}

/// Save a link for later writing in the annotations dictionary.
fn write_link(ctx: &mut PageContext, pos: Point, dest: &Destination, size: Size) {
    let mut min_x = Abs::inf();
    let mut min_y = Abs::inf();
    let mut max_x = -Abs::inf();
    let mut max_y = -Abs::inf();

    // Compute the bounding box of the transformed link.
    for point in [
        pos,
        pos + Point::with_x(size.x),
        pos + Point::with_y(size.y),
        pos + size.to_point(),
    ] {
        let t = point.transform(ctx.state.transform);
        min_x.set_min(t.x);
        min_y.set_min(t.y);
        max_x.set_max(t.x);
        max_y.set_max(t.y);
    }

    let x1 = min_x.to_f32();
    let x2 = max_x.to_f32();
    let y1 = max_y.to_f32();
    let y2 = min_y.to_f32();
    let rect = Rect::new(x1, y1, x2, y2);

    ctx.links.push((dest.clone(), rect));
}

fn to_pdf_line_cap(cap: LineCap) -> LineCapStyle {
    match cap {
        LineCap::Butt => LineCapStyle::ButtCap,
        LineCap::Round => LineCapStyle::RoundCap,
        LineCap::Square => LineCapStyle::ProjectingSquareCap,
    }
}

fn to_pdf_line_join(join: LineJoin) -> LineJoinStyle {
    match join {
        LineJoin::Miter => LineJoinStyle::MiterJoin,
        LineJoin::Round => LineJoinStyle::RoundJoin,
        LineJoin::Bevel => LineJoinStyle::BevelJoin,
    }
}

fn to_pdf_numbering_style(style: PdfPageLabelStyle) -> NumberingStyle {
    match style {
        PdfPageLabelStyle::Arabic => NumberingStyle::Arabic,
        PdfPageLabelStyle::LowerRoman => NumberingStyle::LowerRoman,
        PdfPageLabelStyle::UpperRoman => NumberingStyle::UpperRoman,
        PdfPageLabelStyle::LowerAlpha => NumberingStyle::LowerAlpha,
        PdfPageLabelStyle::UpperAlpha => NumberingStyle::UpperAlpha,
    }
}
