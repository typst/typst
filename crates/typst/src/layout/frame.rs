//! Finished documents.

use std::fmt::{self, Debug, Formatter};
use std::num::NonZeroUsize;
use std::sync::Arc;

use smallvec::SmallVec;

use crate::foundations::{cast, dict, Dict, Label, StyleChain, Value};
use crate::introspection::{Location, Tag};
use crate::layout::{
    Abs, Axes, Corners, FixedAlignment, HideElem, Length, Point, Rel, Sides, Size,
    Transform,
};
use crate::model::{Destination, LinkElem};
use crate::syntax::Span;
use crate::text::TextItem;
use crate::utils::{LazyHash, Numeric};
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
    items: Arc<LazyHash<Vec<(Point, FrameItem)>>>,
    /// The hardness of this frame.
    ///
    /// Determines whether it is a boundary for gradient drawing.
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
            items: Arc::new(LazyHash::new(vec![])),
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

    /// Sets the frame's hardness builder-style.
    pub fn with_kind(mut self, kind: FrameKind) -> Self {
        self.kind = kind;
        self
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

    /// Add multiple items at a position in the foreground.
    ///
    /// The first item in the iterator will be the one that is most in the
    /// background.
    pub fn push_multiple<I>(&mut self, items: I)
    where
        I: IntoIterator<Item = (Point, FrameItem)>,
    {
        Arc::make_mut(&mut self.items).extend(items);
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
        // Skip work if there's nothing to do.
        if frame.items.is_empty() {
            return;
        }

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
                    sink.splice(range, items.into_inner());
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
                sink.splice(
                    range,
                    items.into_inner().into_iter().map(|(p, e)| (p + pos, e)),
                );
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
            self.items = Arc::new(LazyHash::new(vec![]));
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
            for (point, _) in Arc::make_mut(&mut self.items).iter_mut() {
                *point += offset;
            }
        }
    }

    /// Apply late-stage properties from the style chain to this frame. This
    /// includes:
    /// - `HideElem::hidden`
    /// - `LinkElem::dests`
    ///
    /// This must be called on all frames produced by elements
    /// that manually handle styles (because their children can have varying
    /// styles). This currently includes flow, par, and equation.
    ///
    /// Other elements don't manually need to handle it because their parents
    /// that result from realization will take care of it and the styles can
    /// only apply to them as a whole, not part of it (because they don't manage
    /// styles).
    pub fn post_processed(mut self, styles: StyleChain) -> Self {
        self.post_process(styles);
        self
    }

    /// Post process in place.
    pub fn post_process(&mut self, styles: StyleChain) {
        if !self.is_empty() {
            self.post_process_raw(
                LinkElem::dests_in(styles),
                HideElem::hidden_in(styles),
            );
        }
    }

    /// Apply raw late-stage properties from the raw data.
    pub fn post_process_raw(&mut self, dests: SmallVec<[Destination; 1]>, hide: bool) {
        if !self.is_empty() {
            let size = self.size;
            self.push_multiple(
                dests
                    .into_iter()
                    .map(|dest| (Point::zero(), FrameItem::Link(dest, size))),
            );
            if hide {
                self.hide();
            }
        }
    }

    /// Hide all content in the frame, but keep metadata.
    pub fn hide(&mut self) {
        Arc::make_mut(&mut self.items).retain_mut(|(_, item)| match item {
            FrameItem::Group(group) => {
                group.frame.hide();
                !group.frame.is_empty()
            }
            FrameItem::Tag(_) => true,
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
        stroke: &Sides<Option<FixedStroke>>,
        outset: &Sides<Rel<Abs>>,
        radius: &Corners<Rel<Abs>>,
        span: Span,
    ) {
        let outset = outset.relative_to(self.size());
        let size = self.size() + outset.sum_by_axis();
        let pos = Point::new(-outset.left, -outset.top);
        self.prepend_multiple(
            styled_rect(size, radius, fill, stroke)
                .into_iter()
                .map(|x| (pos, FrameItem::Shape(x, span))),
        );
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

    /// Add a label to the frame.
    pub fn label(&mut self, label: Label) {
        self.group(|g| g.label = Some(label));
    }

    /// Set a parent for the frame. As a result, all elements in the frame
    /// become logically ordered immediately after the given location.
    pub fn set_parent(&mut self, parent: Location) {
        if !self.is_empty() {
            self.group(|g| g.parent = Some(parent));
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
    ///
    /// This is used for pages, blocks, and boxes.
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
    /// An internal or external link to a destination.
    Link(Destination, Size),
    /// An introspectable element that produced something within this frame
    /// alongside its key.
    Tag(Tag),
}

impl Debug for FrameItem {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::Group(group) => group.fmt(f),
            Self::Text(text) => write!(f, "{text:?}"),
            Self::Shape(shape, _) => write!(f, "{shape:?}"),
            Self::Image(image, _, _) => write!(f, "{image:?}"),
            Self::Link(dest, _) => write!(f, "Link({dest:?})"),
            Self::Tag(tag) => write!(f, "{tag:?}"),
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
    /// The group's label.
    pub label: Option<Label>,
    /// The group's logical parent. All elements in this group are logically
    /// ordered immediately after the parent's start location.
    pub parent: Option<Location>,
}

impl GroupItem {
    /// Create a new group with default settings.
    pub fn new(frame: Frame) -> Self {
        Self {
            frame,
            transform: Transform::identity(),
            clip_path: None,
            label: None,
            parent: None,
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
