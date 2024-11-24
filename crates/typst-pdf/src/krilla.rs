
use crate::{AbsExt};
use krilla::color::rgb;
use krilla::font::{GlyphId, GlyphUnits};
use krilla::geom::{Point, Transform};
use krilla::path::{Fill, PathBuilder, Stroke};
use krilla::surface::Surface;
use krilla::{PageSettings, SerializeSettings, SvgSettings};
use std::collections::HashMap;
use std::ops::Range;
use std::sync::Arc;
use bytemuck::TransparentWrapper;
use image::GenericImageView;
use krilla::validation::Validator;
use krilla::version::PdfVersion;
use svg2pdf::usvg::{NormalizedF32, Rect};
use typst_library::layout::{Frame, FrameItem, GroupItem, Size};
use typst_library::model::Document;
use typst_library::text::{Font, Glyph, TextItem};
use typst_library::visualize::{ColorSpace, FillRule, FixedStroke, Geometry, Image, ImageKind, LineCap, LineJoin, Paint, Path, PathItem, Shape};

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
}

impl ExportContext {
    pub fn new() -> Self {
        Self {
            fonts: Default::default(),
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
    surface.push_transform(&convert_transform(group.transform));
    process_frame(&group.frame, surface, context);
    surface.pop();
}

pub fn handle_text(t: &TextItem, surface: &mut Surface, context: &mut ExportContext) {
    let font = context
        .fonts
        .entry(t.font.clone())
        .or_insert_with(|| {
            krilla::font::Font::new(
                // TODO: Don't do to_vec here!
                Arc::new(t.font.data().to_vec()),
                t.font.index()
            )
                // TODO: DOn't unwrap
            .unwrap()
        })
        .clone();
    let (paint, opacity) = convert_paint(&t.fill);
    let fill = Fill {
        paint,
        opacity: NormalizedF32::new(opacity as f32 / 255.0).unwrap(),
        ..Default::default()
    };
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

    if let Some(stroke) = t.stroke.as_ref().map(convert_fixed_stroke) {
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

#[typst_macros::time(name = "handle image")]
pub fn handle_image(
    image: &Image,
    size: &Size,
    surface: &mut Surface,
    _: &mut ExportContext,
) {
    match image.kind() {
        ImageKind::Raster(raster) => {
            let image = krilla::image::Image::from_png(raster.data())
            .unwrap();
            surface.draw_image(
                image,
                krilla::geom::Size::from_wh(size.x.to_f32(), size.y.to_f32()).unwrap(),
            );
        }
        ImageKind::Svg(svg) => {
            surface.draw_svg(
                svg.tree(),
                krilla::geom::Size::from_wh(size.x.to_f32(), size.y.to_f32()).unwrap(),
                SvgSettings::default(),
            );
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
            let (paint, opacity) = convert_paint(paint);

            let fill = Fill {
                paint,
                rule: convert_fill_rule(shape.fill_rule),
                opacity: NormalizedF32::new(opacity as f32 / 255.0).unwrap(),
            };
            surface.fill_path(&path, fill);
        }

        if let Some(stroke) = &shape.stroke {
            let stroke = convert_fixed_stroke(stroke);

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
            FrameItem::Image(image, size, _) => {
                handle_image(image, size, surface, context)
            }
            FrameItem::Link(_, _) => {}
            FrameItem::Tag(_) => {}
        }

        surface.pop();
    }
}

fn convert_fill_rule(fill_rule: FillRule) -> krilla::path::FillRule {
    match fill_rule {
        FillRule::NonZero => krilla::path::FillRule::NonZero,
        FillRule::EvenOdd => krilla::path::FillRule::EvenOdd,
    }
}

fn convert_fixed_stroke(stroke: &FixedStroke) -> Stroke {
    let (paint, opacity) = convert_paint(&stroke.paint);
    Stroke {
        paint,
        width: stroke.thickness.to_f32(),
        miter_limit: stroke.miter_limit.get() as f32,
        line_join: convert_linejoin(stroke.join),
        line_cap: convert_linecap(stroke.cap),
        opacity: NormalizedF32::new(opacity as f32 / 255.0).unwrap(),
        ..Default::default()
    }
}

fn convert_linecap(l: LineCap) -> krilla::path::LineCap {
    match l {
        LineCap::Butt => krilla::path::LineCap::Butt,
        LineCap::Round => krilla::path::LineCap::Round,
        LineCap::Square => krilla::path::LineCap::Square,
    }
}

fn convert_linejoin(l: LineJoin) -> krilla::path::LineJoin {
    match l {
        LineJoin::Miter => krilla::path::LineJoin::Miter,
        LineJoin::Round => krilla::path::LineJoin::Round,
        LineJoin::Bevel => krilla::path::LineJoin::Bevel,
    }
}

fn convert_transform(t: crate::Transform) -> krilla::geom::Transform {
    Transform::from_row(
        t.sx.get() as f32,
        t.ky.get() as f32,
        t.kx.get() as f32,
        t.sy.get() as f32,
        t.tx.to_f32(),
        t.ty.to_f32(),
    )
}

fn convert_paint(paint: &Paint) -> (krilla::paint::Paint, u8) {
    match paint {
        Paint::Solid(c) => {
            let components = c.to_space(ColorSpace::Srgb).to_vec4_u8();
            (
                rgb::Color::new(components[0], components[1], components[2]).into(),
                components[3],
            )
        }
        Paint::Gradient(_) => (rgb::Color::black().into(), 255),
        Paint::Pattern(_) => (rgb::Color::black().into(), 255),
    }
}
