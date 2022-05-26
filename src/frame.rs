//! Finished layouts.

use std::fmt::{self, Debug, Formatter, Write};
use std::sync::Arc;

use crate::font::FaceId;
use crate::geom::{
    Align, Em, Length, Numeric, Paint, Point, Shape, Size, Spec, Transform,
};
use crate::image::ImageId;
use crate::library::text::Lang;
use crate::util::{EcoString, MaybeShared};

/// A finished layout with elements at fixed positions.
#[derive(Default, Clone, Eq, PartialEq)]
pub struct Frame {
    /// The size of the frame.
    pub size: Size,
    /// The baseline of the frame measured from the top. If this is `None`, the
    /// frame's implicit baseline is at the bottom.
    pub baseline: Option<Length>,
    /// The elements composing this layout.
    pub elements: Vec<(Point, Element)>,
}

impl Frame {
    /// Create a new, empty frame.
    #[track_caller]
    pub fn new(size: Size) -> Self {
        assert!(size.is_finite());
        Self { size, baseline: None, elements: vec![] }
    }

    /// The baseline of the frame.
    pub fn baseline(&self) -> Length {
        self.baseline.unwrap_or(self.size.y)
    }

    /// Add an element at a position in the background.
    pub fn prepend(&mut self, pos: Point, element: Element) {
        self.elements.insert(0, (pos, element));
    }

    /// Add multiple elements at a position in the background.
    pub fn prepend_multiple<I>(&mut self, insert: I)
    where
        I: IntoIterator<Item = (Point, Element)>,
    {
        self.elements.splice(0 .. 0, insert);
    }

    /// Add an element at a position in the foreground.
    pub fn push(&mut self, pos: Point, element: Element) {
        self.elements.push((pos, element));
    }

    /// The layer the next item will be added on. This corresponds to the number
    /// of elements in the frame.
    pub fn layer(&self) -> usize {
        self.elements.len()
    }

    /// Insert an element at the given layer in the frame.
    ///
    /// This panics if the layer is greater than the number of layers present.
    pub fn insert(&mut self, layer: usize, pos: Point, element: Element) {
        self.elements.insert(layer, (pos, element));
    }

    /// Add a frame.
    ///
    /// Automatically decides whether to inline the frame or to include it as a
    /// group based on the number of elements in the frame.
    pub fn push_frame(&mut self, pos: Point, frame: impl FrameRepr) {
        if self.elements.is_empty() || frame.as_ref().elements.len() <= 5 {
            frame.inline(self, pos);
        } else {
            self.elements.push((pos, Element::Group(Group::new(frame.share()))));
        }
    }

    /// Resize the frame to a new size, distributing new space according to the
    /// given alignments.
    pub fn resize(&mut self, target: Size, aligns: Spec<Align>) {
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
            for (point, _) in &mut self.elements {
                *point += offset;
            }
        }
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
    pub fn group<F>(&mut self, f: F)
    where
        F: FnOnce(&mut Group),
    {
        let mut wrapper = Frame { elements: vec![], ..*self };
        let mut group = Group::new(Arc::new(std::mem::take(self)));
        f(&mut group);
        wrapper.push(Point::zero(), Element::Group(group));
        *self = wrapper;
    }

    /// Link the whole frame to a resource.
    pub fn link(&mut self, dest: Destination) {
        self.push(Point::zero(), Element::Link(dest, self.size));
    }
}

impl Debug for Frame {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_list()
            .entries(self.elements.iter().map(|(_, element)| element))
            .finish()
    }
}

impl AsRef<Frame> for Frame {
    fn as_ref(&self) -> &Frame {
        self
    }
}

/// A representational form of a frame (owned, shared or maybe shared).
pub trait FrameRepr: AsRef<Frame> {
    /// Transform into a shared representation.
    fn share(self) -> Arc<Frame>;

    /// Inline `self` into the sink frame.
    fn inline(self, sink: &mut Frame, offset: Point);
}

impl FrameRepr for Frame {
    fn share(self) -> Arc<Frame> {
        Arc::new(self)
    }

    fn inline(self, sink: &mut Frame, offset: Point) {
        if offset.is_zero() {
            if sink.elements.is_empty() {
                sink.elements = self.elements;
            } else {
                sink.elements.extend(self.elements);
            }
        } else {
            sink.elements
                .extend(self.elements.into_iter().map(|(p, e)| (p + offset, e)));
        }
    }
}

impl FrameRepr for Arc<Frame> {
    fn share(self) -> Arc<Frame> {
        self
    }

    fn inline(self, sink: &mut Frame, offset: Point) {
        match Arc::try_unwrap(self) {
            Ok(frame) => frame.inline(sink, offset),
            Err(rc) => sink
                .elements
                .extend(rc.elements.iter().cloned().map(|(p, e)| (p + offset, e))),
        }
    }
}

impl FrameRepr for MaybeShared<Frame> {
    fn share(self) -> Arc<Frame> {
        match self {
            Self::Owned(owned) => owned.share(),
            Self::Shared(shared) => shared.share(),
        }
    }

    fn inline(self, sink: &mut Frame, offset: Point) {
        match self {
            Self::Owned(owned) => owned.inline(sink, offset),
            Self::Shared(shared) => shared.inline(sink, offset),
        }
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
    Image(ImageId, Size),
    /// A link to an external resource and its trigger region.
    Link(Destination, Size),
    /// A pin identified by index.
    Pin(usize),
}

impl Debug for Element {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::Group(group) => group.fmt(f),
            Self::Text(text) => write!(f, "{text:?}"),
            Self::Shape(shape) => write!(f, "{shape:?}"),
            Self::Image(image, _) => write!(f, "{image:?}"),
            Self::Link(dest, _) => write!(f, "Link({dest:?})"),
            Self::Pin(idx) => write!(f, "Pin({idx})"),
        }
    }
}

/// A group of elements with optional clipping.
#[derive(Clone, Eq, PartialEq)]
pub struct Group {
    /// The group's frame.
    pub frame: Arc<Frame>,
    /// A transformation to apply to the group.
    pub transform: Transform,
    /// Whether the frame should be a clipping boundary.
    pub clips: bool,
}

impl Group {
    /// Create a new group with default settings.
    pub fn new(frame: Arc<Frame>) -> Self {
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
    /// The font face the glyphs are contained in.
    pub face_id: FaceId,
    /// The font size.
    pub size: Length,
    /// Glyph color.
    pub fill: Paint,
    /// The natural language of the text.
    pub lang: Lang,
    /// The glyphs.
    pub glyphs: Vec<Glyph>,
}

impl Text {
    /// The width of the text run.
    pub fn width(&self) -> Length {
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
    /// The glyph's index in the face.
    pub id: u16,
    /// The advance width of the glyph.
    pub x_advance: Em,
    /// The horizontal offset of the glyph.
    pub x_offset: Em,
    /// The first character of the glyph's cluster.
    pub c: char,
}

/// A link destination.
#[derive(Clone, Eq, PartialEq, Hash)]
pub enum Destination {
    /// A link to a point on a page.
    Internal(usize, Point),
    /// A link to a URL.
    Url(EcoString),
}

impl Debug for Destination {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::Internal(page, point) => {
                write!(f, "Internal(Page {}, {:?})", page, point)
            }
            Self::Url(url) => write!(f, "Url({})", url),
        }
    }
}
