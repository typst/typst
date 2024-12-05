use crate::{paint, primitive, AbsExt};
use bytemuck::TransparentWrapper;
use image::{DynamicImage, GenericImageView, Rgba};
use krilla::action::{Action, LinkAction};
use krilla::annotation::{LinkAnnotation, Target};
use krilla::destination::XyzDestination;
use krilla::font::{GlyphId, GlyphUnits};
use krilla::geom::{Point, Transform};
use krilla::image::{BitsPerComponent, CustomImage, ImageColorspace};
use krilla::path::PathBuilder;
use krilla::surface::Surface;
use krilla::validation::Validator;
use krilla::version::PdfVersion;
use krilla::{PageSettings, SerializeSettings, SvgSettings};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::ops::Range;
use std::sync::{Arc, OnceLock};
use svg2pdf::usvg::Rect;
use typst_library::layout::{Abs, Frame, FrameItem, GroupItem, Page, Size};
use typst_library::model::{Destination, Document};
use typst_library::text::{Font, Glyph, TextItem};
use typst_library::visualize::{
    FillRule, Geometry, Image, ImageKind, Path, PathItem, RasterFormat, RasterImage,
    Shape,
};
use crate::primitive::{PointExt, SizeExt, TransformExt};

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

pub struct ExportContext {
    fonts: HashMap<Font, krilla::font::Font>,
    cur_transform: typst_library::layout::Transform,
    annotations: Vec<krilla::annotation::Annotation>,
}

impl ExportContext {
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
pub fn pdf(typst_document: &Document) -> Vec<u8> {
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
    let mut context = ExportContext::new();

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

pub fn handle_group(
    group: &GroupItem,
    surface: &mut Surface,
    context: &mut ExportContext,
) {
    let old = context.cur_transform;
    context.cur_transform = context.cur_transform.pre_concat(group.transform);

    surface.push_transform(&group.transform.as_krilla());
    process_frame(&group.frame, surface, context);

    context.cur_transform = old;
    surface.pop();
}

pub fn handle_text(t: &TextItem, surface: &mut Surface, context: &mut ExportContext) {
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
    _: &mut ExportContext,
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

pub fn handle_shape(shape: &Shape, surface: &mut Surface) {
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

    if let Some(path) = path_builder.finish() {
        if let Some(paint) = &shape.fill {
            let fill = paint::fill(paint, shape.fill_rule);
            surface.fill_path(&path, fill);
        }

        if let Some(stroke) = &shape.stroke {
            let stroke = paint::stroke(stroke);
            surface.stroke_path(&path, stroke);
        }
    }
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

pub fn process_frame(frame: &Frame, surface: &mut Surface, context: &mut ExportContext) {
    for (point, item) in frame.items() {
        surface.push_transform(&Transform::from_translate(
            point.x.to_f32(),
            point.y.to_f32(),
        ));

        match item {
            FrameItem::Group(g) => handle_group(g, surface, context),
            FrameItem::Text(t) => handle_text(t, surface, context),
            FrameItem::Shape(s, _) => handle_shape(s, surface),
            FrameItem::Image(image, size, span) => {
                handle_image(image, *size, surface, context)
            }
            FrameItem::Link(d, s) => handle_link(*point, d, *s, context, surface),
            FrameItem::Tag(_) => {}
        }

        surface.pop();
    }
}

fn handle_link(
    pos: typst_library::layout::Point,
    dest: &Destination,
    size: typst_library::layout::Size,
    ctx: &mut ExportContext,
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
