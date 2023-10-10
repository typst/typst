//! Finished documents.

use std::fmt::{self, Debug, Formatter};
use std::num::NonZeroUsize;
use std::ops::Range;
use std::str::FromStr;
use std::sync::Arc;

use ecow::{eco_format, EcoString};

use crate::eval::{cast, dict, ty, Dict, Repr, Value};
use crate::export::PdfPageLabel;
use crate::font::Font;
use crate::geom::{
    self, styled_rect, Abs, Axes, Color, Corners, Dir, Em, FixedAlign, FixedStroke,
    Geometry, Length, Numeric, Paint, Path, Point, Rel, Shape, Sides, Size, Transform,
};
use crate::image::Image;
use crate::model::{Content, Location, MetaElem, StyleChain};
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
    /// The document's keywords.
    pub keywords: Vec<EcoString>,
}

/// A finished layout with items at fixed positions.
#[derive(Default, Clone, Hash)]
pub struct Frame {
    /// The size of the frame.
    size: Size,
    /// The baseline of the frame measured from the top. If this is `None`, the
    /// frame's implicit baseline is at the bottom.
    baseline: Option<Abs>,
    /// The items composing this layout.
    items: Arc<Vec<(Point, FrameItem)>>,
    /// The hardness of this frame.
    kind: FrameKind,
}

/// Constructor, accessors and setters.
impl Frame {
    /// Create a new, empty frame.
    ///
    /// Panics the size is not finite.
    #[track_caller]
    pub fn new(size: Size, kind: FrameKind) -> Self {
        assert!(size.is_finite());
        Self {
            size,
            baseline: None,
            items: Arc::new(vec![]),
            kind,
        }
    }

    /// Create a new, empty soft frame.
    ///
    /// Panics the size is not finite.
    #[track_caller]
    pub fn soft(size: Size) -> Self {
        Self::new(size, FrameKind::Soft)
    }

    /// Create a new, empty hard frame.
    ///
    /// Panics if the size is not finite.
    #[track_caller]
    pub fn hard(size: Size) -> Self {
        Self::new(size, FrameKind::Hard)
    }

    /// Sets the frame's hardness.
    pub fn set_kind(&mut self, kind: FrameKind) {
        self.kind = kind;
    }

    /// Whether the frame is hard or soft.
    pub fn kind(&self) -> FrameKind {
        self.kind
    }

    /// Whether the frame contains no items.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
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

    /// An iterator over the items inside this frame alongside their positions
    /// relative to the top-left of the frame.
    pub fn items(&self) -> std::slice::Iter<'_, (Point, FrameItem)> {
        self.items.iter()
    }
}

/// Insert items and subframes.
impl Frame {
    /// The layer the next item will be added on. This corresponds to the number
    /// of items in the frame.
    pub fn layer(&self) -> usize {
        self.items.len()
    }

    /// Add an item at a position in the foreground.
    pub fn push(&mut self, pos: Point, item: FrameItem) {
        Arc::make_mut(&mut self.items).push((pos, item));
    }

    /// Add a frame at a position in the foreground.
    ///
    /// Automatically decides whether to inline the frame or to include it as a
    /// group based on the number of items in it.
    pub fn push_frame(&mut self, pos: Point, frame: Frame) {
        if self.should_inline(&frame) {
            self.inline(self.layer(), pos, frame);
        } else {
            self.push(pos, FrameItem::Group(GroupItem::new(frame)));
        }
    }

    /// Add zero-sized metadata at the origin.
    pub fn push_positionless_meta(&mut self, meta: Meta) {
        self.push(Point::zero(), FrameItem::Meta(meta, Size::zero()));
    }

    /// Insert an item at the given layer in the frame.
    ///
    /// This panics if the layer is greater than the number of layers present.
    #[track_caller]
    pub fn insert(&mut self, layer: usize, pos: Point, items: FrameItem) {
        Arc::make_mut(&mut self.items).insert(layer, (pos, items));
    }

    /// Add an item at a position in the background.
    pub fn prepend(&mut self, pos: Point, item: FrameItem) {
        Arc::make_mut(&mut self.items).insert(0, (pos, item));
    }

    /// Add multiple items at a position in the background.
    ///
    /// The first item in the iterator will be the one that is most in the
    /// background.
    pub fn prepend_multiple<I>(&mut self, items: I)
    where
        I: IntoIterator<Item = (Point, FrameItem)>,
    {
        Arc::make_mut(&mut self.items).splice(0..0, items);
    }

    /// Add a frame at a position in the background.
    pub fn prepend_frame(&mut self, pos: Point, frame: Frame) {
        if self.should_inline(&frame) {
            self.inline(0, pos, frame);
        } else {
            self.prepend(pos, FrameItem::Group(GroupItem::new(frame)));
        }
    }

    /// Whether the given frame should be inlined.
    fn should_inline(&self, frame: &Frame) -> bool {
        // We do not inline big frames and hard frames.
        frame.kind().is_soft() && (self.items.is_empty() || frame.items.len() <= 5)
    }

    /// Inline a frame at the given layer.
    fn inline(&mut self, layer: usize, pos: Point, frame: Frame) {
        // Try to just reuse the items.
        if pos.is_zero() && self.items.is_empty() {
            self.items = frame.items;
            return;
        }

        // Try to transfer the items without adjusting the position.
        // Also try to reuse the items if the Arc isn't shared.
        let range = layer..layer;
        if pos.is_zero() {
            let sink = Arc::make_mut(&mut self.items);
            match Arc::try_unwrap(frame.items) {
                Ok(items) => {
                    sink.splice(range, items);
                }
                Err(arc) => {
                    sink.splice(range, arc.iter().cloned());
                }
            }
            return;
        }

        // We have to adjust the item positions.
        // But still try to reuse the items if the Arc isn't shared.
        let sink = Arc::make_mut(&mut self.items);
        match Arc::try_unwrap(frame.items) {
            Ok(items) => {
                sink.splice(range, items.into_iter().map(|(p, e)| (p + pos, e)));
            }
            Err(arc) => {
                sink.splice(range, arc.iter().cloned().map(|(p, e)| (p + pos, e)));
            }
        }
    }
}

/// Modify the frame.
impl Frame {
    /// Remove all items from the frame.
    pub fn clear(&mut self) {
        if Arc::strong_count(&self.items) == 1 {
            Arc::make_mut(&mut self.items).clear();
        } else {
            self.items = Arc::new(vec![]);
        }
    }

    /// Resize the frame to a new size, distributing new space according to the
    /// given alignments.
    pub fn resize(&mut self, target: Size, align: Axes<FixedAlign>) {
        if self.size != target {
            let offset = align.zip_map(target - self.size, FixedAlign::position);
            self.size = target;
            self.translate(offset.to_point());
        }
    }

    /// Move the baseline and contents of the frame by an offset.
    pub fn translate(&mut self, offset: Point) {
        if !offset.is_zero() {
            if let Some(baseline) = &mut self.baseline {
                *baseline += offset.y;
            }
            for (point, _) in Arc::make_mut(&mut self.items) {
                *point += offset;
            }
        }
    }

    /// Attach the metadata from this style chain to the frame.
    pub fn meta(&mut self, styles: StyleChain, force: bool) {
        if force || !self.is_empty() {
            self.meta_iter(MetaElem::data_in(styles));
        }
    }

    /// Attach metadata from an iterator.
    pub fn meta_iter(&mut self, iter: impl IntoIterator<Item = Meta>) {
        let mut hide = false;
        for meta in iter {
            if matches!(meta, Meta::Hide) {
                hide = true;
            } else {
                self.prepend(Point::zero(), FrameItem::Meta(meta, self.size));
            }
        }
        if hide {
            Arc::make_mut(&mut self.items).retain(|(_, item)| {
                matches!(item, FrameItem::Group(_) | FrameItem::Meta(Meta::Elem(_), _))
            });
        }
    }

    /// Add a background fill.
    pub fn fill(&mut self, fill: Paint) {
        self.prepend(
            Point::zero(),
            FrameItem::Shape(Geometry::Rect(self.size()).filled(fill), Span::detached()),
        );
    }

    /// Add a fill and stroke with optional radius and outset to the frame.
    pub fn fill_and_stroke(
        &mut self,
        fill: Option<Paint>,
        stroke: Sides<Option<FixedStroke>>,
        outset: Sides<Rel<Abs>>,
        radius: Corners<Rel<Abs>>,
        span: Span,
    ) {
        let outset = outset.relative_to(self.size());
        let size = self.size() + outset.sum_by_axis();
        let pos = Point::new(-outset.left, -outset.top);
        self.prepend_multiple(
            styled_rect(size, radius, fill, stroke)
                .into_iter()
                .map(|x| (pos, FrameItem::Shape(x, span))),
        )
    }

    /// Arbitrarily transform the contents of the frame.
    pub fn transform(&mut self, transform: Transform) {
        if !self.is_empty() {
            self.group(|g| g.transform = transform);
        }
    }

    /// Clip the contents of a frame to a clip path.
    ///
    /// The clip path can be the size of the frame in the case of a
    /// rectangular frame. In the case of a frame with rounded corner,
    /// this should be a path that matches the frame's outline.
    pub fn clip(&mut self, clip_path: Path) {
        if !self.is_empty() {
            self.group(|g| g.clip_path = Some(clip_path));
        }
    }

    /// Wrap the frame's contents in a group and modify that group with `f`.
    fn group<F>(&mut self, f: F)
    where
        F: FnOnce(&mut GroupItem),
    {
        let mut wrapper = Frame::soft(self.size);
        wrapper.baseline = self.baseline;
        let mut group = GroupItem::new(std::mem::take(self));
        f(&mut group);
        wrapper.push(Point::zero(), FrameItem::Group(group));
        *self = wrapper;
    }
}

/// Tools for debugging.
impl Frame {
    /// Add a full size aqua background and a red baseline for debugging.
    pub fn debug(mut self) -> Self {
        self.debug_in_place();
        self
    }

    /// Debug in place.
    pub fn debug_in_place(&mut self) {
        self.insert(
            0,
            Point::zero(),
            FrameItem::Shape(
                Geometry::Rect(self.size).filled(Color::TEAL.with_alpha(0.5).into()),
                Span::detached(),
            ),
        );
        self.insert(
            1,
            Point::with_y(self.baseline()),
            FrameItem::Shape(
                Geometry::Line(Point::with_x(self.size.x)).stroked(FixedStroke {
                    paint: Color::RED.into(),
                    thickness: Abs::pt(1.0),
                    ..FixedStroke::default()
                }),
                Span::detached(),
            ),
        );
    }

    /// Add a green marker at a position for debugging.
    pub fn mark_point(&mut self, pos: Point) {
        let radius = Abs::pt(2.0);
        self.push(
            pos - Point::splat(radius),
            FrameItem::Shape(
                geom::ellipse(Size::splat(2.0 * radius), Some(Color::GREEN.into()), None),
                Span::detached(),
            ),
        );
    }

    /// Add a green marker line at a position for debugging.
    pub fn mark_line(&mut self, y: Abs) {
        self.push(
            Point::with_y(y),
            FrameItem::Shape(
                Geometry::Line(Point::with_x(self.size.x)).stroked(FixedStroke {
                    paint: Color::GREEN.into(),
                    thickness: Abs::pt(1.0),
                    ..FixedStroke::default()
                }),
                Span::detached(),
            ),
        );
    }
}

impl Debug for Frame {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str("Frame ")?;
        f.debug_list()
            .entries(self.items.iter().map(|(_, item)| item))
            .finish()
    }
}

/// The hardness of a frame.
///
/// This corresponds to whether or not the frame is considered to be the
/// innermost parent of its contents. This is used to determine the coordinate
/// reference system for gradients.
#[derive(Default, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum FrameKind {
    /// A container which follows its parent's size.
    ///
    /// Soft frames are the default since they do not impact the layout of
    /// a gradient set on one of its children.
    #[default]
    Soft,
    /// A container which uses its own size.
    Hard,
}

impl FrameKind {
    /// Returns `true` if the frame is soft.
    pub fn is_soft(self) -> bool {
        matches!(self, Self::Soft)
    }

    /// Returns `true` if the frame is hard.
    pub fn is_hard(self) -> bool {
        matches!(self, Self::Hard)
    }
}

/// The building block frames are composed of.
#[derive(Clone, Hash)]
pub enum FrameItem {
    /// A subframe with optional transformation and clipping.
    Group(GroupItem),
    /// A run of shaped text.
    Text(TextItem),
    /// A geometric shape with optional fill and stroke.
    Shape(Shape, Span),
    /// An image and its size.
    Image(Image, Size, Span),
    /// Meta information and the region it applies to.
    Meta(Meta, Size),
}

impl Debug for FrameItem {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::Group(group) => group.fmt(f),
            Self::Text(text) => write!(f, "{text:?}"),
            Self::Shape(shape, _) => write!(f, "{shape:?}"),
            Self::Image(image, _, _) => write!(f, "{image:?}"),
            Self::Meta(meta, _) => write!(f, "{meta:?}"),
        }
    }
}

/// A subframe with optional transformation and clipping.
#[derive(Clone, Hash)]
pub struct GroupItem {
    /// The group's frame.
    pub frame: Frame,
    /// A transformation to apply to the group.
    pub transform: Transform,
    /// Whether the frame should be a clipping boundary.
    pub clip_path: Option<Path>,
}

impl GroupItem {
    /// Create a new group with default settings.
    pub fn new(frame: Frame) -> Self {
        Self {
            frame,
            transform: Transform::identity(),
            clip_path: None,
        }
    }
}

impl Debug for GroupItem {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str("Group ")?;
        self.frame.fmt(f)
    }
}

/// A run of shaped text.
#[derive(Clone, Eq, PartialEq, Hash)]
pub struct TextItem {
    /// The font the glyphs are contained in.
    pub font: Font,
    /// The font size.
    pub size: Abs,
    /// Glyph color.
    pub fill: Paint,
    /// The natural language of the text.
    pub lang: Lang,
    /// The item's plain text.
    pub text: EcoString,
    /// The glyphs.
    pub glyphs: Vec<Glyph>,
}

impl TextItem {
    /// The width of the text run.
    pub fn width(&self) -> Abs {
        self.glyphs.iter().map(|g| g.x_advance).sum::<Em>().at(self.size)
    }
}

impl Debug for TextItem {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str("Text(")?;
        self.text.fmt(f)?;
        f.write_str(")")
    }
}

/// A glyph in a run of shaped text.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Glyph {
    /// The glyph's index in the font.
    pub id: u16,
    /// The advance width of the glyph.
    pub x_advance: Em,
    /// The horizontal offset of the glyph.
    pub x_offset: Em,
    /// The range of the glyph in its item's text.
    pub range: Range<u16>,
    /// The source code location of the text.
    pub span: (Span, u16),
}

impl Glyph {
    /// The range of the glyph in its item's text.
    pub fn range(&self) -> Range<usize> {
        usize::from(self.range.start)..usize::from(self.range.end)
    }
}

/// An ISO 15924-type script identifier
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct WritingScript([u8; 4], u8);

impl WritingScript {
    /// Return the script as an all lowercase string slice.
    pub fn as_str(&self) -> &str {
        std::str::from_utf8(&self.0[..usize::from(self.1)]).unwrap_or_default()
    }

    /// Return the description of the script as raw bytes.
    pub fn as_bytes(&self) -> &[u8; 4] {
        &self.0
    }
}

impl FromStr for WritingScript {
    type Err = &'static str;

    /// Construct a region from its ISO 15924 code.
    fn from_str(iso: &str) -> Result<Self, Self::Err> {
        let len = iso.len();
        if matches!(len, 3..=4) && iso.is_ascii() {
            let mut bytes = [b' '; 4];
            bytes[..len].copy_from_slice(iso.as_bytes());
            bytes.make_ascii_lowercase();
            Ok(Self(bytes, len as u8))
        } else {
            Err("expected three or four letter script code (ISO 15924 or 'math')")
        }
    }
}

cast! {
    WritingScript,
    self => self.as_str().into_value(),
    string: EcoString => Self::from_str(&string)?,
}

/// An identifier for a natural language.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Lang([u8; 3], u8);

impl Lang {
    pub const ALBANIAN: Self = Self(*b"sq ", 2);
    pub const ARABIC: Self = Self(*b"ar ", 2);
    pub const BOKMÅL: Self = Self(*b"nb ", 2);
    pub const CHINESE: Self = Self(*b"zh ", 2);
    pub const CZECH: Self = Self(*b"cs ", 2);
    pub const DANISH: Self = Self(*b"da ", 2);
    pub const DUTCH: Self = Self(*b"nl ", 2);
    pub const ENGLISH: Self = Self(*b"en ", 2);
    pub const FILIPINO: Self = Self(*b"tl ", 2);
    pub const FINNISH: Self = Self(*b"fi ", 2);
    pub const FRENCH: Self = Self(*b"fr ", 2);
    pub const GERMAN: Self = Self(*b"de ", 2);
    pub const ITALIAN: Self = Self(*b"it ", 2);
    pub const JAPANESE: Self = Self(*b"ja ", 2);
    pub const NYNORSK: Self = Self(*b"nn ", 2);
    pub const POLISH: Self = Self(*b"pl ", 2);
    pub const PORTUGUESE: Self = Self(*b"pt ", 2);
    pub const RUSSIAN: Self = Self(*b"ru ", 2);
    pub const SLOVENIAN: Self = Self(*b"sl ", 2);
    pub const SPANISH: Self = Self(*b"es ", 2);
    pub const SWEDISH: Self = Self(*b"sv ", 2);
    pub const TURKISH: Self = Self(*b"tr ", 2);
    pub const UKRAINIAN: Self = Self(*b"ua ", 2);
    pub const VIETNAMESE: Self = Self(*b"vi ", 2);
    pub const HUNGARIAN: Self = Self(*b"hu ", 2);
    pub const ROMANIAN: Self = Self(*b"ro ", 2);

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

cast! {
    Lang,
    self => self.as_str().into_value(),
    string: EcoString => Self::from_str(&string)?,
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

impl PartialEq<&str> for Region {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
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

cast! {
    Region,
    self => self.as_str().into_value(),
    string: EcoString => Self::from_str(&string)?,
}

/// Meta information that isn't visible or renderable.
#[ty]
#[derive(Clone, PartialEq, Hash)]
pub enum Meta {
    /// An internal or external link to a destination.
    Link(Destination),
    /// An identifiable element that produces something within the area this
    /// metadata is attached to.
    Elem(Content),
    /// The numbering of the current page.
    PageNumbering(Value),
    /// A PDF page label of the current page.
    PdfPageLabel(PdfPageLabel),
    /// Indicates that content should be hidden. This variant doesn't appear
    /// in the final frames as it is removed alongside the content that should
    /// be hidden.
    Hide,
}

cast! {
    type Meta,
}

impl Debug for Meta {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::Link(dest) => write!(f, "Link({dest:?})"),
            Self::Elem(content) => write!(f, "Elem({:?})", content.func()),
            Self::PageNumbering(value) => write!(f, "PageNumbering({value:?})"),
            Self::PdfPageLabel(label) => write!(f, "PdfPageLabel({label:?})"),
            Self::Hide => f.pad("Hide"),
        }
    }
}

impl Repr for Meta {
    fn repr(&self) -> EcoString {
        eco_format!("{self:?}")
    }
}

/// A link destination.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum Destination {
    /// A link to a URL.
    Url(EcoString),
    /// A link to a point on a page.
    Position(Position),
    /// An unresolved link to a location in the document.
    Location(Location),
}

impl Repr for Destination {
    fn repr(&self) -> EcoString {
        eco_format!("{self:?}")
    }
}

cast! {
    Destination,
    self => match self {
        Self::Url(v) => v.into_value(),
        Self::Position(v) => v.into_value(),
        Self::Location(v) => v.into_value(),
    },
    v: EcoString => Self::Url(v),
    v: Position => Self::Position(v),
    v: Location => Self::Location(v),
}

/// A physical position in a document.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Position {
    /// The page, starting at 1.
    pub page: NonZeroUsize,
    /// The exact coordinates on the page (from the top left, as usual).
    pub point: Point,
}

cast! {
    Position,
    self => Value::Dict(self.into()),
    mut dict: Dict => {
        let page = dict.take("page")?.cast()?;
        let x: Length = dict.take("x")?.cast()?;
        let y: Length = dict.take("y")?.cast()?;
        dict.finish(&["page", "x", "y"])?;
        Self { page, point: Point::new(x.abs, y.abs) }
    },
}

impl From<Position> for Dict {
    fn from(pos: Position) -> Self {
        dict! {
            "page" => pos.page,
            "x" => pos.point.x,
            "y" => pos.point.y,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::option_eq;

    #[test]
    fn test_region_option_eq() {
        let region = Some(Region([b'U', b'S']));
        assert!(option_eq(region, "US"));
        assert!(!option_eq(region, "AB"));
    }

    #[test]
    fn test_document_is_send() {
        fn ensure_send<T: Send>() {}
        ensure_send::<Document>();
    }
}
