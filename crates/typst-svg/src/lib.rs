//! Rendering of Typst documents into SVG images.

use std::collections::HashMap;
use std::f32::consts::TAU;
use std::fmt::{self, Display, Formatter, Write};
use std::io::Read;

use base64::Engine;
use ecow::{eco_format, EcoString};
use ttf_parser::{GlyphId, OutlineBuilder};
use typst::foundations::Repr;
use typst::layout::{
    Abs, Angle, Axes, Frame, FrameItem, FrameKind, GroupItem, Point, Quadrant, Ratio,
    Size, Transform,
};
use typst::model::Document;
use typst::text::{Font, TextItem};
use typst::util::hash128;
use typst::visualize::{
    Color, FixedStroke, Geometry, Gradient, Image, ImageFormat, LineCap, LineJoin, Paint,
    Path, PathItem, Pattern, RasterFormat, RatioOrAngle, RelativeTo, Shape, VectorFormat,
};
use xmlwriter::XmlWriter;

/// The number of segments in a conic gradient.
/// This is a heuristic value that seems to work well.
/// Smaller values could be interesting for optimization.
const CONIC_SEGMENT: usize = 360;

/// Export a frame into a SVG file.
#[typst_macros::time(name = "svg")]
pub fn svg(frame: &Frame) -> String {
    let mut renderer = SVGRenderer::new();
    renderer.write_header(frame.size());

    let state = State::new(frame.size(), Transform::identity());
    renderer.render_frame(state, Transform::identity(), frame);
    renderer.finalize()
}

/// Export a document with potentially multiple pages into a single SVG file.
///
/// The padding will be added around and between the individual frames.
pub fn svg_merged(document: &Document, padding: Abs) -> String {
    let width = 2.0 * padding
        + document
            .pages
            .iter()
            .map(|page| page.frame.width())
            .max()
            .unwrap_or_default();
    let height = padding
        + document
            .pages
            .iter()
            .map(|page| page.frame.height() + padding)
            .sum::<Abs>();

    let mut renderer = SVGRenderer::new();
    renderer.write_header(Size::new(width, height));

    let [x, mut y] = [padding; 2];
    for page in &document.pages {
        let ts = Transform::translate(x, y);
        let state = State::new(page.frame.size(), Transform::identity());
        renderer.render_frame(state, ts, &page.frame);
        y += page.frame.height() + padding;
    }

    renderer.finalize()
}

/// Renders one or multiple frames to an SVG file.
struct SVGRenderer {
    /// The internal XML writer.
    xml: XmlWriter,
    /// Prepared glyphs.
    glyphs: Deduplicator<RenderedGlyph>,
    /// Clip paths are used to clip a group. A clip path is a path that defines
    /// the clipping region. The clip path is referenced by the `clip-path`
    /// attribute of the group. The clip path is in the format of `M x y L x y C
    /// x1 y1 x2 y2 x y Z`.
    clip_paths: Deduplicator<EcoString>,
    /// Deduplicated gradients with transform matrices. They use a reference
    /// (`href`) to a "source" gradient instead of being defined inline.
    /// This saves a lot of space since gradients are often reused but with
    /// different transforms. Therefore this allows us to reuse the same gradient
    /// multiple times.
    gradient_refs: Deduplicator<GradientRef>,
    /// Deduplicated patterns with transform matrices. They use a reference
    /// (`href`) to a "source" pattern instead of being defined inline.
    /// This saves a lot of space since patterns are often reused but with
    /// different transforms. Therefore this allows us to reuse the same gradient
    /// multiple times.
    pattern_refs: Deduplicator<PatternRef>,
    /// These are the actual gradients being written in the SVG file.
    /// These gradients are deduplicated because they do not contain the transform
    /// matrix, allowing them to be reused across multiple invocations.
    ///
    /// The `Ratio` is the aspect ratio of the gradient, this is used to correct
    /// the angle of the gradient.
    gradients: Deduplicator<(Gradient, Ratio)>,
    /// These are the actual patterns being written in the SVG file.
    /// These patterns are deduplicated because they do not contain the transform
    /// matrix, allowing them to be reused across multiple invocations.
    ///
    /// The `String` is the rendered pattern frame.
    patterns: Deduplicator<Pattern>,
    /// These are the gradients that compose a conic gradient.
    conic_subgradients: Deduplicator<SVGSubGradient>,
}

/// Contextual information for rendering.
#[derive(Clone, Copy)]
struct State {
    /// The transform of the current item.
    transform: Transform,
    /// The size of the first hard frame in the hierarchy.
    size: Size,
}

impl State {
    fn new(size: Size, transform: Transform) -> Self {
        Self { size, transform }
    }

    /// Pre translate the current item's transform.
    fn pre_translate(self, pos: Point) -> Self {
        self.pre_concat(Transform::translate(pos.x, pos.y))
    }

    /// Pre concat the current item's transform.
    fn pre_concat(self, transform: Transform) -> Self {
        Self {
            transform: self.transform.pre_concat(transform),
            ..self
        }
    }

    /// Sets the size of the first hard frame in the hierarchy.
    fn with_size(self, size: Size) -> Self {
        Self { size, ..self }
    }

    /// Sets the current item's transform.
    fn with_transform(self, transform: Transform) -> Self {
        Self { transform, ..self }
    }
}

/// A reference to a deduplicated gradient, with a transform matrix.
///
/// Allows gradients to be reused across multiple invocations,
/// simply by changing the transform matrix.
#[derive(Hash)]
struct GradientRef {
    /// The ID of the deduplicated gradient
    id: Id,
    /// The gradient kind (used to determine the SVG element to use)
    /// but without needing to clone the entire gradient.
    kind: GradientKind,
    /// The transform matrix to apply to the gradient.
    transform: Transform,
}

/// A reference to a deduplicated pattern, with a transform matrix.
///
/// Allows patterns to be reused across multiple invocations,
/// simply by changing the transform matrix.
#[derive(Hash)]
struct PatternRef {
    /// The ID of the deduplicated gradient
    id: Id,
    /// The transform matrix to apply to the pattern.
    transform: Transform,
    /// The ratio of the size of the cell to the size of the filled area.
    ratio: Axes<Ratio>,
}

/// A subgradient for conic gradients.
#[derive(Hash)]
struct SVGSubGradient {
    /// The center point of the gradient.
    center: Axes<Ratio>,
    /// The start point of the subgradient.
    t0: Angle,
    /// The end point of the subgradient.
    t1: Angle,
    /// The color at the start point of the subgradient.
    c0: Color,
    /// The color at the end point of the subgradient.
    c1: Color,
}

/// The kind of linear gradient.
#[derive(Hash, Clone, Copy, PartialEq, Eq)]
enum GradientKind {
    /// A linear gradient.
    Linear,
    /// A radial gradient.
    Radial,
    /// A conic gradient.
    Conic,
}

impl From<&Gradient> for GradientKind {
    fn from(value: &Gradient) -> Self {
        match value {
            Gradient::Linear { .. } => GradientKind::Linear,
            Gradient::Radial { .. } => GradientKind::Radial,
            Gradient::Conic { .. } => GradientKind::Conic,
        }
    }
}

/// Represents a glyph to be rendered.
enum RenderedGlyph {
    /// A path is a sequence of drawing commands.
    ///
    /// It is in the format of `M x y L x y C x1 y1 x2 y2 x y Z`.
    Path(EcoString),
    /// An image is a URL to an image file, plus the size and transform.
    ///
    /// The url is in the format of `data:image/{format};base64,`.
    Image { url: EcoString, width: f64, height: f64, ts: Transform },
}

impl SVGRenderer {
    /// Create a new SVG renderer with empty glyph and clip path.
    fn new() -> Self {
        SVGRenderer {
            xml: XmlWriter::new(xmlwriter::Options::default()),
            glyphs: Deduplicator::new('g'),
            clip_paths: Deduplicator::new('c'),
            gradient_refs: Deduplicator::new('g'),
            gradients: Deduplicator::new('f'),
            conic_subgradients: Deduplicator::new('s'),
            pattern_refs: Deduplicator::new('p'),
            patterns: Deduplicator::new('t'),
        }
    }

    /// Write the SVG header, including the `viewBox` and `width` and `height`
    /// attributes.
    fn write_header(&mut self, size: Size) {
        self.xml.start_element("svg");
        self.xml.write_attribute("class", "typst-doc");
        self.xml.write_attribute_fmt(
            "viewBox",
            format_args!("0 0 {} {}", size.x.to_pt(), size.y.to_pt()),
        );
        self.xml
            .write_attribute_fmt("width", format_args!("{}pt", size.x.to_pt()));
        self.xml
            .write_attribute_fmt("height", format_args!("{}pt", size.y.to_pt()));
        self.xml.write_attribute("xmlns", "http://www.w3.org/2000/svg");
        self.xml
            .write_attribute("xmlns:xlink", "http://www.w3.org/1999/xlink");
        self.xml.write_attribute("xmlns:h5", "http://www.w3.org/1999/xhtml");
    }

    /// Render a frame to a string.
    fn render_pattern_frame(
        &mut self,
        state: State,
        ts: Transform,
        frame: &Frame,
    ) -> String {
        let mut xml = XmlWriter::new(xmlwriter::Options::default());
        std::mem::swap(&mut self.xml, &mut xml);
        self.render_frame(state, ts, frame);
        std::mem::swap(&mut self.xml, &mut xml);
        xml.end_document()
    }

    /// Render a frame with the given transform.
    fn render_frame(&mut self, state: State, ts: Transform, frame: &Frame) {
        self.xml.start_element("g");
        if !ts.is_identity() {
            self.xml.write_attribute("transform", &SvgMatrix(ts));
        }

        for (pos, item) in frame.items() {
            // File size optimization
            if matches!(item, FrameItem::Meta(_, _)) {
                continue;
            }

            let x = pos.x.to_pt();
            let y = pos.y.to_pt();
            self.xml.start_element("g");
            self.xml
                .write_attribute_fmt("transform", format_args!("translate({x} {y})"));

            match item {
                FrameItem::Group(group) => {
                    self.render_group(state.pre_translate(*pos), group)
                }
                FrameItem::Text(text) => {
                    self.render_text(state.pre_translate(*pos), text)
                }
                FrameItem::Shape(shape, _) => {
                    self.render_shape(state.pre_translate(*pos), shape)
                }
                FrameItem::Image(image, size, _) => self.render_image(image, size),
                FrameItem::Meta(_, _) => unreachable!(),
            };

            self.xml.end_element();
        }

        self.xml.end_element();
    }

    /// Render a group. If the group has `clips` set to true, a clip path will
    /// be created.
    fn render_group(&mut self, state: State, group: &GroupItem) {
        let state = match group.frame.kind() {
            FrameKind::Soft => state.pre_concat(group.transform),
            FrameKind::Hard => state
                .with_transform(Transform::identity())
                .with_size(group.frame.size()),
        };

        self.xml.start_element("g");
        self.xml.write_attribute("class", "typst-group");

        if let Some(clip_path) = &group.clip_path {
            let hash = hash128(&group);
            let id = self.clip_paths.insert_with(hash, || convert_path(clip_path));
            self.xml.write_attribute_fmt("clip-path", format_args!("url(#{id})"));
        }

        self.render_frame(state, group.transform, &group.frame);
        self.xml.end_element();
    }

    /// Render a text item. The text is rendered as a group of glyphs. We will
    /// try to render the text as SVG first, then bitmap, then outline. If none
    /// of them works, we will skip the text.
    fn render_text(&mut self, state: State, text: &TextItem) {
        let scale: f64 = text.size.to_pt() / text.font.units_per_em();

        self.xml.start_element("g");
        self.xml.write_attribute("class", "typst-text");
        self.xml.write_attribute("transform", "scale(1, -1)");

        let mut x: f64 = 0.0;
        for glyph in &text.glyphs {
            let id = GlyphId(glyph.id);
            let offset = x + glyph.x_offset.at(text.size).to_pt();

            self.render_svg_glyph(text, id, offset, scale)
                .or_else(|| self.render_bitmap_glyph(text, id, offset))
                .or_else(|| {
                    self.render_outline_glyph(
                        state
                            .pre_concat(Transform::scale(Ratio::one(), -Ratio::one()))
                            .pre_translate(Point::new(Abs::pt(offset), Abs::zero())),
                        text,
                        id,
                        offset,
                        scale,
                    )
                });

            x += glyph.x_advance.at(text.size).to_pt();
        }

        self.xml.end_element();
    }

    /// Render a glyph defined by an SVG.
    fn render_svg_glyph(
        &mut self,
        text: &TextItem,
        id: GlyphId,
        x_offset: f64,
        scale: f64,
    ) -> Option<()> {
        let data_url = convert_svg_glyph_to_base64_url(&text.font, id)?;
        let upem = Abs::raw(text.font.units_per_em());
        let origin_ascender = text.font.metrics().ascender.at(upem).to_pt();

        let glyph_hash = hash128(&(&text.font, id));
        let id = self.glyphs.insert_with(glyph_hash, || RenderedGlyph::Image {
            url: data_url,
            width: upem.to_pt(),
            height: upem.to_pt(),
            ts: Transform::translate(Abs::zero(), Abs::pt(-origin_ascender))
                .post_concat(Transform::scale(Ratio::new(scale), Ratio::new(-scale))),
        });

        self.xml.start_element("use");
        self.xml.write_attribute_fmt("xlink:href", format_args!("#{id}"));
        self.xml.write_attribute("x", &x_offset);
        self.xml.end_element();

        Some(())
    }

    /// Render a glyph defined by a bitmap.
    fn render_bitmap_glyph(
        &mut self,
        text: &TextItem,
        id: GlyphId,
        x_offset: f64,
    ) -> Option<()> {
        let (image, bitmap_x_offset, bitmap_y_offset) =
            convert_bitmap_glyph_to_image(&text.font, id)?;

        let glyph_hash = hash128(&(&text.font, id));
        let id = self.glyphs.insert_with(glyph_hash, || {
            let width = image.width();
            let height = image.height();
            let url = convert_image_to_base64_url(&image);
            let ts = Transform::translate(
                Abs::pt(bitmap_x_offset),
                Abs::pt(-height - bitmap_y_offset),
            );
            RenderedGlyph::Image { url, width, height, ts }
        });

        let target_height = text.size.to_pt();
        self.xml.start_element("use");
        self.xml.write_attribute_fmt("xlink:href", format_args!("#{id}"));

        // The image is stored with the height of `image.height()`, but we want
        // to render it with a height of `target_height`. So we need to scale
        // it.
        let scale_factor = target_height / image.height();
        self.xml.write_attribute("x", &(x_offset / scale_factor));
        self.xml.write_attribute_fmt(
            "transform",
            format_args!("scale({scale_factor} -{scale_factor})",),
        );
        self.xml.end_element();

        Some(())
    }

    /// Render a glyph defined by an outline.
    fn render_outline_glyph(
        &mut self,
        state: State,
        text: &TextItem,
        glyph_id: GlyphId,
        x_offset: f64,
        scale: f64,
    ) -> Option<()> {
        let scale = Ratio::new(scale);
        let path = convert_outline_glyph_to_path(&text.font, glyph_id, scale)?;
        let hash = hash128(&(&text.font, glyph_id, scale));
        let id = self.glyphs.insert_with(hash, || RenderedGlyph::Path(path));

        let glyph_size = text.font.ttf().glyph_bounding_box(glyph_id)?;
        let width = glyph_size.width() as f64 * scale.get();
        let height = glyph_size.height() as f64 * scale.get();

        self.xml.start_element("use");
        self.xml.write_attribute_fmt("xlink:href", format_args!("#{id}"));
        self.xml.write_attribute_fmt("x", format_args!("{x_offset}"));
        self.write_fill(
            &text.fill,
            Size::new(Abs::pt(width), Abs::pt(height)),
            self.text_paint_transform(state, &text.fill),
        );
        if let Some(stroke) = &text.stroke {
            self.write_stroke(
                stroke,
                Size::new(Abs::pt(width), Abs::pt(height)),
                self.text_paint_transform(state, &stroke.paint),
            );
        }
        self.xml.end_element();

        Some(())
    }

    fn text_paint_transform(&self, state: State, paint: &Paint) -> Transform {
        match paint {
            Paint::Solid(_) => Transform::identity(),
            Paint::Gradient(gradient) => match gradient.unwrap_relative(true) {
                RelativeTo::Self_ => Transform::identity(),
                RelativeTo::Parent => Transform::scale(
                    Ratio::new(state.size.x.to_pt()),
                    Ratio::new(state.size.y.to_pt()),
                )
                .post_concat(state.transform.invert().unwrap()),
            },
            Paint::Pattern(pattern) => match pattern.unwrap_relative(true) {
                RelativeTo::Self_ => Transform::identity(),
                RelativeTo::Parent => state.transform.invert().unwrap(),
            },
        }
    }

    /// Render a shape element.
    fn render_shape(&mut self, state: State, shape: &Shape) {
        self.xml.start_element("path");
        self.xml.write_attribute("class", "typst-shape");

        if let Some(paint) = &shape.fill {
            self.write_fill(
                paint,
                self.shape_fill_size(state, paint, shape),
                self.shape_paint_transform(state, paint, shape),
            );
        } else {
            self.xml.write_attribute("fill", "none");
        }

        if let Some(stroke) = &shape.stroke {
            self.write_stroke(
                stroke,
                self.shape_fill_size(state, &stroke.paint, shape),
                self.shape_paint_transform(state, &stroke.paint, shape),
            );
        }

        let path = convert_geometry_to_path(&shape.geometry);
        self.xml.write_attribute("d", &path);
        self.xml.end_element();
    }

    /// Calculate the transform of the shape's fill or stroke.
    fn shape_paint_transform(
        &self,
        state: State,
        paint: &Paint,
        shape: &Shape,
    ) -> Transform {
        let mut shape_size = shape.geometry.bbox_size();
        // Edge cases for strokes.
        if shape_size.x.to_pt() == 0.0 {
            shape_size.x = Abs::pt(1.0);
        }

        if shape_size.y.to_pt() == 0.0 {
            shape_size.y = Abs::pt(1.0);
        }

        if let Paint::Gradient(gradient) = paint {
            match gradient.unwrap_relative(false) {
                RelativeTo::Self_ => Transform::scale(
                    Ratio::new(shape_size.x.to_pt()),
                    Ratio::new(shape_size.y.to_pt()),
                ),
                RelativeTo::Parent => Transform::scale(
                    Ratio::new(state.size.x.to_pt()),
                    Ratio::new(state.size.y.to_pt()),
                )
                .post_concat(state.transform.invert().unwrap()),
            }
        } else if let Paint::Pattern(pattern) = paint {
            match pattern.unwrap_relative(false) {
                RelativeTo::Self_ => Transform::identity(),
                RelativeTo::Parent => state.transform.invert().unwrap(),
            }
        } else {
            Transform::identity()
        }
    }

    /// Calculate the size of the shape's fill.
    fn shape_fill_size(&self, state: State, paint: &Paint, shape: &Shape) -> Size {
        let mut shape_size = shape.geometry.bbox_size();
        // Edge cases for strokes.
        if shape_size.x.to_pt() == 0.0 {
            shape_size.x = Abs::pt(1.0);
        }

        if shape_size.y.to_pt() == 0.0 {
            shape_size.y = Abs::pt(1.0);
        }

        if let Paint::Gradient(gradient) = paint {
            match gradient.unwrap_relative(false) {
                RelativeTo::Self_ => shape_size,
                RelativeTo::Parent => state.size,
            }
        } else {
            shape_size
        }
    }

    /// Write a fill attribute.
    fn write_fill(&mut self, fill: &Paint, size: Size, ts: Transform) {
        match fill {
            Paint::Solid(color) => self.xml.write_attribute("fill", &color.encode()),
            Paint::Gradient(gradient) => {
                let id = self.push_gradient(gradient, size, ts);
                self.xml.write_attribute_fmt("fill", format_args!("url(#{id})"));
            }
            Paint::Pattern(pattern) => {
                let id = self.push_pattern(pattern, size, ts);
                self.xml.write_attribute_fmt("fill", format_args!("url(#{id})"));
            }
        }
    }

    /// Pushes a gradient to the list of gradients to write SVG file.
    ///
    /// If the gradient is already present, returns the id of the existing
    /// gradient. Otherwise, inserts the gradient and returns the id of the
    /// inserted gradient. If the transform of the gradient is the identify
    /// matrix, the returned ID will be the ID of the "source" gradient,
    /// this is a file size optimization.
    fn push_gradient(&mut self, gradient: &Gradient, size: Size, ts: Transform) -> Id {
        let gradient_id = self
            .gradients
            .insert_with(hash128(&(gradient, size.aspect_ratio())), || {
                (gradient.clone(), size.aspect_ratio())
            });

        if ts.is_identity() {
            return gradient_id;
        }

        self.gradient_refs
            .insert_with(hash128(&(gradient_id, ts)), || GradientRef {
                id: gradient_id,
                kind: gradient.into(),
                transform: ts,
            })
    }

    fn push_pattern(&mut self, pattern: &Pattern, size: Size, ts: Transform) -> Id {
        let pattern_size = pattern.size() + pattern.spacing();
        // Unfortunately due to a limitation of `xmlwriter`, we need to
        // render the frame twice: once to allocate all of the resources
        // that it needs and once to actually render it.
        self.render_pattern_frame(
            State::new(pattern_size, Transform::identity()),
            Transform::identity(),
            pattern.frame(),
        );

        let pattern_id = self.patterns.insert_with(hash128(pattern), || pattern.clone());
        self.pattern_refs
            .insert_with(hash128(&(pattern_id, ts)), || PatternRef {
                id: pattern_id,
                transform: ts,
                ratio: Axes::new(
                    Ratio::new(pattern_size.x.to_pt() / size.x.to_pt()),
                    Ratio::new(pattern_size.y.to_pt() / size.y.to_pt()),
                ),
            })
    }

    /// Write a stroke attribute.
    fn write_stroke(
        &mut self,
        stroke: &FixedStroke,
        size: Size,
        fill_transform: Transform,
    ) {
        match &stroke.paint {
            Paint::Solid(color) => self.xml.write_attribute("stroke", &color.encode()),
            Paint::Gradient(gradient) => {
                let id = self.push_gradient(gradient, size, fill_transform);
                self.xml.write_attribute_fmt("stroke", format_args!("url(#{id})"));
            }
            Paint::Pattern(pattern) => {
                let id = self.push_pattern(pattern, size, fill_transform);
                self.xml.write_attribute_fmt("stroke", format_args!("url(#{id})"));
            }
        }

        self.xml.write_attribute("stroke-width", &stroke.thickness.to_pt());
        self.xml.write_attribute(
            "stroke-linecap",
            match stroke.cap {
                LineCap::Butt => "butt",
                LineCap::Round => "round",
                LineCap::Square => "square",
            },
        );
        self.xml.write_attribute(
            "stroke-linejoin",
            match stroke.join {
                LineJoin::Miter => "miter",
                LineJoin::Round => "round",
                LineJoin::Bevel => "bevel",
            },
        );
        self.xml
            .write_attribute("stroke-miterlimit", &stroke.miter_limit.get());
        if let Some(pattern) = &stroke.dash {
            self.xml.write_attribute("stroke-dashoffset", &pattern.phase.to_pt());
            self.xml.write_attribute(
                "stroke-dasharray",
                &pattern
                    .array
                    .iter()
                    .map(|dash| dash.to_pt().to_string())
                    .collect::<Vec<_>>()
                    .join(" "),
            );
        }
    }

    /// Render an image element.
    fn render_image(&mut self, image: &Image, size: &Axes<Abs>) {
        let url = convert_image_to_base64_url(image);
        self.xml.start_element("image");
        self.xml.write_attribute("xlink:href", &url);
        self.xml.write_attribute("width", &size.x.to_pt());
        self.xml.write_attribute("height", &size.y.to_pt());
        self.xml.write_attribute("preserveAspectRatio", "none");
        self.xml.end_element();
    }

    /// Finalize the SVG file. This must be called after all rendering is done.
    fn finalize(mut self) -> String {
        self.write_glyph_defs();
        self.write_clip_path_defs();
        self.write_gradients();
        self.write_gradient_refs();
        self.write_subgradients();
        self.write_patterns();
        self.write_pattern_refs();
        self.xml.end_document()
    }

    /// Build the glyph definitions.
    fn write_glyph_defs(&mut self) {
        if self.glyphs.is_empty() {
            return;
        }

        self.xml.start_element("defs");
        self.xml.write_attribute("id", "glyph");

        for (id, glyph) in self.glyphs.iter() {
            self.xml.start_element("symbol");
            self.xml.write_attribute("id", &id);
            self.xml.write_attribute("overflow", "visible");

            match glyph {
                RenderedGlyph::Path(path) => {
                    self.xml.start_element("path");
                    self.xml.write_attribute("d", &path);
                    self.xml.end_element();
                }
                RenderedGlyph::Image { url, width, height, ts } => {
                    self.xml.start_element("image");
                    self.xml.write_attribute("xlink:href", &url);
                    self.xml.write_attribute("width", &width);
                    self.xml.write_attribute("height", &height);
                    if !ts.is_identity() {
                        self.xml.write_attribute("transform", &SvgMatrix(*ts));
                    }
                    self.xml.write_attribute("preserveAspectRatio", "none");
                    self.xml.end_element();
                }
            }

            self.xml.end_element();
        }

        self.xml.end_element();
    }

    /// Build the clip path definitions.
    fn write_clip_path_defs(&mut self) {
        if self.clip_paths.is_empty() {
            return;
        }

        self.xml.start_element("defs");
        self.xml.write_attribute("id", "clip-path");

        for (id, path) in self.clip_paths.iter() {
            self.xml.start_element("clipPath");
            self.xml.write_attribute("id", &id);
            self.xml.start_element("path");
            self.xml.write_attribute("d", &path);
            self.xml.end_element();
            self.xml.end_element();
        }

        self.xml.end_element();
    }

    /// Write the raw gradients (without transform) to the SVG file.
    fn write_gradients(&mut self) {
        if self.gradients.is_empty() {
            return;
        }

        self.xml.start_element("defs");
        self.xml.write_attribute("id", "gradients");

        for (id, (gradient, ratio)) in self.gradients.iter() {
            match &gradient {
                Gradient::Linear(linear) => {
                    self.xml.start_element("linearGradient");
                    self.xml.write_attribute("id", &id);
                    self.xml.write_attribute("spreadMethod", "pad");
                    self.xml.write_attribute("gradientUnits", "userSpaceOnUse");

                    let angle = Gradient::correct_aspect_ratio(linear.angle, *ratio);
                    let (sin, cos) = (angle.sin(), angle.cos());
                    let length = sin.abs() + cos.abs();
                    let (x1, y1, x2, y2) = match angle.quadrant() {
                        Quadrant::First => (0.0, 0.0, cos * length, sin * length),
                        Quadrant::Second => (1.0, 0.0, cos * length + 1.0, sin * length),
                        Quadrant::Third => {
                            (1.0, 1.0, cos * length + 1.0, sin * length + 1.0)
                        }
                        Quadrant::Fourth => (0.0, 1.0, cos * length, sin * length + 1.0),
                    };

                    self.xml.write_attribute("x1", &x1);
                    self.xml.write_attribute("y1", &y1);
                    self.xml.write_attribute("x2", &x2);
                    self.xml.write_attribute("y2", &y2);
                }
                Gradient::Radial(radial) => {
                    self.xml.start_element("radialGradient");
                    self.xml.write_attribute("id", &id);
                    self.xml.write_attribute("spreadMethod", "pad");
                    self.xml.write_attribute("gradientUnits", "userSpaceOnUse");
                    self.xml.write_attribute("cx", &radial.center.x.get());
                    self.xml.write_attribute("cy", &radial.center.y.get());
                    self.xml.write_attribute("r", &radial.radius.get());
                    self.xml.write_attribute("fx", &radial.focal_center.x.get());
                    self.xml.write_attribute("fy", &radial.focal_center.y.get());
                    self.xml.write_attribute("fr", &radial.focal_radius.get());
                }
                Gradient::Conic(conic) => {
                    self.xml.start_element("pattern");
                    self.xml.write_attribute("id", &id);
                    self.xml.write_attribute("viewBox", "0 0 1 1");
                    self.xml.write_attribute("preserveAspectRatio", "none");
                    self.xml.write_attribute("patternUnits", "userSpaceOnUse");
                    self.xml.write_attribute("width", "2");
                    self.xml.write_attribute("height", "2");
                    self.xml.write_attribute("x", "-0.5");
                    self.xml.write_attribute("y", "-0.5");

                    // The rotation angle, negated to match rotation in PNG.
                    let angle: f32 =
                        -(Gradient::correct_aspect_ratio(conic.angle, *ratio).to_rad()
                            as f32)
                            .rem_euclid(TAU);
                    let center: (f32, f32) =
                        (conic.center.x.get() as f32, conic.center.y.get() as f32);

                    // We build an arg segment for each segment of a circle.
                    let dtheta = TAU / CONIC_SEGMENT as f32;
                    for i in 0..CONIC_SEGMENT {
                        let theta1 = dtheta * i as f32;
                        let theta2 = dtheta * (i + 1) as f32;

                        // Create the path for the segment.
                        let mut builder = SvgPathBuilder::default();
                        builder.move_to(
                            correct_pattern_pos(center.0),
                            correct_pattern_pos(center.1),
                        );
                        builder.line_to(
                            correct_pattern_pos(-2.0 * (theta1 + angle).cos() + center.0),
                            correct_pattern_pos(2.0 * (theta1 + angle).sin() + center.1),
                        );
                        builder.arc(
                            (2.0, 2.0),
                            0.0,
                            0,
                            1,
                            (
                                correct_pattern_pos(
                                    -2.0 * (theta2 + angle).cos() + center.0,
                                ),
                                correct_pattern_pos(
                                    2.0 * (theta2 + angle).sin() + center.1,
                                ),
                            ),
                        );
                        builder.close();

                        let t1 = (i as f32) / CONIC_SEGMENT as f32;
                        let t2 = (i + 1) as f32 / CONIC_SEGMENT as f32;
                        let subgradient = SVGSubGradient {
                            center: conic.center,
                            t0: Angle::rad((theta1 + angle) as f64),
                            t1: Angle::rad((theta2 + angle) as f64),
                            c0: gradient
                                .sample(RatioOrAngle::Ratio(Ratio::new(t1 as f64))),
                            c1: gradient
                                .sample(RatioOrAngle::Ratio(Ratio::new(t2 as f64))),
                        };
                        let id = self
                            .conic_subgradients
                            .insert_with(hash128(&subgradient), || subgradient);

                        // Add the path to the pattern.
                        self.xml.start_element("path");
                        self.xml.write_attribute("d", &builder.0);
                        self.xml.write_attribute_fmt("fill", format_args!("url(#{id})"));
                        self.xml
                            .write_attribute_fmt("stroke", format_args!("url(#{id})"));
                        self.xml.write_attribute("stroke-width", "0");
                        self.xml.write_attribute("shape-rendering", "optimizeSpeed");
                        self.xml.end_element();
                    }

                    // We skip the default stop generation code.
                    self.xml.end_element();
                    continue;
                }
            }

            for window in gradient.stops_ref().windows(2) {
                let (start_c, start_t) = window[0];
                let (end_c, end_t) = window[1];

                self.xml.start_element("stop");
                self.xml.write_attribute("offset", &start_t.repr());
                self.xml.write_attribute("stop-color", &start_c.to_hex());
                self.xml.end_element();

                // Generate (256 / len) stops between the two stops.
                // This is a workaround for a bug in many readers:
                // They tend to just ignore the color space of the gradient.
                // The goal is to have smooth gradients but not to balloon the file size
                // too much if there are already a lot of stops as in most presets.
                let len = if gradient.anti_alias() {
                    (256 / gradient.stops_ref().len() as u32).max(2)
                } else {
                    2
                };

                for i in 1..(len - 1) {
                    let t0 = i as f64 / (len - 1) as f64;
                    let t = start_t + (end_t - start_t) * t0;
                    let c = gradient.sample(RatioOrAngle::Ratio(t));

                    self.xml.start_element("stop");
                    self.xml.write_attribute("offset", &t.repr());
                    self.xml.write_attribute("stop-color", &c.to_hex());
                    self.xml.end_element();
                }

                self.xml.start_element("stop");
                self.xml.write_attribute("offset", &end_t.repr());
                self.xml.write_attribute("stop-color", &end_c.to_hex());
                self.xml.end_element()
            }

            self.xml.end_element();
        }

        self.xml.end_element()
    }

    /// Write the sub-gradients that are used for conic gradients.
    fn write_subgradients(&mut self) {
        if self.conic_subgradients.is_empty() {
            return;
        }

        self.xml.start_element("defs");
        self.xml.write_attribute("id", "subgradients");
        for (id, gradient) in self.conic_subgradients.iter() {
            let x1 = 2.0 - gradient.t0.cos() as f32 + gradient.center.x.get() as f32;
            let y1 = gradient.t0.sin() as f32 + gradient.center.y.get() as f32;
            let x2 = 2.0 - gradient.t1.cos() as f32 + gradient.center.x.get() as f32;
            let y2 = gradient.t1.sin() as f32 + gradient.center.y.get() as f32;

            self.xml.start_element("linearGradient");
            self.xml.write_attribute("id", &id);
            self.xml.write_attribute("gradientUnits", "objectBoundingBox");
            self.xml.write_attribute("x1", &x1);
            self.xml.write_attribute("y1", &y1);
            self.xml.write_attribute("x2", &x2);
            self.xml.write_attribute("y2", &y2);

            self.xml.start_element("stop");
            self.xml.write_attribute("offset", "0%");
            self.xml.write_attribute("stop-color", &gradient.c0.to_hex());
            self.xml.end_element();

            self.xml.start_element("stop");
            self.xml.write_attribute("offset", "100%");
            self.xml.write_attribute("stop-color", &gradient.c1.to_hex());
            self.xml.end_element();

            self.xml.end_element();
        }
        self.xml.end_element();
    }

    fn write_gradient_refs(&mut self) {
        if self.gradient_refs.is_empty() {
            return;
        }

        self.xml.start_element("defs");
        self.xml.write_attribute("id", "gradient-refs");
        for (id, gradient_ref) in self.gradient_refs.iter() {
            match gradient_ref.kind {
                GradientKind::Linear => {
                    self.xml.start_element("linearGradient");
                    self.xml.write_attribute(
                        "gradientTransform",
                        &SvgMatrix(gradient_ref.transform),
                    );
                }
                GradientKind::Radial => {
                    self.xml.start_element("radialGradient");
                    self.xml.write_attribute(
                        "gradientTransform",
                        &SvgMatrix(gradient_ref.transform),
                    );
                }
                GradientKind::Conic => {
                    self.xml.start_element("pattern");
                    self.xml.write_attribute(
                        "patternTransform",
                        &SvgMatrix(gradient_ref.transform),
                    );
                }
            }

            self.xml.write_attribute("id", &id);

            // Writing the href attribute to the "reference" gradient.
            self.xml
                .write_attribute_fmt("href", format_args!("#{}", gradient_ref.id));

            // Also writing the xlink:href attribute for compatibility.
            self.xml
                .write_attribute_fmt("xlink:href", format_args!("#{}", gradient_ref.id));
            self.xml.end_element();
        }

        self.xml.end_element();
    }

    /// Write the raw gradients (without transform) to the SVG file.
    fn write_patterns(&mut self) {
        if self.patterns.is_empty() {
            return;
        }

        self.xml.start_element("defs");
        self.xml.write_attribute("id", "patterns");

        for (id, pattern) in
            self.patterns.iter().map(|(i, p)| (i, p.clone())).collect::<Vec<_>>()
        {
            let size = pattern.size() + pattern.spacing();
            self.xml.start_element("pattern");
            self.xml.write_attribute("id", &id);
            self.xml.write_attribute("width", &size.x.to_pt());
            self.xml.write_attribute("height", &size.y.to_pt());
            self.xml.write_attribute("patternUnits", "userSpaceOnUse");
            self.xml.write_attribute_fmt(
                "viewBox",
                format_args!("0 0 {:.3} {:.3}", size.x.to_pt(), size.y.to_pt()),
            );

            // Render the frame.
            let state = State::new(size, Transform::identity());
            let ts = Transform::identity();
            self.render_frame(state, ts, pattern.frame());

            self.xml.end_element();
        }

        self.xml.end_element()
    }

    /// Writes the references to the deduplicated patterns for each usage site.
    fn write_pattern_refs(&mut self) {
        if self.pattern_refs.is_empty() {
            return;
        }

        self.xml.start_element("defs");
        self.xml.write_attribute("id", "pattern-refs");
        for (id, pattern_ref) in self.pattern_refs.iter() {
            self.xml.start_element("pattern");
            self.xml
                .write_attribute("patternTransform", &SvgMatrix(pattern_ref.transform));

            self.xml.write_attribute("id", &id);

            // Writing the href attribute to the "reference" pattern.
            self.xml
                .write_attribute_fmt("href", format_args!("#{}", pattern_ref.id));

            // Also writing the xlink:href attribute for compatibility.
            self.xml
                .write_attribute_fmt("xlink:href", format_args!("#{}", pattern_ref.id));
            self.xml.end_element();
        }

        self.xml.end_element();
    }
}

/// Convert an outline glyph to an SVG path.
#[comemo::memoize]
fn convert_outline_glyph_to_path(
    font: &Font,
    id: GlyphId,
    scale: Ratio,
) -> Option<EcoString> {
    let mut builder = SvgPathBuilder::with_scale(scale);
    font.ttf().outline_glyph(id, &mut builder)?;
    Some(builder.0)
}

/// Convert a bitmap glyph to an encoded image URL.
#[comemo::memoize]
fn convert_bitmap_glyph_to_image(font: &Font, id: GlyphId) -> Option<(Image, f64, f64)> {
    let raster = font.ttf().glyph_raster_image(id, std::u16::MAX)?;
    if raster.format != ttf_parser::RasterImageFormat::PNG {
        return None;
    }
    let image = Image::new(raster.data.into(), RasterFormat::Png.into(), None).ok()?;
    Some((image, raster.x as f64, raster.y as f64))
}

/// Convert an SVG glyph to an encoded image URL.
#[comemo::memoize]
fn convert_svg_glyph_to_base64_url(font: &Font, id: GlyphId) -> Option<EcoString> {
    let mut data = font.ttf().glyph_svg_image(id)?.data;

    // Decompress SVGZ.
    let mut decoded = vec![];
    if data.starts_with(&[0x1f, 0x8b]) {
        let mut decoder = flate2::read::GzDecoder::new(data);
        decoder.read_to_end(&mut decoded).ok()?;
        data = &decoded;
    }

    let upem = Abs::raw(font.units_per_em());
    let (width, height) = (upem.to_pt(), upem.to_pt());
    let origin_ascender = font.metrics().ascender.at(upem).to_pt();

    // Parse XML.
    let mut svg_str = std::str::from_utf8(data).ok()?.to_owned();
    let mut start_span = None;
    let mut last_viewbox = None;

    // Parse xml and find the viewBox of the svg element.
    // <svg viewBox="0 0 1000 1000">...</svg>
    // ~~~~~^~~~~~~
    for n in xmlparser::Tokenizer::from(svg_str.as_str()) {
        let tok = n.unwrap();
        match tok {
            xmlparser::Token::ElementStart { span, local, .. } => {
                if local.as_str() == "svg" {
                    start_span = Some(span);
                    break;
                }
            }
            xmlparser::Token::Attribute { span, local, value, .. } => {
                if local.as_str() == "viewBox" {
                    last_viewbox = Some((span, value));
                }
            }
            xmlparser::Token::ElementEnd { .. } => break,
            _ => {}
        }
    }

    if last_viewbox.is_none() {
        // Correct the viewbox if it is not present. `-origin_ascender` is to
        // make sure the glyph is rendered at the correct position
        svg_str.insert_str(
            start_span.unwrap().range().end,
            format!(r#" viewBox="0 {} {width} {height}""#, -origin_ascender).as_str(),
        );
    }

    let mut url: EcoString = "data:image/svg+xml;base64,".into();
    let b64_encoded =
        base64::engine::general_purpose::STANDARD.encode(svg_str.as_bytes());
    url.push_str(&b64_encoded);

    Some(url)
}

/// Convert a geometry to an SVG path.
#[comemo::memoize]
fn convert_geometry_to_path(geometry: &Geometry) -> EcoString {
    let mut builder = SvgPathBuilder::default();
    match geometry {
        Geometry::Line(t) => {
            builder.move_to(0.0, 0.0);
            builder.line_to(t.x.to_pt() as f32, t.y.to_pt() as f32);
        }
        Geometry::Rect(rect) => {
            let x = rect.x.to_pt() as f32;
            let y = rect.y.to_pt() as f32;
            builder.rect(x, y);
        }
        Geometry::Path(p) => return convert_path(p),
    };
    builder.0
}

fn convert_path(path: &Path) -> EcoString {
    let mut builder = SvgPathBuilder::default();
    for item in &path.0 {
        match item {
            PathItem::MoveTo(m) => {
                builder.move_to(m.x.to_pt() as f32, m.y.to_pt() as f32)
            }
            PathItem::LineTo(l) => {
                builder.line_to(l.x.to_pt() as f32, l.y.to_pt() as f32)
            }
            PathItem::CubicTo(c1, c2, t) => builder.curve_to(
                c1.x.to_pt() as f32,
                c1.y.to_pt() as f32,
                c2.x.to_pt() as f32,
                c2.y.to_pt() as f32,
                t.x.to_pt() as f32,
                t.y.to_pt() as f32,
            ),
            PathItem::ClosePath => builder.close(),
        }
    }
    builder.0
}

/// Encode an image into a data URL. The format of the URL is
/// `data:image/{format};base64,`.
#[comemo::memoize]
fn convert_image_to_base64_url(image: &Image) -> EcoString {
    let format = match image.format() {
        ImageFormat::Raster(f) => match f {
            RasterFormat::Png => "png",
            RasterFormat::Jpg => "jpeg",
            RasterFormat::Gif => "gif",
        },
        ImageFormat::Vector(f) => match f {
            VectorFormat::Svg => "svg+xml",
        },
    };

    let mut url = eco_format!("data:image/{format};base64,");
    let data = base64::engine::general_purpose::STANDARD.encode(image.data());
    url.push_str(&data);
    url
}

/// Deduplicates its elements. It is used to deduplicate glyphs and clip paths.
/// The `H` is the hash type, and `T` is the value type. The `PREFIX` is the
/// prefix of the index. This is used to distinguish between glyphs and clip
/// paths.
#[derive(Debug, Clone)]
struct Deduplicator<T> {
    kind: char,
    vec: Vec<(u128, T)>,
    present: HashMap<u128, Id>,
}

impl<T> Deduplicator<T> {
    fn new(kind: char) -> Self {
        Self { kind, vec: Vec::new(), present: HashMap::new() }
    }

    /// Inserts a value into the vector. If the hash is already present, returns
    /// the index of the existing value and `f` will not be called. Otherwise,
    /// inserts the value and returns the id of the inserted value.
    #[must_use = "returns the index of the inserted value"]
    fn insert_with<F>(&mut self, hash: u128, f: F) -> Id
    where
        F: FnOnce() -> T,
    {
        *self.present.entry(hash).or_insert_with(|| {
            let index = self.vec.len();
            self.vec.push((hash, f()));
            Id(self.kind, hash, index)
        })
    }

    /// Iterate over the elements alongside their ids.
    fn iter(&self) -> impl Iterator<Item = (Id, &T)> {
        self.vec
            .iter()
            .enumerate()
            .map(|(i, (id, v))| (Id(self.kind, *id, i), v))
    }

    /// Returns true if the deduplicator is empty.
    fn is_empty(&self) -> bool {
        self.vec.is_empty()
    }
}

/// Identifies a `<def>`.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
struct Id(char, u128, usize);

impl Display for Id {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}{:0X}", self.0, self.1)
    }
}

/// Displays as an SVG matrix.
struct SvgMatrix(Transform);

impl Display for SvgMatrix {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        // Convert a [`Transform`] into a SVG transform string.
        // See https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/transform
        write!(
            f,
            "matrix({} {} {} {} {} {})",
            self.0.sx.get(),
            self.0.ky.get(),
            self.0.kx.get(),
            self.0.sy.get(),
            self.0.tx.to_pt(),
            self.0.ty.to_pt()
        )
    }
}

/// A builder for SVG path.
struct SvgPathBuilder(pub EcoString, pub Ratio);

impl SvgPathBuilder {
    fn with_scale(scale: Ratio) -> Self {
        Self(EcoString::new(), scale)
    }

    fn scale(&self) -> f32 {
        self.1.get() as f32
    }

    /// Create a rectangle path. The rectangle is created with the top-left
    /// corner at (0, 0). The width and height are the size of the rectangle.
    fn rect(&mut self, width: f32, height: f32) {
        self.move_to(0.0, 0.0);
        self.line_to(0.0, height);
        self.line_to(width, height);
        self.line_to(width, 0.0);
        self.close();
    }

    /// Creates an arc path.
    fn arc(
        &mut self,
        radius: (f32, f32),
        x_axis_rot: f32,
        large_arc_flag: u32,
        sweep_flag: u32,
        pos: (f32, f32),
    ) {
        let scale = self.scale();
        write!(
            &mut self.0,
            "A {rx} {ry} {x_axis_rot} {large_arc_flag} {sweep_flag} {x} {y} ",
            rx = radius.0 * scale,
            ry = radius.1 * scale,
            x = pos.0 * scale,
            y = pos.1 * scale,
        )
        .unwrap();
    }
}

impl Default for SvgPathBuilder {
    fn default() -> Self {
        Self(Default::default(), Ratio::one())
    }
}

/// A builder for SVG path. This is used to build the path for a glyph.
impl ttf_parser::OutlineBuilder for SvgPathBuilder {
    fn move_to(&mut self, x: f32, y: f32) {
        let scale = self.scale();
        write!(&mut self.0, "M {} {} ", x * scale, y * scale).unwrap();
    }

    fn line_to(&mut self, x: f32, y: f32) {
        let scale = self.scale();
        write!(&mut self.0, "L {} {} ", x * scale, y * scale).unwrap();
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        let scale = self.scale();
        write!(
            &mut self.0,
            "Q {} {} {} {} ",
            x1 * scale,
            y1 * scale,
            x * scale,
            y * scale
        )
        .unwrap();
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        let scale = self.scale();
        write!(
            &mut self.0,
            "C {} {} {} {} {} {} ",
            x1 * scale,
            y1 * scale,
            x2 * scale,
            y2 * scale,
            x * scale,
            y * scale
        )
        .unwrap();
    }

    fn close(&mut self) {
        write!(&mut self.0, "Z ").unwrap();
    }
}

/// Encode the color as an SVG color.
trait ColorEncode {
    /// Encode the color.
    fn encode(&self) -> EcoString;
}

impl ColorEncode for Color {
    fn encode(&self) -> EcoString {
        match *self {
            c @ Color::Rgb(_)
            | c @ Color::Luma(_)
            | c @ Color::Cmyk(_)
            | c @ Color::Hsv(_) => c.to_hex(),
            Color::LinearRgb(rgb) => {
                if rgb.alpha != 1.0 {
                    eco_format!(
                        "color(srgb-linear {:.5} {:.5} {:.5} / {:.5})",
                        rgb.red,
                        rgb.green,
                        rgb.blue,
                        rgb.alpha
                    )
                } else {
                    eco_format!(
                        "color(srgb-linear {:.5} {:.5} {:.5})",
                        rgb.red,
                        rgb.green,
                        rgb.blue,
                    )
                }
            }
            Color::Oklab(oklab) => {
                if oklab.alpha != 1.0 {
                    eco_format!(
                        "oklab({:.3}% {:.5} {:.5} / {:.5})",
                        oklab.l * 100.0,
                        oklab.a,
                        oklab.b,
                        oklab.alpha
                    )
                } else {
                    eco_format!(
                        "oklab({:.3}% {:.5} {:.5})",
                        oklab.l * 100.0,
                        oklab.a,
                        oklab.b,
                    )
                }
            }
            Color::Oklch(oklch) => {
                if oklch.alpha != 1.0 {
                    eco_format!(
                        "oklch({:.3}% {:.5} {:.3}deg / {:.3})",
                        oklch.l * 100.0,
                        oklch.chroma,
                        oklch.hue.into_degrees(),
                        oklch.alpha
                    )
                } else {
                    eco_format!(
                        "oklch({:.3}% {:.5} {:.3}deg)",
                        oklch.l * 100.0,
                        oklch.chroma,
                        oklch.hue.into_degrees(),
                    )
                }
            }
            Color::Hsl(hsl) => {
                if hsl.alpha != 1.0 {
                    eco_format!(
                        "hsla({:.3}deg {:.3}% {:.3}% / {:.5})",
                        hsl.hue.into_degrees(),
                        hsl.saturation * 100.0,
                        hsl.lightness * 100.0,
                        hsl.alpha,
                    )
                } else {
                    eco_format!(
                        "hsl({:.3}deg {:.3}% {:.3}%)",
                        hsl.hue.into_degrees(),
                        hsl.saturation * 100.0,
                        hsl.lightness * 100.0,
                    )
                }
            }
        }
    }
}

/// Maps a coordinate in a unit size square to a coordinate in the pattern.
fn correct_pattern_pos(x: f32) -> f32 {
    (x + 0.5) / 2.0
}
