use crate::{paint, AbsExt};
use bytemuck::TransparentWrapper;
use image::{GenericImageView};
use krilla::action::{Action, LinkAction};
use krilla::annotation::{LinkAnnotation, Target};
use krilla::destination::XyzDestination;
use krilla::font::{GlyphId, GlyphUnits};
use krilla::geom::{Point, Transform};
use krilla::path::PathBuilder;
use krilla::surface::Surface;
use krilla::validation::Validator;
use krilla::version::PdfVersion;
use krilla::{PageSettings, SerializeSettings, SvgSettings};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::ops::Range;
use std::sync::Arc;
use svg2pdf::usvg::Rect;
use typst_library::layout::{Abs, Frame, FrameItem, GroupItem, PagedDocument, Size};
use typst_library::model::Destination;
use typst_library::text::{Font, Glyph, TextItem};
use typst_library::visualize::{FillRule, Geometry, Image, ImageKind, Paint, Path, PathItem, Shape};
use crate::content_old::Transforms;
use crate::primitive::{PointExt, SizeExt, TransformExt};

#[derive(Debug, Clone)]
struct State {
    /// The transform of the current item.
    transform: typst_library::layout::Transform,
    /// The transform of first hard frame in the hierarchy.
    container_transform: typst_library::layout::Transform,
    /// The size of the first hard frame in the hierarchy.
    size: Size,
}

impl State {
    /// Creates a new, clean state for a given `size`.
    pub fn new(size: Size) -> Self {
        Self {
            transform: typst_library::layout::Transform::identity(),
            container_transform: typst_library::layout::Transform::identity(),
            size,
        }
    }

    pub fn transform(&mut self, transform: typst_library::layout::Transform) {
        self.transform = self.transform.pre_concat(transform);
        if self.container_transform.is_identity() {
            self.container_transform = self.transform;
        }
    }

    fn group_transform(&mut self, transform: typst_library::layout::Transform) {
        self.container_transform =
            self.container_transform.pre_concat(transform);
    }

    /// Creates the [`Transforms`] structure for the current item.
    pub fn transforms(&self, size: Size, pos: typst_library::layout::Point) -> Transforms {
        Transforms {
            transform: self.transform.pre_concat(typst_library::layout::Transform::translate(pos.x, pos.y)),
            container_transform: self.container_transform,
            container_size: self.size,
            size,
        }
    }
}

struct FrameContext {
    states: Vec<State>
}

impl FrameContext {
    pub fn new(size: Size) -> Self {
        Self {
            states: vec![State::new(size)],
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

    pub fn state_mut(&mut self) -> &State {
        self.states.last_mut().unwrap()
    }
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

pub struct GlobalContext {
    fonts: HashMap<Font, krilla::font::Font>,
    cur_transform: typst_library::layout::Transform,
    annotations: Vec<krilla::annotation::Annotation>,
}

impl GlobalContext {
    pub fn new() -> Self {
        Self {
            fonts: Default::default(),
            cur_transform: typst_library::layout::Transform::identity(),
            annotations: vec![],
        }
    }
}

// TODO: Change rustybuzz cluster behavior so it works with ActualText

#[typst_macros::time(name = "write pdf")]
pub fn pdf(typst_document: &PagedDocument) -> Vec<u8> {
    let settings = SerializeSettings {
        compress_content_streams: true,
        no_device_cs: false,
        ascii_compatible: false,
        xmp_metadata: true,
        cmyk_profile: None,
        validator: Validator::None,
        enable_tagging: false,
        pdf_version: PdfVersion::Pdf17,
    };

    let mut document = krilla::Document::new_with(settings);
    let mut context = GlobalContext::new();

    for typst_page in &typst_document.pages {
        let settings = PageSettings::new(
            typst_page.frame.width().to_f32(),
            typst_page.frame.height().to_f32(),
        );
        let mut page = document.start_page_with(settings);
        let mut surface = page.surface();
        process_frame(&typst_page.frame, &mut surface, &mut context);
        surface.finish();

        let annotations = std::mem::take(&mut context.annotations);
        for annotation in annotations {
            page.add_annotation(annotation);
        }
    }

    finish(document)
}

#[typst_macros::time(name = "finish document")]
pub fn finish(document: krilla::Document) -> Vec<u8> {
    // TODO: Don't unwrap
    document.finish().unwrap()
}

pub fn process_frame(frame: &Frame, fill: Option<Paint>, surface: &mut Surface, gc: &mut GlobalContext) {
    let mut fc = FrameContext::new(frame.size());

    for (point, item) in frame.items() {
        surface.push_transform(&Transform::from_translate(
            point.x.to_f32(),
            point.y.to_f32(),
        ));

        match item {
            FrameItem::Group(g) => handle_group(g, surface, gc),
            FrameItem::Text(t) => handle_text(t, surface, gc),
            FrameItem::Shape(s, _) => handle_shape(s, surface),
            FrameItem::Image(image, size, span) => {
                handle_image(image, *size, surface, gc)
            }
            FrameItem::Link(d, s) => handle_link(*point, d, *s, gc, surface),
            FrameItem::Tag(_) => {}
        }

        surface.pop();
    }
}

pub fn handle_group(
    group: &GroupItem,
    surface: &mut Surface,
    context: &mut GlobalContext,
) {
    let old = context.cur_transform;
    context.cur_transform = context.cur_transform.pre_concat(group.transform);

    surface.push_transform(&group.transform.as_krilla());
    process_frame(&group.frame, surface, context);

    context.cur_transform = old;
    surface.pop();
}

pub fn handle_text(t: &TextItem, surface: &mut Surface, context: &mut GlobalContext) {
    let font = context
        .fonts
        .entry(t.font.clone())
        .or_insert_with(|| {
            krilla::font::Font::new(Arc::new(t.font.data().clone()), t.font.index(), true)
                // TODO: DOn't unwrap
                .unwrap()
        })
        .clone();
    let fill = paint::fill(&t.fill, FillRule::NonZero);
    let text = t.text.as_str();
    let size = t.size;

    let glyphs: &[PdfGlyph] = TransparentWrapper::wrap_slice(t.glyphs.as_slice());

    surface.fill_glyphs(
        Point::from_xy(0.0, 0.0),
        fill,
        &glyphs,
        font.clone(),
        text,
        size.to_f32(),
        GlyphUnits::Normalized,
        false,
    );

    if let Some(stroke) = t.stroke.as_ref().map(paint::stroke) {
        surface.stroke_glyphs(
            Point::from_xy(0.0, 0.0),
            stroke,
            &glyphs,
            font.clone(),
            text,
            size.to_f32(),
            GlyphUnits::Normalized,
            true,
        );
    }
}

pub fn handle_image(
    image: &Image,
    size: Size,
    surface: &mut Surface,
    _: &mut GlobalContext,
) {
    match image.kind() {
        ImageKind::Raster(raster) => {
            // TODO: Don't unwrap
            let image = crate::image::raster(raster.clone()).unwrap();
            surface.draw_image(image, size.as_krilla());
        }
        ImageKind::Svg(svg) => {
            surface.draw_svg(svg.tree(), size.as_krilla(), SvgSettings::default());
        }
    }
}

pub fn handle_shape(fc: &FrameContext, pos: Point, shape: &Shape, surface: &mut Surface) {
    let mut path_builder = PathBuilder::new();

    match &shape.geometry {
        Geometry::Line(l) => {
            path_builder.move_to(0.0, 0.0);
            path_builder.line_to(l.x.to_f32(), l.y.to_f32());
        }
        Geometry::Rect(r) => {
            path_builder.push_rect(
                Rect::from_xywh(0.0, 0.0, r.x.to_f32(), r.y.to_f32()).unwrap(),
            );
        }
        Geometry::Path(p) => {
            convert_path(p, &mut path_builder);
        }
    }

    surface.push_transform(&fc.state().transform.as_krilla());
    surface.push_transform(&Transform::from_translate(pos.x, pos.y));

    if let Some(path) = path_builder.finish() {
        if let Some(paint) = &shape.fill {
            let fill = paint::fill(paint, shape.fill_rule);
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
            let stroke = paint::stroke(stroke);
            surface.stroke_path(&path, stroke);
        }
    }

    surface.pop();
    surface.pop();
}

pub fn convert_path(path: &Path, builder: &mut PathBuilder) {
    for item in &path.0 {
        match item {
            PathItem::MoveTo(p) => builder.move_to(p.x.to_f32(), p.y.to_f32()),
            PathItem::LineTo(p) => builder.line_to(p.x.to_f32(), p.y.to_f32()),
            PathItem::CubicTo(p1, p2, p3) => builder.cubic_to(
                p1.x.to_f32(),
                p1.y.to_f32(),
                p2.x.to_f32(),
                p2.y.to_f32(),
                p3.x.to_f32(),
                p3.y.to_f32(),
            ),
            PathItem::ClosePath => builder.close(),
        }
    }
}

fn handle_link(
    pos: typst_library::layout::Point,
    dest: &Destination,
    size: typst_library::layout::Size,
    ctx: &mut GlobalContext,
    surface: &mut Surface,
) {
    let mut min_x = Abs::inf();
    let mut min_y = Abs::inf();
    let mut max_x = -Abs::inf();
    let mut max_y = -Abs::inf();

    // Compute the bounding box of the transformed link.
    for point in [
        pos,
        pos + typst_library::layout::Point::with_x(size.x),
        pos + typst_library::layout::Point::with_y(size.y),
        pos + size.to_point(),
    ] {
        let t = point.transform(ctx.cur_transform);
        min_x.set_min(t.x);
        min_y.set_min(t.y);
        max_x.set_max(t.x);
        max_y.set_max(t.y);
    }

    let x1 = min_x.to_f32();
    let x2 = max_x.to_f32();
    let y1 = min_y.to_f32();
    let y2 = max_y.to_f32();
    let rect = krilla::geom::Rect::from_ltrb(x1, y1, x2, y2).unwrap();

    let target = match dest {
        Destination::Url(u) => {
            Target::Action(Action::Link(LinkAction::new(u.to_string())))
        }
        Destination::Position(p) => {
            Target::Destination(krilla::destination::Destination::Xyz(
                XyzDestination::new(p.page.get() - 1, p.point.as_krilla()),
            ))
        }
        Destination::Location(_) => return,
    };

    ctx.annotations.push(LinkAnnotation::new(rect, target).into());
}
