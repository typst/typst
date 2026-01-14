//! Rendering of Typst documents into SVG images.

mod image;
mod paint;
mod path;
mod shape;
mod text;
mod write;

pub use image::{convert_image_scaling, convert_image_to_base64_url};
use indexmap::IndexMap;
use rustc_hash::FxBuildHasher;
use typst_library::introspection::Introspector;
use typst_library::model::Destination;

use std::hash::Hash;

use ecow::EcoString;
use typst_library::layout::{
    Abs, Frame, FrameItem, FrameKind, GroupItem, Page, PagedDocument, Point, Ratio, Size,
    Transform,
};
use typst_library::visualize::{Geometry, Gradient, Tiling};
use xmlwriter::XmlWriter;

use crate::paint::{GradientRef, SVGSubGradient, TilingRef};
use crate::text::RenderedGlyph;
use crate::write::{SvgDisplay, SvgElem, SvgIdRef, SvgTransform, SvgUrl, SvgWrite};

const XML_WRITE_OPTIONS: xmlwriter::Options = xmlwriter::Options {
    use_single_quote: false,
    indent: xmlwriter::Indent::Spaces(2),
    attributes_indent: xmlwriter::Indent::None,
};

/// Export a frame into a SVG file.
#[typst_macros::time(name = "svg")]
pub fn svg(page: &Page) -> String {
    let mut renderer = SVGRenderer::new();
    let mut xml = XmlWriter::new(XML_WRITE_OPTIONS);
    let mut svg = svg_header(&mut xml, page.frame.size());

    let state = State::new(page.frame.size());
    renderer.render_page(&mut svg, &state, Transform::identity(), page);
    renderer.finalize(svg);
    xml.end_document()
}

/// Export a frame into a SVG file.
#[typst_macros::time(name = "svg frame")]
pub fn svg_frame(frame: &Frame) -> String {
    let mut renderer = SVGRenderer::new();
    let mut xml = XmlWriter::new(XML_WRITE_OPTIONS);
    let mut svg = svg_header(&mut xml, frame.size());

    let state = State::new(frame.size());
    renderer.render_frame(&mut svg, &state, frame);
    renderer.finalize(svg);
    xml.end_document()
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
    let mut renderer = SVGRenderer::with_options(Some(introspector));
    let mut xml = XmlWriter::new(xmlwriter::Options {
        indent: xmlwriter::Indent::None,
        ..XML_WRITE_OPTIONS
    });
    let mut svg = svg_header_with_custom_attrs(&mut xml, frame.size(), |svg| {
        if let Some(id) = id {
            svg.attr("id", id);
        }
        svg.attr_with("style", |attr| {
            // TODO: Maybe make this a little more elegant?
            attr.push_str("overflow: visible; width: ");
            attr.push_num(frame.width() / text_size);
            attr.push_str("em; height: ");
            attr.push_num(frame.height() / text_size);
            attr.push_str("em;");
        });
    });

    let state = State::new(frame.size());
    renderer.render_frame(&mut svg, &state, frame);

    for (pos, id) in link_points {
        renderer.render_link_point(&mut svg, *pos, id);
    }

    renderer.finalize(svg);
    xml.end_document()
}

/// Export a document with potentially multiple pages into a single SVG file.
///
/// The gap will be added between the individual pages.
pub fn svg_merged(document: &PagedDocument, gap: Abs) -> String {
    let width = document
        .pages
        .iter()
        .map(|page| page.frame.width())
        .max()
        .unwrap_or_default();
    let height = document.pages.len().saturating_sub(1) as f64 * gap
        + document.pages.iter().map(|page| page.frame.height()).sum::<Abs>();

    let mut renderer = SVGRenderer::new();
    let mut xml = XmlWriter::new(XML_WRITE_OPTIONS);
    let mut svg = svg_header(&mut xml, Size::new(width, height));

    let mut y = Abs::zero();
    for page in &document.pages {
        let state = State::new(page.frame.size());
        renderer.render_page(
            &mut svg,
            &state,
            Transform::translate(Abs::zero(), y),
            page,
        );
        y += page.frame.height() + gap;
    }

    renderer.finalize(svg);
    xml.end_document()
}

/// Renders one or multiple frames to an SVG file.
struct SVGRenderer<'a> {
    /// The document's introspector, if we're writing an HTML frame.
    introspector: Option<&'a Introspector>,
    /// Prepared glyphs.
    glyphs: Deduplicator<Option<RenderedGlyph>>,
    /// Clip paths are used to clip a group. A clip path is a path that defines
    /// the clipping region. The clip path is referenced by the `clip-path`
    /// attribute of the group. The clip path is in the format of `M x y L x y C
    /// x1 y1 x2 y2 x y Z`.
    clip_paths: Deduplicator<EcoString>,
    /// These are the actual gradients being written in the SVG file.
    /// These gradients are deduplicated because they do not contain the transform
    /// matrix, allowing them to be reused across multiple invocations.
    ///
    /// The `Ratio` is the aspect ratio of the gradient, this is used to correct
    /// the angle of the gradient.
    gradients: Deduplicator<(Gradient, Ratio)>,
    /// Deduplicated gradients with transform matrices. They use a reference
    /// (`href`) to a "source" gradient instead of being defined inline.
    /// This saves a lot of space since gradients are often reused but with
    /// different transforms. Therefore this allows us to reuse the same gradient
    /// multiple times.
    gradient_refs: Deduplicator<GradientRef>,
    /// These are the gradients that compose a conic gradient.
    conic_subgradients: Deduplicator<SVGSubGradient>,
    /// These are the actual tilings being written in the SVG file.
    /// These tilings are deduplicated because they do not contain the transform
    /// matrix, allowing them to be reused across multiple invocations.
    ///
    /// The `String` is the rendered tiling frame.
    tilings: Deduplicator<Tiling>,
    /// Deduplicated tilings with transform matrices. They use a reference
    /// (`href`) to a "source" tiling instead of being defined inline.
    /// This saves a lot of space since tilings are often reused but with
    /// different transforms. Therefore this allows us to reuse the same gradient
    /// multiple times.
    tiling_refs: Deduplicator<TilingRef>,
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
    fn new(size: Size) -> Self {
        Self { size, transform: Transform::identity() }
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
        Self::with_options(None)
    }

    /// Create a new SVG renderer with the given configuration.
    fn with_options(introspector: Option<&'a Introspector>) -> Self {
        SVGRenderer {
            introspector,
            glyphs: Deduplicator::new('g'),
            clip_paths: Deduplicator::new('c'),
            gradients: Deduplicator::new('f'),
            gradient_refs: Deduplicator::new('r'),
            conic_subgradients: Deduplicator::new('s'),
            tilings: Deduplicator::new('t'),
            tiling_refs: Deduplicator::new('p'),
        }
    }

    /// Render a page with the given transform.
    fn render_page(
        &mut self,
        svg: &mut SvgElem,
        state: &State,
        ts: Transform,
        page: &Page,
    ) {
        let mut svg = svg.lazy_elem("g");
        if !ts.is_identity() {
            svg.init().attr("transform", SvgTransform(ts));
        }

        if let Some(fill) = page.fill_or_white() {
            let shape = Geometry::Rect(page.frame.size()).filled(fill);
            self.render_shape(svg.lazy(), state, &shape);
        }

        self.render_frame(svg.lazy(), state, &page.frame);
    }

    /// Render a frame with the given transform.
    fn render_frame(&mut self, svg: &mut SvgElem, state: &State, frame: &Frame) {
        for (pos, item) in frame.items() {
            let state = state.pre_translate(*pos);
            match item {
                FrameItem::Group(group) => self.render_group(svg, &state, group),
                FrameItem::Text(text) => self.render_text(svg, &state, text),
                FrameItem::Shape(shape, _) => self.render_shape(svg, &state, shape),
                FrameItem::Image(image, size, _) => {
                    self.render_image(svg, &state, image, size)
                }
                FrameItem::Link(dest, size) => self.render_link(svg, &state, dest, *size),
                FrameItem::Tag(_) => {}
            };
        }
    }

    /// Render a group. If the group has `clips` set to true, a clip path will
    /// be created.
    fn render_group(&mut self, svg: &mut SvgElem, state: &State, group: &GroupItem) {
        let mut svg = svg.lazy_elem("g");

        let state = match group.frame.kind() {
            FrameKind::Soft => state.pre_concat(group.transform),
            FrameKind::Hard => {
                // Always generate a group for hard frames.
                svg.init();

                let transform = state.transform.pre_concat(group.transform);
                if !transform.is_identity() {
                    svg.init().attr("transform", SvgTransform(transform));
                }
                state
                    .with_transform(Transform::identity())
                    .with_size(group.frame.size())
            }
        };

        if let Some(label) = group.label {
            svg.init().attr("data-typst-label", label.resolve());
        }

        if let Some(clip_curve) = &group.clip {
            let offset = Point::new(state.transform.tx, state.transform.ty);
            let id = self.clip_paths.insert_with((clip_curve, offset), || {
                shape::convert_curve(offset, clip_curve)
            });
            svg.init().attr("clip-path", SvgUrl(id));
        }

        self.render_frame(svg.lazy(), &state, &group.frame);
    }

    /// Render a link element.
    fn render_link(
        &mut self,
        svg: &mut SvgElem,
        state: &State,
        dest: &Destination,
        size: Size,
    ) {
        let mut a = svg.elem("a");
        if !state.transform.is_identity() {
            a.attr("transform", SvgTransform(state.transform));
        }

        match dest {
            Destination::Location(loc) => {
                // TODO: Location links on the same page could also be supported
                // outside of HTML.
                if let Some(introspector) = self.introspector
                    && let Some(id) = introspector.html_id(*loc)
                {
                    a.attr("href", SvgIdRef(id));
                    a.attr("xlink:href", SvgIdRef(id));
                }
            }
            Destination::Position(_) => {
                // TODO: Links on the same page could be supported.
            }
            Destination::Url(url) => {
                a.attr("href", url.as_str());
                a.attr("xlink:href", url.as_str());
            }
        }

        a.elem("rect")
            .attr("width", size.x.to_pt())
            .attr("height", size.y.to_pt())
            .attr("fill", "transparent")
            .attr("stroke", "none");
    }

    /// Renders a linkable point that can be used to link into an HTML frame.
    fn render_link_point(&mut self, svg: &mut SvgElem, pos: Point, id: &str) {
        svg.elem("g")
            .attr("id", id)
            .attr("transform", SvgTransform(Transform::translate(pos.x, pos.y)));
    }

    /// Finalize the SVG file. This must be called after all rendering is done.
    fn finalize(mut self, mut svg: SvgElem) {
        self.write_glyph_defs(&mut svg);
        self.write_clip_path_defs(&mut svg);
        self.write_gradients(&mut svg);
        self.write_gradient_refs(&mut svg);
        self.write_subgradients(&mut svg);
        self.write_tilings(&mut svg);
        self.write_tiling_refs(&mut svg);
    }

    /// Build the clip path definitions.
    fn write_clip_path_defs(&self, svg: &mut SvgElem) {
        if self.clip_paths.is_empty() {
            return;
        }

        let mut defs = svg.elem("defs");
        for (id, path) in self.clip_paths.iter() {
            defs.elem("clipPath").attr("id", id).with(|svg| {
                svg.elem("path").attr("d", path);
            });
        }
    }
}

/// Write the default SVG header, including a `typst-doc` class, the
/// `viewBox` and `width` and `height` attributes.
fn svg_header(xml: &mut XmlWriter, size: Size) -> SvgElem<'_> {
    svg_header_with_custom_attrs(xml, size, |_| {})
}

/// Write the SVG header with additional attributes and standard attributes.
fn svg_header_with_custom_attrs(
    xml: &mut XmlWriter,
    size: Size,
    write_custom_attrs: impl FnOnce(&mut SvgElem),
) -> SvgElem<'_> {
    // Clamp the size of SVGs to at least one pt. resvg and probably also
    // other SVG parsers don't handle SVGs with 0 sized dimensions.
    let size = size.max(Size::splat(Abs::pt(1.0)));

    let mut svg = SvgElem::new(xml, "svg");

    write_custom_attrs(&mut svg);

    svg.attr_with("viewBox", |attr| {
        attr.push_nums([0.0, 0.0, size.x.to_pt(), size.y.to_pt()])
    });
    svg.attr_with("width", |attr| {
        attr.push_num(size.x.to_pt());
        attr.push_str("pt");
    });
    svg.attr_with("height", |attr| {
        attr.push_num(size.y.to_pt());
        attr.push_str("pt");
    });
    svg.attr("xmlns", "http://www.w3.org/2000/svg");
    svg.attr("xmlns:xlink", "http://www.w3.org/1999/xlink");
    svg.attr("xmlns:h5", "http://www.w3.org/1999/xhtml");

    svg
}

/// Deduplicates its elements. It is used to deduplicate glyphs and clip paths.
/// The `H` is the hash type, and `T` is the value type. The `PREFIX` is the
/// prefix of the index. This is used to distinguish between glyphs and clip
/// paths.
#[derive(Debug, Default, Clone)]
struct Deduplicator<T> {
    kind: char,
    map: IndexMap<u128, T, FxBuildHasher>,
}

impl<T> Deduplicator<T> {
    fn new(kind: char) -> Self {
        Self { kind, map: IndexMap::default() }
    }

    /// Inserts a value into the vector. If the hash is already present, returns
    /// the index of the existing value and `f` will not be called. Otherwise,
    /// inserts the value and returns the id of the inserted value.
    #[must_use = "returns the id of the inserted value"]
    fn insert_with<K, F>(&mut self, key: K, f: F) -> DedupId
    where
        K: Hash,
        F: FnOnce() -> T,
    {
        self.insert_with_val(key, f).0
    }

    /// Same as [`Self::insert_with`], but it also returns a reference to the
    /// cached or inserted value.
    #[must_use]
    fn insert_with_val<K, F>(&mut self, key: K, f: F) -> (DedupId, &mut T)
    where
        K: Hash,
        F: FnOnce() -> T,
    {
        let hash = typst_utils::hash128(&key);
        let val = self.map.entry(hash).or_insert_with(f);
        (DedupId(self.kind, hash), val)
    }

    /// Iterate over the elements alongside their ids.
    fn iter(&self) -> impl Iterator<Item = (DedupId, &T)> {
        self.map.iter().map(|(hash, v)| (DedupId(self.kind, *hash), v))
    }

    /// Returns true if the deduplicator is empty.
    fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}

/// Identifies a `<def>`.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
struct DedupId(char, u128);

impl SvgDisplay for DedupId {
    fn fmt(&self, f: &mut impl SvgWrite) {
        let Self(kind, hash) = *self;
        f.push_char(kind);

        let mut digits = [0; 32];
        for (i, byte) in hash.to_be_bytes().into_iter().enumerate() {
            digits[2 * i] = to_hex_digit((byte >> 4) & 0x0F);
            digits[2 * i + 1] = to_hex_digit(byte & 0x0F);
        }

        // The digits are all valid ASCII hex characters.
        let str = std::str::from_utf8(&digits).unwrap();
        f.push_str(str.trim_start_matches('0'));

        fn to_hex_digit(nibble: u8) -> u8 {
            match nibble {
                0..10 => b'0' + nibble,
                _ => b'A' + (nibble - 10),
            }
        }
    }
}
