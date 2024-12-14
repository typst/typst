use crate::content_old::Transforms;
use crate::primitive::{PointExt, SizeExt, TransformExt};
use crate::{paint, AbsExt};
use bytemuck::TransparentWrapper;
use image::GenericImageView;
use krilla::action::{Action, LinkAction};
use krilla::annotation::{LinkAnnotation, Target};
use krilla::destination::XyzDestination;
use krilla::font::{GlyphId, GlyphUnits};
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
use typst_library::layout::{
    Abs, Frame, FrameItem, GroupItem, PagedDocument, Point, Size, Transform,
};
use typst_library::model::Destination;
use typst_library::text::{Font, Glyph, TextItem};
use typst_library::visualize::{
    FillRule, Geometry, Image, ImageKind, Paint, Path, PathItem, Shape,
};

#[derive(Debug, Clone)]
struct State {
    /// The transform of the current item.
    transform: Transform,
    /// The transform of first hard frame in the hierarchy.
    container_transform: Transform,
    /// The size of the first hard frame in the hierarchy.
    size: Size,
}

impl State {
    /// Creates a new, clean state for a given `size`.
    fn new(size: Size) -> Self {
        Self {
            transform: Transform::identity(),
            container_transform: Transform::identity(),
            size,
        }
    }

    pub fn size(&mut self, size: Size) {
        self.size = size;
    }

    pub fn transform(&mut self, transform: Transform) {
        self.transform = self.transform.pre_concat(transform);
    }

    fn set_container_transform(&mut self) {
        self.container_transform = self.transform;
    }

    /// Creates the [`Transforms`] structure for the current item.
    pub fn transforms(&self, size: Size) -> Transforms {
        Transforms {
            transform: self.transform,
            container_transform: self.container_transform,
            container_size: self.size,
            size,
        }
    }
}

pub(crate) struct FrameContext {
    states: Vec<State>,
}

impl FrameContext {
    pub fn new(size: Size) -> Self {
        Self { states: vec![State::new(size)] }
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
    annotations: Vec<krilla::annotation::Annotation>,
}

impl GlobalContext {
    pub fn new() -> Self {
        Self { fonts: Default::default(), annotations: vec![] }
    }
}

// TODO: Change rustybuzz cluster behavior so it works with ActualText

#[typst_macros::time(name = "write pdf")]
pub fn pdf(typst_document: &PagedDocument) -> Vec<u8> {
    let settings = SerializeSettings {
        compress_content_streams: true,
        no_device_cs: true,
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
        let mut fc = FrameContext::new(typst_page.frame.size());
        // println!("{:?}", &typst_page.frame);
        process_frame(
            &mut fc,
            &typst_page.frame,
            typst_page.fill_or_transparent(),
            &mut surface,
            &mut context,
        );
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

pub fn process_frame(
    fc: &mut FrameContext,
    frame: &Frame,
    fill: Option<Paint>,
    surface: &mut Surface,
    gc: &mut GlobalContext,
) {
    fc.push();

    if frame.kind().is_hard() {
        fc.state_mut().set_container_transform();
        fc.state_mut().size(frame.size());
    }

    if let Some(fill) = fill {
        let shape = Geometry::Rect(frame.size()).filled(fill);
        handle_shape(fc, &shape, surface, gc);
    }

    for (point, item) in frame.items() {
        fc.push();
        fc.state_mut().transform(Transform::translate(point.x, point.y));
        match item {
            FrameItem::Group(g) => handle_group(fc, g, surface, gc),
            FrameItem::Text(t) => handle_text(fc, t, surface, gc),
            FrameItem::Shape(s, _) => handle_shape(fc, s, surface, gc),
            FrameItem::Image(image, size, span) => {
                handle_image(fc, image, *size, surface)
            }
            FrameItem::Link(d, s) => {}
            FrameItem::Tag(_) => {}
        }

        fc.pop();
    }

    fc.pop();
}

pub fn handle_group(
    fc: &mut FrameContext,
    group: &GroupItem,
    surface: &mut Surface,
    context: &mut GlobalContext,
) {
    fc.push();
    fc.state_mut().transform(group.transform);

    let clip_path = group
        .clip_path
        .as_ref()
        .and_then(|p| {
            let mut builder = PathBuilder::new();
            convert_path(p, &mut builder);
            builder.finish()
        })
        .and_then(|p| p.transform(fc.state().transform.as_krilla()));

    if let Some(clip_path) = &clip_path {
        surface.push_clip_path(clip_path, &krilla::path::FillRule::NonZero);
    }

    process_frame(fc, &group.frame, None, surface, context);

    if clip_path.is_some() {
        surface.pop();
    }

    fc.pop();
}

pub fn handle_text(
    fc: &mut FrameContext,
    t: &TextItem,
    surface: &mut Surface,
    gc: &mut GlobalContext,
) {
    let font = gc
        .fonts
        .entry(t.font.clone())
        .or_insert_with(|| {
            krilla::font::Font::new(Arc::new(t.font.data().clone()), t.font.index(), true)
                // TODO: DOn't unwrap
                .unwrap()
        })
        .clone();
    let fill = paint::fill(
        gc,
        &t.fill,
        FillRule::NonZero,
        true,
        surface,
        fc.state().transforms(Size::zero()),
    );
    let text = t.text.as_str();
    let size = t.size;

    let glyphs: &[PdfGlyph] = TransparentWrapper::wrap_slice(t.glyphs.as_slice());

    surface.push_transform(&fc.state().transform.as_krilla());

    surface.fill_glyphs(
        krilla::geom::Point::from_xy(0.0, 0.0),
        fill,
        &glyphs,
        font.clone(),
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
        surface.stroke_glyphs(
            krilla::geom::Point::from_xy(0.0, 0.0),
            stroke,
            &glyphs,
            font.clone(),
            text,
            size.to_f32(),
            GlyphUnits::Normalized,
            true,
        );
    }

    surface.pop();
}

pub fn handle_image(
    fc: &mut FrameContext,
    image: &Image,
    size: Size,
    surface: &mut Surface,
) {
    surface.push_transform(&fc.state().transform.as_krilla());

    match image.kind() {
        ImageKind::Raster(raster) => {
            // TODO: Don't unwrap
            let image = crate::image::raster(raster.clone()).unwrap();
            surface.draw_image(image, size.as_krilla());
        }
        ImageKind::Svg(svg) => {
            surface.draw_svg(
                svg.tree(),
                size.as_krilla(),
                SvgSettings {
                    embed_text: !svg.flatten_text(),
                    ..Default::default()
                },
            );
        }
    }

    surface.pop();
}

pub fn handle_shape(
    fc: &mut FrameContext,
    shape: &Shape,
    surface: &mut Surface,
    gc: &mut GlobalContext,
) {
    let mut path_builder = PathBuilder::new();

    match &shape.geometry {
        Geometry::Line(l) => {
            path_builder.move_to(0.0, 0.0);
            path_builder.line_to(l.x.to_f32(), l.y.to_f32());
        }
        Geometry::Rect(r) => {
            if let Some(r) = Rect::from_xywh(0.0, 0.0, r.x.to_f32(), r.y.to_f32()) {
                path_builder.push_rect(r);
            }
        }
        Geometry::Path(p) => {
            convert_path(p, &mut path_builder);
        }
    }

    surface.push_transform(&fc.state().transform.as_krilla());

    if let Some(path) = path_builder.finish() {
        if let Some(paint) = &shape.fill {
            let fill = paint::fill(
                gc,
                &paint,
                shape.fill_rule,
                false,
                surface,
                fc.state().transforms(Size::zero()),
            );
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
                fc.state().transforms(Size::zero()),
            );
            surface.stroke_path(&path, stroke);
        }
    }

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

// fn handle_link(
//     pos: typst_library::layout::Point,
//     dest: &Destination,
//     size: typst_library::layout::Size,
//     ctx: &mut GlobalContext,
//     surface: &mut Surface,
// ) {
//     let mut min_x = Abs::inf();
//     let mut min_y = Abs::inf();
//     let mut max_x = -Abs::inf();
//     let mut max_y = -Abs::inf();
//
//     // Compute the bounding box of the transformed link.
//     for point in [
//         pos,
//         pos + Point::with_x(size.x),
//         pos + Point::with_y(size.y),
//         pos + size.to_point(),
//     ] {
//         let t = point.transform(ctx.cur_transform);
//         min_x.set_min(t.x);
//         min_y.set_min(t.y);
//         max_x.set_max(t.x);
//         max_y.set_max(t.y);
//     }
//
//     let x1 = min_x.to_f32();
//     let x2 = max_x.to_f32();
//     let y1 = min_y.to_f32();
//     let y2 = max_y.to_f32();
//     let rect = krilla::geom::Rect::from_ltrb(x1, y1, x2, y2).unwrap();
//
//     let target = match dest {
//         Destination::Url(u) => {
//             Target::Action(Action::Link(LinkAction::new(u.to_string())))
//         }
//         Destination::Position(p) => {
//             Target::Destination(krilla::destination::Destination::Xyz(
//                 XyzDestination::new(p.page.get() - 1, p.point.as_krilla()),
//             ))
//         }
//         Destination::Location(_) => return,
//     };
//
//     ctx.annotations.push(LinkAnnotation::new(rect, target).into());
// }
