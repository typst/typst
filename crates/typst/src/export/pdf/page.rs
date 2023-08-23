use ecow::eco_format;
use pdf_writer::types::{
    ActionType, AnnotationType, ColorSpaceOperand, LineCapStyle, LineJoinStyle,
};
use pdf_writer::writers::ColorSpace;
use pdf_writer::{Content, Filter, Finish, Name, Rect, Ref, Str};

use super::external_graphics_state::ExternalGraphicsState;
use super::{deflate, AbsExt, EmExt, PdfContext, RefExt, D65_GRAY, SRGB};
use crate::doc::{Destination, Frame, FrameItem, GroupItem, Meta, TextItem};
use crate::font::Font;
use crate::geom::{
    self, Abs, Color, Em, Geometry, LineCap, LineJoin, Numeric, Paint, Point, Ratio,
    Shape, Size, Stroke, Transform,
};
use crate::image::Image;

/// Construct page objects.
#[tracing::instrument(skip_all)]
pub fn construct_pages(ctx: &mut PdfContext, frames: &[Frame]) {
    for frame in frames {
        construct_page(ctx, frame);
    }
}

/// Construct a page object.
#[tracing::instrument(skip_all)]
pub fn construct_page(ctx: &mut PdfContext, frame: &Frame) {
    let page_ref = ctx.alloc.bump();
    ctx.page_refs.push(page_ref);
    ctx.page_heights.push(frame.height().to_f32());

    let mut ctx = PageContext {
        parent: ctx,
        page_ref,
        uses_opacities: false,
        content: Content::new(),
        state: State::default(),
        saves: vec![],
        bottom: 0.0,
        links: vec![],
    };

    let size = frame.size();

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

    let page = Page {
        size,
        content: ctx.content,
        id: ctx.page_ref,
        uses_opacities: ctx.uses_opacities,
        links: ctx.links,
    };

    ctx.parent.pages.push(page);
}

/// Write the page tree.
#[tracing::instrument(skip_all)]
pub fn write_page_tree(ctx: &mut PdfContext) {
    for page in std::mem::take(&mut ctx.pages).into_iter() {
        write_page(ctx, page);
    }

    let mut pages = ctx.writer.pages(ctx.page_tree_ref);
    pages
        .count(ctx.page_refs.len() as i32)
        .kids(ctx.page_refs.iter().copied());

    let mut resources = pages.resources();
    let mut spaces = resources.color_spaces();
    spaces.insert(SRGB).start::<ColorSpace>().srgb();
    spaces.insert(D65_GRAY).start::<ColorSpace>().d65_gray();
    spaces.finish();

    let mut fonts = resources.fonts();
    for (font_ref, f) in ctx.font_map.pdf_indices(&ctx.font_refs) {
        let name = eco_format!("F{}", f);
        fonts.pair(Name(name.as_bytes()), font_ref);
    }

    fonts.finish();

    let mut images = resources.x_objects();
    for (image_ref, im) in ctx.image_map.pdf_indices(&ctx.image_refs) {
        let name = eco_format!("Im{}", im);
        images.pair(Name(name.as_bytes()), image_ref);
    }

    images.finish();

    let mut ext_gs_states = resources.ext_g_states();
    for (gs_ref, gs) in ctx.ext_gs_map.pdf_indices(&ctx.ext_gs_refs) {
        let name = eco_format!("Gs{}", gs);
        ext_gs_states.pair(Name(name.as_bytes()), gs_ref);
    }
    ext_gs_states.finish();

    resources.finish();
    pages.finish();
}

/// Write a page tree node.
#[tracing::instrument(skip_all)]
fn write_page(ctx: &mut PdfContext, page: Page) {
    let content_id = ctx.alloc.bump();

    let mut page_writer = ctx.writer.page(page.id);
    page_writer.parent(ctx.page_tree_ref);

    let w = page.size.x.to_f32();
    let h = page.size.y.to_f32();
    page_writer.media_box(Rect::new(0.0, 0.0, w, h));
    page_writer.contents(content_id);

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
    for (dest, rect) in page.links {
        let mut annotation = annotations.push();
        annotation.subtype(AnnotationType::Link).rect(rect);
        annotation.border(0.0, 0.0, 0.0, None);

        let pos = match dest {
            Destination::Url(uri) => {
                annotation
                    .action()
                    .action_type(ActionType::Uri)
                    .uri(Str(uri.as_bytes()));
                continue;
            }
            Destination::Position(pos) => pos,
            Destination::Location(loc) => ctx.introspector.position(loc),
        };

        let index = pos.page.get() - 1;
        let y = (pos.point.y - Abs::pt(10.0)).max(Abs::zero());
        if let Some(&height) = ctx.page_heights.get(index) {
            annotation
                .action()
                .action_type(ActionType::GoTo)
                .destination()
                .page(ctx.page_refs[index])
                .xyz(pos.point.x.to_f32(), height - y.to_f32(), None);
        }
    }

    annotations.finish();
    page_writer.finish();

    let data = page.content.finish();
    let data = deflate(&data);
    ctx.writer.stream(content_id, &data).filter(Filter::FlateDecode);
}

/// Data for an exported page.
pub struct Page {
    /// The indirect object id of the page.
    pub id: Ref,
    /// The page's dimensions.
    pub size: Size,
    /// The page's content stream.
    pub content: Content,
    /// Whether the page uses opacities.
    pub uses_opacities: bool,
    /// Links in the PDF coordinate system.
    pub links: Vec<(Destination, Rect)>,
}

/// An exporter for the contents of a single PDF page.
struct PageContext<'a, 'b> {
    parent: &'a mut PdfContext<'b>,
    page_ref: Ref,
    content: Content,
    state: State,
    saves: Vec<State>,
    bottom: f32,
    uses_opacities: bool,
    links: Vec<(Destination, Rect)>,
}

/// A simulated graphics state used to deduplicate graphics state changes and
/// keep track of the current transformation matrix for link annotations.
#[derive(Debug, Default, Clone)]
struct State {
    transform: Transform,
    font: Option<(Font, Abs)>,
    fill: Option<Paint>,
    fill_space: Option<Name<'static>>,
    external_graphics_state: Option<ExternalGraphicsState>,
    stroke: Option<Stroke>,
    stroke_space: Option<Name<'static>>,
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

    fn set_external_graphics_state(&mut self, graphics_state: &ExternalGraphicsState) {
        let current_state = self.state.external_graphics_state.as_ref();
        if current_state != Some(graphics_state) {
            self.parent.ext_gs_map.insert(*graphics_state);
            let name = eco_format!("Gs{}", self.parent.ext_gs_map.map(*graphics_state));
            self.content.set_parameters(Name(name.as_bytes()));

            if graphics_state.uses_opacities() {
                self.uses_opacities = true;
            }
        }
    }

    fn set_opacities(&mut self, stroke: Option<&Stroke>, fill: Option<&Paint>) {
        let stroke_opacity = stroke
            .map(|stroke| {
                let Paint::Solid(color) = stroke.paint;
                if let Color::Rgba(rgba_color) = color {
                    rgba_color.a
                } else {
                    255
                }
            })
            .unwrap_or(255);
        let fill_opacity = fill
            .map(|paint| {
                let Paint::Solid(color) = paint;
                if let Color::Rgba(rgba_color) = color {
                    rgba_color.a
                } else {
                    255
                }
            })
            .unwrap_or(255);
        self.set_external_graphics_state(&ExternalGraphicsState {
            stroke_opacity,
            fill_opacity,
        });
    }

    fn transform(&mut self, transform: Transform) {
        let Transform { sx, ky, kx, sy, tx, ty } = transform;
        self.state.transform = self.state.transform.pre_concat(transform);
        self.content.transform([
            sx.get() as _,
            ky.get() as _,
            kx.get() as _,
            sy.get() as _,
            tx.to_f32(),
            ty.to_f32(),
        ]);
    }

    fn set_font(&mut self, font: &Font, size: Abs) {
        if self.state.font.as_ref().map(|(f, s)| (f, *s)) != Some((font, size)) {
            self.parent.font_map.insert(font.clone());
            let name = eco_format!("F{}", self.parent.font_map.map(font.clone()));
            self.content.set_font(Name(name.as_bytes()), size.to_f32());
            self.state.font = Some((font.clone(), size));
        }
    }

    fn set_fill(&mut self, fill: &Paint) {
        if self.state.fill.as_ref() != Some(fill) {
            let f = |c| c as f32 / 255.0;
            let Paint::Solid(color) = fill;
            match color {
                Color::Luma(c) => {
                    self.set_fill_color_space(D65_GRAY);
                    self.content.set_fill_gray(f(c.0));
                }
                Color::Rgba(c) => {
                    self.set_fill_color_space(SRGB);
                    self.content.set_fill_color([f(c.r), f(c.g), f(c.b)]);
                }
                Color::Cmyk(c) => {
                    self.reset_fill_color_space();
                    self.content.set_fill_cmyk(f(c.c), f(c.m), f(c.y), f(c.k));
                }
            }
            self.state.fill = Some(fill.clone());
        }
    }

    fn set_fill_color_space(&mut self, space: Name<'static>) {
        if self.state.fill_space != Some(space) {
            self.content.set_fill_color_space(ColorSpaceOperand::Named(space));
            self.state.fill_space = Some(space);
        }
    }

    fn reset_fill_color_space(&mut self) {
        self.state.fill_space = None;
    }

    fn set_stroke(&mut self, stroke: &Stroke) {
        if self.state.stroke.as_ref() != Some(stroke) {
            let Stroke {
                paint,
                thickness,
                line_cap,
                line_join,
                dash_pattern,
                miter_limit,
            } = stroke;

            let f = |c| c as f32 / 255.0;
            let Paint::Solid(color) = paint;
            match color {
                Color::Luma(c) => {
                    self.set_stroke_color_space(D65_GRAY);
                    self.content.set_stroke_gray(f(c.0));
                }
                Color::Rgba(c) => {
                    self.set_stroke_color_space(SRGB);
                    self.content.set_stroke_color([f(c.r), f(c.g), f(c.b)]);
                }
                Color::Cmyk(c) => {
                    self.reset_stroke_color_space();
                    self.content.set_stroke_cmyk(f(c.c), f(c.m), f(c.y), f(c.k));
                }
            }

            self.content.set_line_width(thickness.to_f32());
            if self.state.stroke.as_ref().map(|s| &s.line_cap) != Some(line_cap) {
                self.content.set_line_cap(line_cap.into());
            }
            if self.state.stroke.as_ref().map(|s| &s.line_join) != Some(line_join) {
                self.content.set_line_join(line_join.into());
            }
            if self.state.stroke.as_ref().map(|s| &s.dash_pattern) != Some(dash_pattern) {
                if let Some(pattern) = dash_pattern {
                    self.content.set_dash_pattern(
                        pattern.array.iter().map(|l| l.to_f32()),
                        pattern.phase.to_f32(),
                    );
                } else {
                    self.content.set_dash_pattern([], 0.0);
                }
            }
            if self.state.stroke.as_ref().map(|s| &s.miter_limit) != Some(miter_limit) {
                self.content.set_miter_limit(miter_limit.0 as f32);
            }
            self.state.stroke = Some(stroke.clone());
        }
    }

    fn set_stroke_color_space(&mut self, space: Name<'static>) {
        if self.state.stroke_space != Some(space) {
            self.content.set_stroke_color_space(ColorSpaceOperand::Named(space));
            self.state.stroke_space = Some(space);
        }
    }

    fn reset_stroke_color_space(&mut self) {
        self.state.stroke_space = None;
    }
}

/// Encode a frame into the content stream.
fn write_frame(ctx: &mut PageContext, frame: &Frame) {
    for &(pos, ref item) in frame.items() {
        let x = pos.x.to_f32();
        let y = pos.y.to_f32();
        match item {
            FrameItem::Group(group) => write_group(ctx, pos, group),
            FrameItem::Text(text) => write_text(ctx, x, y, text),
            FrameItem::Shape(shape, _) => write_shape(ctx, x, y, shape),
            FrameItem::Image(image, size, _) => write_image(ctx, x, y, image, *size),
            FrameItem::Meta(meta, size) => match meta {
                Meta::Link(dest) => write_link(ctx, pos, dest, *size),
                Meta::Elem(_) => {}
                Meta::Hide => {}
                Meta::PageNumbering(_) => {}
            },
        }
    }
}

/// Encode a group into the content stream.
fn write_group(ctx: &mut PageContext, pos: Point, group: &GroupItem) {
    let translation = Transform::translate(pos.x, pos.y);

    ctx.save_state();
    ctx.transform(translation.pre_concat(group.transform));

    if group.clips {
        let size = group.frame.size();
        let w = size.x.to_f32();
        let h = size.y.to_f32();
        ctx.content.move_to(0.0, 0.0);
        ctx.content.line_to(w, 0.0);
        ctx.content.line_to(w, h);
        ctx.content.line_to(0.0, h);
        ctx.content.clip_nonzero();
        ctx.content.end_path();
    }

    write_frame(ctx, &group.frame);
    ctx.restore_state();
}

/// Encode a text run into the content stream.
fn write_text(ctx: &mut PageContext, x: f32, y: f32, text: &TextItem) {
    *ctx.parent.languages.entry(text.lang).or_insert(0) += text.glyphs.len();

    let glyph_set = ctx.parent.glyph_sets.entry(text.font.clone()).or_default();
    for g in &text.glyphs {
        let segment = &text.text[g.range()];
        glyph_set.entry(g.id).or_insert_with(|| segment.into());
    }

    ctx.set_fill(&text.fill);
    ctx.set_font(&text.font, text.size);
    ctx.set_opacities(None, Some(&text.fill));
    ctx.content.begin_text();

    // Positiosn the text.
    ctx.content.set_text_matrix([1.0, 0.0, 0.0, -1.0, x, y]);

    let mut positioned = ctx.content.show_positioned();
    let mut items = positioned.items();
    let mut adjustment = Em::zero();
    let mut encoded = vec![];

    // Write the glyphs with kerning adjustments.
    for glyph in &text.glyphs {
        adjustment += glyph.x_offset;

        if !adjustment.is_zero() {
            if !encoded.is_empty() {
                items.show(Str(&encoded));
                encoded.clear();
            }

            items.adjust(-adjustment.to_font_units());
            adjustment = Em::zero();
        }

        encoded.push((glyph.id >> 8) as u8);
        encoded.push((glyph.id & 0xff) as u8);

        if let Some(advance) = text.font.advance(glyph.id) {
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

/// Encode a geometrical shape into the content stream.
fn write_shape(ctx: &mut PageContext, x: f32, y: f32, shape: &Shape) {
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
        ctx.set_fill(fill);
    }

    if let Some(stroke) = stroke {
        ctx.set_stroke(stroke);
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
            if w > 0.0 && h > 0.0 {
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
fn write_path(ctx: &mut PageContext, x: f32, y: f32, path: &geom::Path) {
    for elem in &path.0 {
        match elem {
            geom::PathItem::MoveTo(p) => {
                ctx.content.move_to(x + p.x.to_f32(), y + p.y.to_f32())
            }
            geom::PathItem::LineTo(p) => {
                ctx.content.line_to(x + p.x.to_f32(), y + p.y.to_f32())
            }
            geom::PathItem::CubicTo(p1, p2, p3) => ctx.content.cubic_to(
                x + p1.x.to_f32(),
                y + p1.y.to_f32(),
                x + p2.x.to_f32(),
                y + p2.y.to_f32(),
                x + p3.x.to_f32(),
                y + p3.y.to_f32(),
            ),
            geom::PathItem::ClosePath => ctx.content.close_path(),
        };
    }
}

/// Encode a vector or raster image into the content stream.
fn write_image(ctx: &mut PageContext, x: f32, y: f32, image: &Image, size: Size) {
    ctx.parent.image_map.insert(image.clone());
    let name = eco_format!("Im{}", ctx.parent.image_map.map(image.clone()));
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

impl From<&LineCap> for LineCapStyle {
    fn from(line_cap: &LineCap) -> Self {
        match line_cap {
            LineCap::Butt => LineCapStyle::ButtCap,
            LineCap::Round => LineCapStyle::RoundCap,
            LineCap::Square => LineCapStyle::ProjectingSquareCap,
        }
    }
}

impl From<&LineJoin> for LineJoinStyle {
    fn from(line_join: &LineJoin) -> Self {
        match line_join {
            LineJoin::Miter => LineJoinStyle::MiterJoin,
            LineJoin::Round => LineJoinStyle::RoundJoin,
            LineJoin::Bevel => LineJoinStyle::BevelJoin,
        }
    }
}
