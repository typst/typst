//! Rendering of Typst documents into SVG images.

mod image;
mod paint;
mod shape;
mod text;

pub use image::{convert_image_scaling, convert_image_to_base64_url};
use rustc_hash::FxHashMap;
use typst_library::introspection::Introspector;
use typst_library::model::Destination;

use std::fmt::{self, Display, Formatter, Write};

use ecow::EcoString;
use typst_library::layout::{
    Abs, Frame, FrameItem, FrameKind, GroupItem, Page, PagedDocument, Point, Ratio,
    Sides, Size, Transform,
};
use typst_library::visualize::{Geometry, Gradient, Tiling};
use typst_utils::hash128;
use xmlwriter::XmlWriter;

use crate::paint::{GradientRef, SVGSubGradient, TilingRef};
use crate::text::RenderedGlyph;

#[derive(Clone, Copy)]
pub struct Options {
    pub render_bleed: bool,
}

/// Export a frame into a SVG file.
#[typst_macros::time(name = "svg")]
pub fn svg(page: &Page, opts: Options) -> String {
    let bleed = if opts.render_bleed { page.bleed } else { Sides::default() };
    let size = page.frame.size() + bleed.sum_by_axis();

    let mut renderer = SVGRenderer::new();
    renderer.write_header(size);

    let state = State::new(size, Transform::identity());
    let ts = Transform::translate(bleed.left, bleed.top);
    renderer.render_page(&state, ts, page);
    renderer.finalize()
}

/// Export a frame into a SVG file.
#[typst_macros::time(name = "svg frame")]
pub fn svg_frame(frame: &Frame) -> String {
    let mut renderer = SVGRenderer::new();
    renderer.write_header(frame.size());

    let state = State::new(frame.size(), Transform::identity());
    renderer.render_frame(&state, frame);
    renderer.finalize()
}

/// Export a frame into an SVG suitable for embedding into HTML.
#[typst_macros::time(name = "svg html frame")]
pub fn svg_html_frame(
    frame: &Frame,
    text_size: Abs,
    id: Option<&str>,
    link_points: &[(Point, EcoString)],
    introspector: &Introspector,
) -> String {
    let mut renderer = SVGRenderer::with_options(
        xmlwriter::Options {
            indent: xmlwriter::Indent::None,
            ..Default::default()
        },
        Some(introspector),
    );
    renderer.write_header_with_custom_attrs(frame.size(), |xml| {
        if let Some(id) = id {
            xml.write_attribute("id", id);
        }
        xml.write_attribute("class", "typst-frame");
        xml.write_attribute_fmt(
            "style",
            format_args!(
                "overflow: visible; width: {}em; height: {}em;",
                frame.width() / text_size,
                frame.height() / text_size,
            ),
        );
    });

    let state = State::new(frame.size(), Transform::identity());
    renderer.render_frame(&state, frame);

    for (pos, id) in link_points {
        renderer.render_link_point(*pos, id);
    }

    renderer.finalize()
}

/// Export a document with potentially multiple pages into a single SVG file.
///
/// The padding will be added around and between the individual frames.
pub fn svg_merged(document: &PagedDocument, padding: Abs) -> String {
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
        renderer.render_page(&state, ts, page);
        y += page.frame.height() + padding;
    }

    renderer.finalize()
}

/// Renders one or multiple frames to an SVG file.
struct SVGRenderer<'a> {
    /// The internal XML writer.
    xml: XmlWriter,
    /// The document's introspector, if we're writing an HTML frame.
    introspector: Option<&'a Introspector>,
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
    /// Deduplicated tilings with transform matrices. They use a reference
    /// (`href`) to a "source" tiling instead of being defined inline.
    /// This saves a lot of space since tilings are often reused but with
    /// different transforms. Therefore this allows us to reuse the same gradient
    /// multiple times.
    tiling_refs: Deduplicator<TilingRef>,
    /// These are the actual gradients being written in the SVG file.
    /// These gradients are deduplicated because they do not contain the transform
    /// matrix, allowing them to be reused across multiple invocations.
    ///
    /// The `Ratio` is the aspect ratio of the gradient, this is used to correct
    /// the angle of the gradient.
    gradients: Deduplicator<(Gradient, Ratio)>,
    /// These are the actual tilings being written in the SVG file.
    /// These tilings are deduplicated because they do not contain the transform
    /// matrix, allowing them to be reused across multiple invocations.
    ///
    /// The `String` is the rendered tiling frame.
    tilings: Deduplicator<Tiling>,
    /// These are the gradients that compose a conic gradient.
    conic_subgradients: Deduplicator<SVGSubGradient>,
}

/// Contextual information for rendering.
#[derive(Copy, Clone)]
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

impl<'a> SVGRenderer<'a> {
    /// Create a new SVG renderer with empty glyph and clip path.
    fn new() -> Self {
        Self::with_options(Default::default(), None)
    }

    /// Create a new SVG renderer with the given configuration.
    fn with_options(
        options: xmlwriter::Options,
        introspector: Option<&'a Introspector>,
    ) -> Self {
        SVGRenderer {
            xml: XmlWriter::new(options),
            introspector,
            glyphs: Deduplicator::new('g'),
            clip_paths: Deduplicator::new('c'),
            gradient_refs: Deduplicator::new('g'),
            gradients: Deduplicator::new('f'),
            conic_subgradients: Deduplicator::new('s'),
            tiling_refs: Deduplicator::new('p'),
            tilings: Deduplicator::new('t'),
        }
    }

    /// Write the default SVG header, including a `typst-doc` class, the
    /// `viewBox` and `width` and `height` attributes.
    fn write_header(&mut self, size: Size) {
        self.write_header_with_custom_attrs(size, |xml| {
            xml.write_attribute("class", "typst-doc");
        });
    }

    /// Write the SVG header with additional attributes and standard attributes.
    fn write_header_with_custom_attrs(
        &mut self,
        size: Size,
        write_custom_attrs: impl FnOnce(&mut XmlWriter),
    ) {
        self.xml.start_element("svg");
        write_custom_attrs(&mut self.xml);
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

    /// Render a page with the given transform.
    fn render_page(&mut self, state: &State, ts: Transform, page: &Page) {
        if let Some(fill) = page.fill_or_white() {
            let shape =
                Geometry::Rect(page.frame.size() + page.bleed.sum_by_axis()).filled(fill);
            self.render_shape(state, &shape);
        }

        if !ts.is_identity() {
            self.xml.start_element("g");
            self.xml.write_attribute("transform", &SvgMatrix(ts));
        }

        self.render_frame(state, &page.frame);

        if !ts.is_identity() {
            self.xml.end_element();
        }
    }

    /// Render a frame with the given transform.
    fn render_frame(&mut self, state: &State, frame: &Frame) {
        self.xml.start_element("g");

        for (pos, item) in frame.items() {
            let state = state.pre_translate(*pos);
            match item {
                FrameItem::Group(group) => self.render_group(&state, group),
                FrameItem::Text(text) => self.render_text(&state, text),
                FrameItem::Shape(shape, _) => self.render_shape(&state, shape),
                FrameItem::Image(image, size, _) => {
                    self.render_image(&state, image, size)
                }
                FrameItem::Link(dest, size) => self.render_link(&state, dest, *size),
                FrameItem::Tag(_) => {}
            };
        }

        self.xml.end_element();
    }

    /// Render a group. If the group has `clips` set to true, a clip path will
    /// be created.
    fn render_group(&mut self, state: &State, group: &GroupItem) {
        self.xml.start_element("g");
        self.xml.write_attribute("class", "typst-group");

        let state = match group.frame.kind() {
            FrameKind::Soft => state.pre_concat(group.transform),
            FrameKind::Hard => {
                let transform = state.transform.pre_concat(group.transform);
                if !transform.is_identity() {
                    self.xml.write_attribute("transform", &SvgMatrix(transform));
                }
                state
                    .with_transform(Transform::identity())
                    .with_size(group.frame.size())
            }
        };

        if let Some(label) = group.label {
            self.xml.write_attribute("data-typst-label", &label.resolve());
        }

        if let Some(clip_curve) = &group.clip {
            let offset = Point::new(state.transform.tx, state.transform.ty);
            let hash = hash128(&(&clip_curve, &offset));
            let id = self
                .clip_paths
                .insert_with(hash, || shape::convert_curve(offset, clip_curve));
            self.xml.write_attribute_fmt("clip-path", format_args!("url(#{id})"));
        }

        self.render_frame(&state, &group.frame);
        self.xml.end_element();
    }

    /// Render a link element.
    fn render_link(&mut self, state: &State, dest: &Destination, size: Size) {
        self.xml.start_element("a");
        if !state.transform.is_identity() {
            self.xml.write_attribute("transform", &SvgMatrix(state.transform));
        }

        match dest {
            Destination::Location(loc) => {
                // TODO: Location links on the same page could also be supported
                // outside of HTML.
                if let Some(introspector) = self.introspector
                    && let Some(id) = introspector.html_id(*loc)
                {
                    self.xml.write_attribute_fmt("href", format_args!("#{id}"));
                    self.xml.write_attribute_fmt("xlink:href", format_args!("#{id}"));
                }
            }
            Destination::Position(_) => {
                // TODO: Links on the same page could be supported.
            }
            Destination::Url(url) => {
                self.xml.write_attribute("href", url.as_str());
                self.xml.write_attribute("xlink:href", url.as_str());
            }
        }

        self.xml.start_element("rect");
        self.xml
            .write_attribute_fmt("width", format_args!("{}", size.x.to_pt()));
        self.xml
            .write_attribute_fmt("height", format_args!("{}", size.y.to_pt()));
        self.xml.write_attribute("fill", "transparent");
        self.xml.write_attribute("stroke", "none");
        self.xml.end_element();

        self.xml.end_element();
    }

    /// Renders a linkable point that can be used to link into an HTML frame.
    fn render_link_point(&mut self, pos: Point, id: &str) {
        self.xml.start_element("g");
        self.xml.write_attribute("id", id);
        self.xml.write_attribute_fmt(
            "transform",
            format_args!("translate({} {})", pos.x.to_pt(), pos.y.to_pt()),
        );
        self.xml.end_element();
    }

    /// Finalize the SVG file. This must be called after all rendering is done.
    fn finalize(mut self) -> String {
        self.write_glyph_defs();
        self.write_clip_path_defs();
        self.write_gradients();
        self.write_gradient_refs();
        self.write_subgradients();
        self.write_tilings();
        self.write_tiling_refs();
        self.xml.end_document()
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
}

/// Deduplicates its elements. It is used to deduplicate glyphs and clip paths.
/// The `H` is the hash type, and `T` is the value type. The `PREFIX` is the
/// prefix of the index. This is used to distinguish between glyphs and clip
/// paths.
#[derive(Debug, Clone)]
struct Deduplicator<T> {
    kind: char,
    vec: Vec<(u128, T)>,
    present: FxHashMap<u128, Id>,
}

impl<T> Deduplicator<T> {
    fn new(kind: char) -> Self {
        Self {
            kind,
            vec: Vec::new(),
            present: FxHashMap::default(),
        }
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

/// A builder for SVG path using relative coordinates.
struct SvgPathBuilder {
    pub path: EcoString,
    pub scale: Ratio,
    pub last_close_point: Point,
    pub last_point: Point,
}

impl SvgPathBuilder {
    fn with_translate(pos: Point) -> Self {
        // add initial M node to transform the entire path
        Self {
            path: EcoString::from(format!("M {} {}", pos.x.to_pt(), pos.y.to_pt())),
            scale: Ratio::one(),
            last_close_point: pos,
            last_point: Point::zero(),
        }
    }

    fn with_scale(scale: Ratio) -> Self {
        Self {
            path: EcoString::from("M 0 0"),
            scale,
            last_close_point: Point::zero(),
            last_point: Point::zero(),
        }
    }

    fn scale(&self) -> f32 {
        self.scale.get() as f32
    }

    fn set_point(&mut self, x: f32, y: f32) {
        let point = Point::new(
            Abs::pt(f64::from(x * self.scale())),
            Abs::pt(f64::from(y * self.scale())),
        );

        self.last_point = point;
    }

    fn map_x(&self, x: f32) -> f32 {
        x * self.scale() - self.last_point.x.to_pt() as f32
    }

    fn map_y(&self, y: f32) -> f32 {
        y * self.scale() - self.last_point.y.to_pt() as f32
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
        let rx = self.map_x(radius.0);
        let ry = self.map_y(radius.1);
        let x = self.map_x(pos.0);
        let y = self.map_y(pos.1);
        write!(
            &mut self.path,
            "a {rx} {ry} {x_axis_rot} {large_arc_flag} {sweep_flag} {x} {y} "
        )
        .unwrap();

        self.set_point(x, y);
    }

    fn move_to(&mut self, x: f32, y: f32) {
        let _x = self.map_x(x);
        let _y = self.map_y(y);
        if _x != 0.0 || _y != 0.0 {
            write!(&mut self.path, "m {_x} {_y} ").unwrap();
        }

        self.set_point(x, y);
        self.last_close_point = self.last_point;
    }

    fn line_to(&mut self, x: f32, y: f32) {
        let _x = self.map_x(x);
        let _y = self.map_y(y);

        if _x != 0.0 && _y != 0.0 {
            write!(&mut self.path, "l {_x} {_y} ").unwrap();
        } else if _x != 0.0 {
            write!(&mut self.path, "h {_x} ").unwrap();
        } else if _y != 0.0 {
            write!(&mut self.path, "v {_y} ").unwrap();
        }

        self.set_point(x, y);
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        let curve = format!(
            "c {} {} {} {} {} {} ",
            self.map_x(x1),
            self.map_y(y1),
            self.map_x(x2),
            self.map_y(y2),
            self.map_x(x),
            self.map_y(y)
        );
        write!(&mut self.path, "{curve}").unwrap();
        self.set_point(x, y);
    }

    fn close(&mut self) {
        write!(&mut self.path, "Z ").unwrap();
        self.last_point = self.last_close_point;
    }
}

/// A builder for SVG path. This is used to build the path for a glyph.
impl ttf_parser::OutlineBuilder for SvgPathBuilder {
    fn move_to(&mut self, x: f32, y: f32) {
        self.move_to(x, y);
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.line_to(x, y);
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        let _x1 = self.map_x(x1);
        let _y1 = self.map_y(y1);
        let _x = self.map_x(x);
        let _y = self.map_y(y);

        write!(&mut self.path, "q {_x1} {_y1} {_x} {_y} ").unwrap();

        self.set_point(x, y);
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        self.curve_to(x1, y1, x2, y2, x, y);
    }

    fn close(&mut self) {
        self.close();
    }
}

impl Default for SvgPathBuilder {
    fn default() -> Self {
        Self {
            path: Default::default(),
            scale: Ratio::one(),
            last_close_point: Point::zero(),
            last_point: Point::zero(),
        }
    }
}
