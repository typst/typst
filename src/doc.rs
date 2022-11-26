//! Finished documents.

use std::fmt::{self, Debug, Formatter, Write};
use std::num::NonZeroUsize;
use std::str::FromStr;
use std::sync::Arc;

use crate::font::Font;
use crate::geom::{
    Abs, Align, Axes, Dir, Em, Numeric, Paint, Point, Shape, Size, Transform,
};
use crate::image::Image;
use crate::model::{dict, Dict, Value};
use crate::util::EcoString;

/// A finished document with metadata and page frames.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Document {
    /// The document's metadata.
    pub metadata: Metadata,
    /// The page frames.
    pub pages: Vec<Frame>,
}

/// Document metadata.
#[derive(Debug, Default, Clone, Eq, PartialEq)]
pub struct Metadata {
    /// The document's title.
    pub title: Option<EcoString>,
    /// The document's author.
    pub author: Option<EcoString>,
}

/// A finished layout with elements at fixed positions.
#[derive(Default, Clone, Eq, PartialEq)]
pub struct Frame {
    /// The size of the frame.
    size: Size,
    /// The baseline of the frame measured from the top. If this is `None`, the
    /// frame's implicit baseline is at the bottom.
    baseline: Option<Abs>,
    /// The semantic role of the frame.
    role: Option<Role>,
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
        Self {
            size,
            baseline: None,
            role: None,
            elements: Arc::new(vec![]),
        }
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

    /// The baseline of the frame.
    pub fn baseline(&self) -> Abs {
        self.baseline.unwrap_or(self.size.y)
    }

    /// Set the frame's baseline from the top.
    pub fn set_baseline(&mut self, baseline: Abs) {
        self.baseline = Some(baseline);
    }

    /// The role of the frame.
    pub fn role(&self) -> Option<Role> {
        self.role
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
                Element::Text(content) => {
                    for glyph in &content.glyphs {
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

/// Inserting elements and subframes.
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
    /// group based on the number of elements in and the role of the frame.
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
        (self.elements.is_empty() || frame.elements.len() <= 5)
            && frame.role().map_or(true, |role| role.is_weak())
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

    /// Link the whole frame to a resource.
    pub fn link(&mut self, dest: Destination) {
        self.push(Point::zero(), Element::Link(dest, self.size));
    }

    /// Arbitrarily transform the contents of the frame.
    pub fn transform(&mut self, transform: Transform) {
        self.group(|g| g.transform = transform);
    }

    /// Clip the contents of a frame to its size.
    pub fn clip(&mut self) {
        self.group(|g| g.clips = true);
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

impl Debug for Frame {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        if let Some(role) = self.role {
            write!(f, "{role:?} ")?;
        }

        f.debug_list()
            .entries(self.elements.iter().map(|(_, element)| element))
            .finish()
    }
}

/// The building block frames are composed of.
#[derive(Clone, Eq, PartialEq)]
pub enum Element {
    /// A group of elements.
    Group(Group),
    /// A run of shaped text.
    Text(Text),
    /// A geometric shape with optional fill and stroke.
    Shape(Shape),
    /// An image and its size.
    Image(Image, Size),
    /// A link to an external resource and its trigger region.
    Link(Destination, Size),
}

impl Debug for Element {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::Group(group) => group.fmt(f),
            Self::Text(text) => write!(f, "{text:?}"),
            Self::Shape(shape) => write!(f, "{shape:?}"),
            Self::Image(image, _) => write!(f, "{image:?}"),
            Self::Link(dest, _) => write!(f, "Link({dest:?})"),
        }
    }
}

/// A group of elements with optional clipping.
#[derive(Clone, Eq, PartialEq)]
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
#[derive(Clone, Eq, PartialEq)]
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
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Glyph {
    /// The glyph's index in the font.
    pub id: u16,
    /// The advance width of the glyph.
    pub x_advance: Em,
    /// The horizontal offset of the glyph.
    pub x_offset: Em,
    /// The first character of the glyph's cluster.
    pub c: char,
}

/// An identifier for a natural language.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Lang([u8; 3], u8);

impl Lang {
    /// The code for the english language.
    pub const ENGLISH: Self = Self(*b"en ", 2);

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

/// A link destination.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum Destination {
    /// A link to a point on a page.
    Internal(Location),
    /// A link to a URL.
    Url(EcoString),
}

/// A physical location in a document.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Location {
    /// The page, starting at 1.
    pub page: NonZeroUsize,
    /// The exact coordinates on the page (from the top left, as usual).
    pub pos: Point,
}

impl Location {
    /// Encode into a user-facing dictionary.
    pub fn encode(&self) -> Dict {
        dict! {
            "page" => Value::Int(self.page.get() as i64),
            "x" => Value::Length(self.pos.x.into()),
            "y" => Value::Length(self.pos.y.into()),
        }
    }
}

/// A semantic role of a frame.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Role {
    /// A paragraph.
    Paragraph,
    /// A heading of the given level and whether it should be part of the
    /// outline.
    Heading { level: NonZeroUsize, outlined: bool },
    /// A generic block-level subdivision.
    GenericBlock,
    /// A generic inline subdivision.
    GenericInline,
    /// A list and whether it is ordered.
    List { ordered: bool },
    /// A list item. Must have a list parent.
    ListItem,
    /// The label of a list item. Must have a list item parent.
    ListLabel,
    /// The body of a list item. Must have a list item parent.
    ListItemBody,
    /// A mathematical formula.
    Formula,
    /// A table.
    Table,
    /// A table row. Must have a table parent.
    TableRow,
    /// A table cell. Must have a table row parent.
    TableCell,
    /// A code fragment.
    Code,
    /// A page header.
    Header,
    /// A page footer.
    Footer,
    /// A page background.
    Background,
    /// A page foreground.
    Foreground,
}

impl Role {
    /// Whether the role describes a generic element and is not very
    /// descriptive.
    pub fn is_weak(self) -> bool {
        // In Typst, all text is in a paragraph, so paragraph isn't very
        // descriptive.
        matches!(self, Self::Paragraph | Self::GenericBlock | Self::GenericInline)
    }
}
