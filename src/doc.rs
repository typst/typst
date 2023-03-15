//! Finished documents.

use std::fmt::{self, Debug, Formatter, Write};
use std::num::NonZeroUsize;
use std::str::FromStr;
use std::sync::Arc;

use ecow::EcoString;

use crate::eval::{cast_from_value, cast_to_value, dict, Dict, Value};
use crate::font::Font;
use crate::geom::{
    self, rounded_rect, Abs, Align, Axes, Color, Corners, Dir, Em, Geometry, Length,
    Numeric, Paint, Point, Rel, RgbaColor, Shape, Sides, Size, Stroke, Transform,
};
use crate::image::Image;
use crate::model::{node, Content, Fold, Introspector, StableId, StyleChain};
use crate::syntax::Span;

/// A finished document with metadata and page frames.
#[derive(Debug, Default, Clone, Hash)]
pub struct Document {
    /// The page frames.
    pub pages: Vec<Frame>,
    /// The document's title.
    pub title: Option<EcoString>,
    /// The document's author.
    pub author: Vec<EcoString>,
}

/// A finished layout with elements at fixed positions.
#[derive(Default, Clone, Hash)]
pub struct Frame {
    /// The size of the frame.
    size: Size,
    /// The baseline of the frame measured from the top. If this is `None`, the
    /// frame's implicit baseline is at the bottom.
    baseline: Option<Abs>,
    /// The elements composing this layout.
    elements: Arc<Vec<(Point, Element)>>,
}

/// Constructor, accessors and setters.
impl Frame {
    /// Create a new, empty frame.
    ///
    /// Panics the size is not finite.
    #[track_caller]
    pub fn new(size: Size) -> Self {
        assert!(size.is_finite());
        Self { size, baseline: None, elements: Arc::new(vec![]) }
    }

    /// Whether the frame contains no elements.
    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }

    /// The size of the frame.
    pub fn size(&self) -> Size {
        self.size
    }

    /// The size of the frame, mutably.
    pub fn size_mut(&mut self) -> &mut Size {
        &mut self.size
    }

    /// Set the size of the frame.
    pub fn set_size(&mut self, size: Size) {
        self.size = size;
    }

    /// The width of the frame.
    pub fn width(&self) -> Abs {
        self.size.x
    }

    /// The height of the frame.
    pub fn height(&self) -> Abs {
        self.size.y
    }

    /// The vertical position of the frame's baseline.
    pub fn baseline(&self) -> Abs {
        self.baseline.unwrap_or(self.size.y)
    }

    /// Whether the frame has a non-default baseline.
    pub fn has_baseline(&self) -> bool {
        self.baseline.is_some()
    }

    /// Set the frame's baseline from the top.
    pub fn set_baseline(&mut self, baseline: Abs) {
        self.baseline = Some(baseline);
    }

    /// The distance from the baseline to the top of the frame.
    ///
    /// This is the same as `baseline()`, but more in line with the terminology
    /// used in math layout.
    pub fn ascent(&self) -> Abs {
        self.baseline()
    }

    /// The distance from the baseline to the bottom of the frame.
    pub fn descent(&self) -> Abs {
        self.size.y - self.baseline()
    }

    /// An iterator over the elements inside this frame alongside their
    /// positions relative to the top-left of the frame.
    pub fn elements(&self) -> std::slice::Iter<'_, (Point, Element)> {
        self.elements.iter()
    }

    /// Recover the text inside of the frame and its children.
    pub fn text(&self) -> EcoString {
        let mut text = EcoString::new();
        for (_, element) in self.elements() {
            match element {
                Element::Text(element) => {
                    for glyph in &element.glyphs {
                        text.push(glyph.c);
                    }
                }
                Element::Group(group) => text.push_str(&group.frame.text()),
                _ => {}
            }
        }
        text
    }
}

/// Insert elements and subframes.
impl Frame {
    /// The layer the next item will be added on. This corresponds to the number
    /// of elements in the frame.
    pub fn layer(&self) -> usize {
        self.elements.len()
    }

    /// Add an element at a position in the foreground.
    pub fn push(&mut self, pos: Point, element: Element) {
        Arc::make_mut(&mut self.elements).push((pos, element));
    }

    /// Add a frame at a position in the foreground.
    ///
    /// Automatically decides whether to inline the frame or to include it as a
    /// group based on the number of elements in it.
    pub fn push_frame(&mut self, pos: Point, frame: Frame) {
        if self.should_inline(&frame) {
            self.inline(self.layer(), pos, frame);
        } else {
            self.push(pos, Element::Group(Group::new(frame)));
        }
    }

    /// Insert an element at the given layer in the frame.
    ///
    /// This panics if the layer is greater than the number of layers present.
    #[track_caller]
    pub fn insert(&mut self, layer: usize, pos: Point, element: Element) {
        Arc::make_mut(&mut self.elements).insert(layer, (pos, element));
    }

    /// Add an element at a position in the background.
    pub fn prepend(&mut self, pos: Point, element: Element) {
        Arc::make_mut(&mut self.elements).insert(0, (pos, element));
    }

    /// Add multiple elements at a position in the background.
    ///
    /// The first element in the iterator will be the one that is most in the
    /// background.
    pub fn prepend_multiple<I>(&mut self, elements: I)
    where
        I: IntoIterator<Item = (Point, Element)>,
    {
        Arc::make_mut(&mut self.elements).splice(0..0, elements);
    }

    /// Add a frame at a position in the background.
    pub fn prepend_frame(&mut self, pos: Point, frame: Frame) {
        if self.should_inline(&frame) {
            self.inline(0, pos, frame);
        } else {
            self.prepend(pos, Element::Group(Group::new(frame)));
        }
    }

    /// Whether the given frame should be inlined.
    fn should_inline(&self, frame: &Frame) -> bool {
        self.elements.is_empty() || frame.elements.len() <= 5
    }

    /// Inline a frame at the given layer.
    fn inline(&mut self, layer: usize, pos: Point, frame: Frame) {
        // Try to just reuse the elements.
        if pos.is_zero() && self.elements.is_empty() {
            self.elements = frame.elements;
            return;
        }

        // Try to transfer the elements without adjusting the position.
        // Also try to reuse the elements if the Arc isn't shared.
        let range = layer..layer;
        if pos.is_zero() {
            let sink = Arc::make_mut(&mut self.elements);
            match Arc::try_unwrap(frame.elements) {
                Ok(elements) => {
                    sink.splice(range, elements);
                }
                Err(arc) => {
                    sink.splice(range, arc.iter().cloned());
                }
            }
            return;
        }

        // We must adjust the element positions.
        // But still try to reuse the elements if the Arc isn't shared.
        let sink = Arc::make_mut(&mut self.elements);
        match Arc::try_unwrap(frame.elements) {
            Ok(elements) => {
                sink.splice(range, elements.into_iter().map(|(p, e)| (p + pos, e)));
            }
            Err(arc) => {
                sink.splice(range, arc.iter().cloned().map(|(p, e)| (p + pos, e)));
            }
        }
    }
}

/// Modify the frame.
impl Frame {
    /// Remove all elements from the frame.
    pub fn clear(&mut self) {
        if Arc::strong_count(&self.elements) == 1 {
            Arc::make_mut(&mut self.elements).clear();
        } else {
            self.elements = Arc::new(vec![]);
        }
    }

    /// Resize the frame to a new size, distributing new space according to the
    /// given alignments.
    pub fn resize(&mut self, target: Size, aligns: Axes<Align>) {
        if self.size != target {
            let offset = Point::new(
                aligns.x.position(target.x - self.size.x),
                aligns.y.position(target.y - self.size.y),
            );
            self.size = target;
            self.translate(offset);
        }
    }

    /// Move the baseline and contents of the frame by an offset.
    pub fn translate(&mut self, offset: Point) {
        if !offset.is_zero() {
            if let Some(baseline) = &mut self.baseline {
                *baseline += offset.y;
            }
            for (point, _) in Arc::make_mut(&mut self.elements) {
                *point += offset;
            }
        }
    }

    /// Attach the metadata from this style chain to the frame.
    pub fn meta(&mut self, styles: StyleChain) {
        if self.is_empty() {
            return;
        }
        for meta in MetaNode::data_in(styles) {
            if matches!(meta, Meta::Hide) {
                self.clear();
                break;
            }
            self.prepend(Point::zero(), Element::Meta(meta, self.size));
        }
    }

    /// Add a background fill.
    pub fn fill(&mut self, fill: Paint) {
        self.prepend(
            Point::zero(),
            Element::Shape(Geometry::Rect(self.size()).filled(fill)),
        );
    }

    /// Add a fill and stroke with optional radius and outset to the frame.
    pub fn fill_and_stroke(
        &mut self,
        fill: Option<Paint>,
        stroke: Sides<Option<Stroke>>,
        outset: Sides<Rel<Abs>>,
        radius: Corners<Rel<Abs>>,
    ) {
        let outset = outset.relative_to(self.size());
        let size = self.size() + outset.sum_by_axis();
        let pos = Point::new(-outset.left, -outset.top);
        let radius = radius.map(|side| side.relative_to(size.x.min(size.y) / 2.0));
        self.prepend_multiple(
            rounded_rect(size, radius, fill, stroke)
                .into_iter()
                .map(|x| (pos, Element::Shape(x))),
        )
    }

    /// Arbitrarily transform the contents of the frame.
    pub fn transform(&mut self, transform: Transform) {
        if !self.is_empty() {
            self.group(|g| g.transform = transform);
        }
    }

    /// Clip the contents of a frame to its size.
    pub fn clip(&mut self) {
        if !self.is_empty() {
            self.group(|g| g.clips = true);
        }
    }

    /// Wrap the frame's contents in a group and modify that group with `f`.
    fn group<F>(&mut self, f: F)
    where
        F: FnOnce(&mut Group),
    {
        let mut wrapper = Frame::new(self.size);
        wrapper.baseline = self.baseline;
        let mut group = Group::new(std::mem::take(self));
        f(&mut group);
        wrapper.push(Point::zero(), Element::Group(group));
        *self = wrapper;
    }
}

/// Tools for debugging.
impl Frame {
    /// Add a full size aqua background and a red baseline for debugging.
    pub fn debug(mut self) -> Self {
        self.insert(
            0,
            Point::zero(),
            Element::Shape(
                Geometry::Rect(self.size)
                    .filled(RgbaColor { a: 100, ..Color::TEAL.to_rgba() }.into()),
            ),
        );
        self.insert(
            1,
            Point::with_y(self.baseline()),
            Element::Shape(
                Geometry::Line(Point::with_x(self.size.x)).stroked(Stroke {
                    paint: Color::RED.into(),
                    thickness: Abs::pt(1.0),
                }),
            ),
        );
        self
    }

    /// Add a green marker at a position for debugging.
    pub fn mark_point(&mut self, pos: Point) {
        let radius = Abs::pt(2.0);
        self.push(
            pos - Point::splat(radius),
            Element::Shape(geom::ellipse(
                Size::splat(2.0 * radius),
                Some(Color::GREEN.into()),
                None,
            )),
        );
    }

    /// Add a green marker line at a position for debugging.
    pub fn mark_line(&mut self, y: Abs) {
        self.push(
            Point::with_y(y),
            Element::Shape(Geometry::Line(Point::with_x(self.size.x)).stroked(Stroke {
                paint: Color::GREEN.into(),
                thickness: Abs::pt(1.0),
            })),
        );
    }
}

impl Debug for Frame {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str("Frame ")?;
        f.debug_list()
            .entries(self.elements.iter().map(|(_, element)| element))
            .finish()
    }
}

/// The building block frames are composed of.
#[derive(Clone, Hash)]
pub enum Element {
    /// A group of elements.
    Group(Group),
    /// A run of shaped text.
    Text(Text),
    /// A geometric shape with optional fill and stroke.
    Shape(Shape),
    /// An image and its size.
    Image(Image, Size),
    /// Meta information and the region it applies to.
    Meta(Meta, Size),
}

impl Debug for Element {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::Group(group) => group.fmt(f),
            Self::Text(text) => write!(f, "{text:?}"),
            Self::Shape(shape) => write!(f, "{shape:?}"),
            Self::Image(image, _) => write!(f, "{image:?}"),
            Self::Meta(meta, _) => write!(f, "{meta:?}"),
        }
    }
}

/// A group of elements with optional clipping.
#[derive(Clone, Hash)]
pub struct Group {
    /// The group's frame.
    pub frame: Frame,
    /// A transformation to apply to the group.
    pub transform: Transform,
    /// Whether the frame should be a clipping boundary.
    pub clips: bool,
}

impl Group {
    /// Create a new group with default settings.
    pub fn new(frame: Frame) -> Self {
        Self {
            frame,
            transform: Transform::identity(),
            clips: false,
        }
    }
}

impl Debug for Group {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str("Group ")?;
        self.frame.fmt(f)
    }
}

/// A run of shaped text.
#[derive(Clone, Eq, PartialEq, Hash)]
pub struct Text {
    /// The font the glyphs are contained in.
    pub font: Font,
    /// The font size.
    pub size: Abs,
    /// Glyph color.
    pub fill: Paint,
    /// The natural language of the text.
    pub lang: Lang,
    /// The glyphs.
    pub glyphs: Vec<Glyph>,
}

impl Text {
    /// The width of the text run.
    pub fn width(&self) -> Abs {
        self.glyphs.iter().map(|g| g.x_advance).sum::<Em>().at(self.size)
    }
}

impl Debug for Text {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        // This is only a rough approxmiation of the source text.
        f.write_str("Text(\"")?;
        for glyph in &self.glyphs {
            for c in glyph.c.escape_debug() {
                f.write_char(c)?;
            }
        }
        f.write_str("\")")
    }
}

/// A glyph in a run of shaped text.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Glyph {
    /// The glyph's index in the font.
    pub id: u16,
    /// The advance width of the glyph.
    pub x_advance: Em,
    /// The horizontal offset of the glyph.
    pub x_offset: Em,
    /// The first character of the glyph's cluster.
    pub c: char,
    /// The source code location of the text.
    pub span: Span,
    /// The offset within the spanned text.
    pub offset: u16,
}

/// An identifier for a natural language.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Lang([u8; 3], u8);

impl Lang {
    pub const ENGLISH: Self = Self(*b"en ", 2);
    pub const GERMAN: Self = Self(*b"de ", 2);

    /// Return the language code as an all lowercase string slice.
    pub fn as_str(&self) -> &str {
        std::str::from_utf8(&self.0[..usize::from(self.1)]).unwrap_or_default()
    }

    /// The default direction for the language.
    pub fn dir(self) -> Dir {
        match self.as_str() {
            "ar" | "dv" | "fa" | "he" | "ks" | "pa" | "ps" | "sd" | "ug" | "ur"
            | "yi" => Dir::RTL,
            _ => Dir::LTR,
        }
    }
}

impl FromStr for Lang {
    type Err = &'static str;

    /// Construct a language from a two- or three-byte ISO 639-1/2/3 code.
    fn from_str(iso: &str) -> Result<Self, Self::Err> {
        let len = iso.len();
        if matches!(len, 2..=3) && iso.is_ascii() {
            let mut bytes = [b' '; 3];
            bytes[..len].copy_from_slice(iso.as_bytes());
            bytes.make_ascii_lowercase();
            Ok(Self(bytes, len as u8))
        } else {
            Err("expected two or three letter language code (ISO 639-1/2/3)")
        }
    }
}

cast_from_value! {
    Lang,
    string: EcoString => Self::from_str(&string)?,
}

cast_to_value! {
    v: Lang => v.as_str().into()
}

/// An identifier for a region somewhere in the world.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Region([u8; 2]);

impl Region {
    /// Return the region code as an all uppercase string slice.
    pub fn as_str(&self) -> &str {
        std::str::from_utf8(&self.0).unwrap_or_default()
    }
}

impl FromStr for Region {
    type Err = &'static str;

    /// Construct a region from its two-byte ISO 3166-1 alpha-2 code.
    fn from_str(iso: &str) -> Result<Self, Self::Err> {
        if iso.len() == 2 && iso.is_ascii() {
            let mut bytes: [u8; 2] = iso.as_bytes().try_into().unwrap();
            bytes.make_ascii_uppercase();
            Ok(Self(bytes))
        } else {
            Err("expected two letter region code (ISO 3166-1 alpha-2)")
        }
    }
}

cast_from_value! {
    Region,
    string: EcoString => Self::from_str(&string)?,
}

cast_to_value! {
    v: Region => v.as_str().into()
}

/// Meta information that isn't visible or renderable.
#[derive(Debug, Clone, Hash)]
pub enum Meta {
    /// Indicates that the content should be hidden.
    Hide,
    /// An internal or external link.
    Link(Link),
    /// An identifiable piece of content that produces something within the
    /// area this metadata is attached to.
    Node(Content),
}

/// A possibly unresolved link.
#[derive(Debug, Clone, Hash)]
pub enum Link {
    /// A fully resolved.
    Dest(Destination),
    /// An unresolved link to a node.
    Node(StableId),
}

impl Link {
    /// Resolve a destination.
    ///
    /// Needs to lazily provide an introspector.
    pub fn resolve<'a>(
        &self,
        introspector: impl FnOnce() -> &'a Introspector,
    ) -> Option<Destination> {
        match self {
            Self::Dest(dest) => Some(dest.clone()),
            Self::Node(id) => introspector().location(*id).map(Destination::Internal),
        }
    }
}

/// Host for metadata.
///
/// Display: Meta
/// Category: special
#[node]
pub struct MetaNode {
    /// Metadata that should be attached to all elements affected by this style
    /// property.
    #[fold]
    pub data: Vec<Meta>,
}

impl Fold for Vec<Meta> {
    type Output = Self;

    fn fold(mut self, outer: Self::Output) -> Self::Output {
        self.extend(outer);
        self
    }
}

cast_from_value! {
    Meta: "meta",
}

impl PartialEq for Meta {
    fn eq(&self, other: &Self) -> bool {
        crate::util::hash128(self) == crate::util::hash128(other)
    }
}

/// A link destination.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum Destination {
    /// A link to a point on a page.
    Internal(Location),
    /// A link to a URL.
    Url(EcoString),
}

cast_from_value! {
    Destination,
    loc: Location => Self::Internal(loc),
    string: EcoString => Self::Url(string),
}

cast_to_value! {
    v: Destination => match v {
        Destination::Internal(loc) => loc.into(),
        Destination::Url(url) => url.into(),
    }
}

/// A physical location in a document.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Location {
    /// The page, starting at 1.
    pub page: NonZeroUsize,
    /// The exact coordinates on the page (from the top left, as usual).
    pub pos: Point,
}

cast_from_value! {
    Location,
    mut dict: Dict => {
        let page = dict.take("page")?.cast()?;
        let x: Length = dict.take("x")?.cast()?;
        let y: Length = dict.take("y")?.cast()?;
        dict.finish(&["page", "x", "y"])?;
        Self { page, pos: Point::new(x.abs, y.abs) }
    },
}

cast_to_value! {
    v: Location => Value::Dict(dict! {
        "page" => Value::Int(v.page.get() as i64),
        "x" => Value::Length(v.pos.x.into()),
        "y" => Value::Length(v.pos.y.into()),
    })
}
