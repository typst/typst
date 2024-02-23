//! Finished documents.

use std::fmt::{self, Debug, Formatter};
use std::num::NonZeroUsize;
use std::sync::Arc;

use crate::foundations::{cast, dict, Dict, StyleChain, Value};
use crate::introspection::{Meta, MetaElem};
use crate::layout::{
    Abs, Axes, Corners, FixedAlignment, Length, Point, Rel, Sides, Size, Transform,
};
use crate::syntax::Span;
use crate::text::TextItem;
use crate::util::Numeric;
use crate::visualize::{
    ellipse, styled_rect, Color, FixedStroke, Geometry, Image, Paint, Path, Shape,
};

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
    /// Panics if the size is not finite.
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
    pub fn insert(&mut self, layer: usize, pos: Point, item: FrameItem) {
        Arc::make_mut(&mut self.items).insert(layer, (pos, item));
    }

    /// Add an item at a position in the background.
    pub fn prepend(&mut self, pos: Point, item: FrameItem) {
        self.insert(0, pos, item);
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

    /// Adjust the frame's size, translate the original content by an offset
    /// computed according to the given alignments, and return the amount of
    /// offset.
    pub fn resize(&mut self, target: Size, align: Axes<FixedAlignment>) -> Point {
        if self.size == target {
            return Point::zero();
        }
        let offset =
            align.zip_map(target - self.size, FixedAlignment::position).to_point();
        self.size = target;
        self.translate(offset);
        offset
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
        let size = self.size;
        self.prepend_multiple(iter.into_iter().filter_map(|meta| {
            if matches!(meta, Meta::Hide) {
                hide = true;
                None
            } else {
                Some((Point::zero(), FrameItem::Meta(meta, size)))
            }
        }));
        if hide {
            self.hide();
        }
    }

    /// Hide all content in the frame, but keep metadata.
    pub fn hide(&mut self) {
        Arc::make_mut(&mut self.items).retain_mut(|(_, item)| match item {
            FrameItem::Group(group) => {
                group.frame.hide();
                !group.frame.is_empty()
            }
            FrameItem::Meta(Meta::Elem(_), _) => true,
            _ => false,
        });
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
    pub fn mark_box(mut self) -> Self {
        self.mark_box_in_place();
        self
    }

    /// Debug in place. Add a full size aqua background and a red baseline for debugging.
    pub fn mark_box_in_place(&mut self) {
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
                Geometry::Line(Point::with_x(self.size.x))
                    .stroked(FixedStroke::from_pair(Color::RED, Abs::pt(1.0))),
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
                ellipse(Size::splat(2.0 * radius), Some(Color::GREEN.into()), None),
                Span::detached(),
            ),
        );
    }

    /// Add a green marker line at a position for debugging.
    pub fn mark_line(&mut self, y: Abs) {
        self.push(
            Point::with_y(y),
            FrameItem::Shape(
                Geometry::Line(Point::with_x(self.size.x))
                    .stroked(FixedStroke::from_pair(Color::GREEN, Abs::pt(1.0))),
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
