//! Generic writer for PDF content.
//!
//! It is used to write page contents, color glyph instructions, and patterns.
//!
//! See also [`pdf_writer::Content`].

use ecow::eco_format;
use pdf_writer::{
    types::{ColorSpaceOperand, LineCapStyle, LineJoinStyle, TextRenderingMode},
    Content, Finish, Name, Rect, Str,
};
use typst::layout::{
    Abs, Em, Frame, FrameItem, GroupItem, Point, Ratio, Size, Transform,
};
use typst::model::Destination;
use typst::text::{color::is_color_glyph, Font, TextItem, TextItemView};
use typst::utils::{Deferred, Numeric, SliceExt};
use typst::visualize::{
    FixedStroke, Geometry, Image, LineCap, LineJoin, Paint, Path, PathItem, Shape,
};

use crate::color_font::ColorFontMap;
use crate::extg::ExtGState;
use crate::image::deferred_image;
use crate::{color::PaintEncode, resources::Resources};
use crate::{deflate_deferred, AbsExt, EmExt};

/// Encode a [`Frame`] into a content stream.
///
/// The resources that were used in the stream will be added to `resources`.
///
/// `color_glyph_width` should be `None` unless the `Frame` represents a [color
/// glyph].
///
/// [color glyph]: `crate::color_font`
pub fn build(
    resources: &mut Resources<()>,
    frame: &Frame,
    color_glyph_width: Option<f32>,
) -> Encoded {
    let size = frame.size();
    let mut ctx = Builder::new(resources, size);

    if let Some(width) = color_glyph_width {
        ctx.content.start_color_glyph(width);
    }

    // Make the coordinate system start at the top-left.
    ctx.transform(
        // Make the Y axis go upwards
        Transform::scale(Ratio::one(), -Ratio::one())
            // Also move the origin to the top left corner
            .post_concat(Transform::translate(Abs::zero(), size.y)),
    );

    // Encode the frame into the content stream.
    write_frame(&mut ctx, frame);

    Encoded {
        size,
        content: deflate_deferred(ctx.content.finish()),
        uses_opacities: ctx.uses_opacities,
        links: ctx.links,
    }
}

/// An encoded content stream.
pub struct Encoded {
    /// The dimensions of the content.
    pub size: Size,
    /// The actual content stream.
    pub content: Deferred<Vec<u8>>,
    /// Whether the content opacities.
    pub uses_opacities: bool,
    /// Links in the PDF coordinate system.
    pub links: Vec<(Destination, Rect)>,
}

/// An exporter for a single PDF content stream.
///
/// Content streams are a series of PDF commands. They can reference external
/// objects only through resources.
///
/// Content streams can be used for page contents, but also to describe color
/// glyphs and patterns.
pub struct Builder<'a, R = ()> {
    /// A list of all resources that are used in the content stream.
    pub(crate) resources: &'a mut Resources<R>,
    /// The PDF content stream that is being built.
    pub content: Content,
    /// Current graphic state.
    state: State,
    /// Stack of saved graphic states.
    saves: Vec<State>,
    /// Wheter any stroke or fill was not totally opaque.
    uses_opacities: bool,
    /// All clickable links that are present in this content.
    links: Vec<(Destination, Rect)>,
}

impl<'a, R> Builder<'a, R> {
    /// Create a new content builder.
    pub fn new(resources: &'a mut Resources<R>, size: Size) -> Self {
        Builder {
            resources,
            uses_opacities: false,
            content: Content::new(),
            state: State::new(size),
            saves: vec![],
            links: vec![],
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
    /// The current font.
    font: Option<(Font, Abs)>,
    /// The current fill paint.
    fill: Option<Paint>,
    /// The color space of the current fill paint.
    fill_space: Option<Name<'static>>,
    /// The current external graphic state.
    external_graphics_state: Option<ExtGState>,
    /// The current stroke paint.
    stroke: Option<FixedStroke>,
    /// The color space of the current stroke paint.
    stroke_space: Option<Name<'static>>,
    /// The current text rendering mode.
    text_rendering_mode: TextRenderingMode,
}

impl State {
    /// Creates a new, clean state for a given `size`.
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

impl Builder<'_, ()> {
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
            let index = self.resources.ext_gs.insert(*graphics_state);
            let name = eco_format!("Gs{index}");
            self.content.set_parameters(Name(name.as_bytes()));

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
            let index = self.resources.fonts.insert(font.clone());
            let name = eco_format!("F{index}");
            self.content.set_font(Name(name.as_bytes()), size.to_f32());
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
pub(crate) fn write_frame(ctx: &mut Builder, frame: &Frame) {
    for &(pos, ref item) in frame.items() {
        let x = pos.x.to_f32();
        let y = pos.y.to_f32();
        match item {
            FrameItem::Group(group) => write_group(ctx, pos, group),
            FrameItem::Text(text) => write_text(ctx, pos, text),
            FrameItem::Shape(shape, _) => write_shape(ctx, pos, shape),
            FrameItem::Image(image, size, _) => write_image(ctx, x, y, image, *size),
            FrameItem::Link(dest, size) => write_link(ctx, pos, dest, *size),
            FrameItem::Tag(_) => {}
        }
    }
}

/// Encode a group into the content stream.
fn write_group(ctx: &mut Builder, pos: Point, group: &GroupItem) {
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
fn write_text(ctx: &mut Builder, pos: Point, text: &TextItem) {
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

/// Encodes a text run (without any color glyph) into the content stream.
fn write_normal_text(ctx: &mut Builder, pos: Point, text: TextItemView) {
    let x = pos.x.to_f32();
    let y = pos.y.to_f32();

    *ctx.resources.languages.entry(text.item.lang).or_insert(0) += text.glyph_range.len();

    let glyph_set = ctx.resources.glyph_sets.entry(text.item.font.clone()).or_default();
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

/// Encodes a text run made only of color glyphs into the content stream
fn write_color_glyphs(ctx: &mut Builder, pos: Point, text: TextItemView) {
    let x = pos.x.to_f32();
    let y = pos.y.to_f32();

    let mut last_font = None;

    ctx.content.begin_text();
    ctx.content.set_text_matrix([1.0, 0.0, 0.0, -1.0, x, y]);
    // So that the next call to ctx.set_font() will change the font to one that
    // displays regular glyphs and not color glyphs.
    ctx.state.font = None;

    let glyph_set = ctx.resources.glyph_sets.entry(text.item.font.clone()).or_default();

    for glyph in text.glyphs() {
        // Retrieve the Type3 font reference and the glyph index in the font.
        let color_fonts = ctx
            .resources
            .color_fonts
            .get_or_insert_with(|| Box::new(ColorFontMap::new()));
        let (font, index) = color_fonts.get(&text.item.font, glyph.id);

        if last_font != Some(font) {
            ctx.content.set_font(
                Name(eco_format!("Cf{}", font).as_bytes()),
                text.item.size.to_f32(),
            );
            last_font = Some(font);
        }

        ctx.content.show(Str(&[index]));

        glyph_set
            .entry(glyph.id)
            .or_insert_with(|| text.text()[glyph.range()].into());
    }
    ctx.content.end_text();
}

/// Encode a geometrical shape into the content stream.
fn write_shape(ctx: &mut Builder, pos: Point, shape: &Shape) {
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
fn write_path(ctx: &mut Builder, x: f32, y: f32, path: &Path) {
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
fn write_image(ctx: &mut Builder, x: f32, y: f32, image: &Image, size: Size) {
    let index = ctx.resources.images.insert(image.clone());
    ctx.resources.deferred_images.entry(index).or_insert_with(|| {
        let (image, color_space) = deferred_image(image.clone());
        if let Some(color_space) = color_space {
            ctx.resources.colors.mark_as_used(color_space);
        }
        image
    });

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

    ctx.content.restore_state();
}

/// Save a link for later writing in the annotations dictionary.
fn write_link(ctx: &mut Builder, pos: Point, dest: &Destination, size: Size) {
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
