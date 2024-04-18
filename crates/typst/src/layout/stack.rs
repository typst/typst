use std::fmt::{self, Debug, Formatter};
use typst_syntax::Span;

use crate::diag::{bail, SourceResult};
use crate::engine::Engine;
use crate::foundations::{cast, elem, Content, Packed, Resolve, StyleChain, StyledElem};
use crate::layout::{
    Abs, AlignElem, Axes, Axis, Dir, FixedAlignment, Fr, Fragment, Frame, HElem,
    LayoutMultiple, Point, Regions, Size, Spacing, VElem,
};
use crate::util::{Get, Numeric};

/// Arranges content and spacing horizontally or vertically.
///
/// The stack places a list of items along an axis, with optional spacing
/// between each item.
///
/// # Example
/// ```example
/// #stack(
///   dir: ttb,
///   rect(width: 40pt),
///   rect(width: 120pt),
///   rect(width: 90pt),
/// )
/// ```
#[elem(LayoutMultiple)]
pub struct StackElem {
    /// The direction along which the items are stacked. Possible values are:
    ///
    /// - `{ltr}`: Left to right.
    /// - `{rtl}`: Right to left.
    /// - `{ttb}`: Top to bottom.
    /// - `{btt}`: Bottom to top.
    ///
    /// You can use the `start` and `end` methods to obtain the initial and
    /// final points (respectively) of a direction, as `alignment`. You can also
    /// use the `axis` method to determine whether a direction is
    /// `{"horizontal"}` or `{"vertical"}`. The `inv` method returns a
    /// direction's inverse direction.
    ///
    /// For example, `{ttb.start()}` is `top`, `{ttb.end()}` is `bottom`,
    /// `{ttb.axis()}` is `{"vertical"}` and `{ttb.inv()}` is equal to `btt`.
    #[default(Dir::TTB)]
    pub dir: Dir,

    /// Spacing to insert between items where no explicit spacing was provided.
    pub spacing: Option<Spacing>,

    /// The children to stack along the axis.
    #[variadic]
    pub children: Vec<StackChild>,
}

impl LayoutMultiple for Packed<StackElem> {
    #[typst_macros::time(name = "stack", span = self.span())]
    fn layout(
        &self,
        engine: &mut Engine,
        styles: StyleChain,
        regions: Regions,
    ) -> SourceResult<Fragment> {
        let mut layouter =
            StackLayouter::new(self.span(), self.dir(styles), regions, styles);
        let axis = layouter.dir.axis();

        // Spacing to insert before the next block.
        let spacing = self.spacing(styles);
        let mut deferred = None;

        for child in self.children() {
            match child {
                StackChild::Spacing(kind) => {
                    layouter.layout_spacing(*kind);
                    deferred = None;
                }
                StackChild::Block(block) => {
                    // Transparently handle `h`.
                    if let (Axis::X, Some(h)) = (axis, block.to_packed::<HElem>()) {
                        layouter.layout_spacing(*h.amount());
                        deferred = None;
                        continue;
                    }

                    // Transparently handle `v`.
                    if let (Axis::Y, Some(v)) = (axis, block.to_packed::<VElem>()) {
                        layouter.layout_spacing(*v.amount());
                        deferred = None;
                        continue;
                    }

                    if let Some(kind) = deferred {
                        layouter.layout_spacing(kind);
                    }

                    layouter.layout_block(engine, block, styles)?;
                    deferred = spacing;
                }
            }
        }

        layouter.finish()
    }
}

/// A child of a stack element.
#[derive(Clone, PartialEq, Hash)]
pub enum StackChild {
    /// Spacing between other children.
    Spacing(Spacing),
    /// Arbitrary block-level content.
    Block(Content),
}

impl Debug for StackChild {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::Spacing(kind) => kind.fmt(f),
            Self::Block(block) => block.fmt(f),
        }
    }
}

cast! {
    StackChild,
    self => match self {
        Self::Spacing(spacing) => spacing.into_value(),
        Self::Block(content) => content.into_value(),
    },
    v: Spacing => Self::Spacing(v),
    v: Content => Self::Block(v),
}

/// Performs stack layout.
struct StackLayouter<'a> {
    /// The span to raise errors at during layout.
    span: Span,
    /// The stacking direction.
    dir: Dir,
    /// The axis of the stacking direction.
    axis: Axis,
    /// The regions to layout children into.
    regions: Regions<'a>,
    /// The inherited styles.
    styles: StyleChain<'a>,
    /// Whether the stack itself should expand to fill the region.
    expand: Axes<bool>,
    /// The initial size of the current region before we started subtracting.
    initial: Size,
    /// The generic size used by the frames for the current region.
    used: Gen<Abs>,
    /// The sum of fractions in the current region.
    fr: Fr,
    /// Already layouted items whose exact positions are not yet known due to
    /// fractional spacing.
    items: Vec<StackItem>,
    /// Finished frames for previous regions.
    finished: Vec<Frame>,
}

/// A prepared item in a stack layout.
enum StackItem {
    /// Absolute spacing between other items.
    Absolute(Abs),
    /// Fractional spacing between other items.
    Fractional(Fr),
    /// A frame for a layouted block.
    Frame(Frame, Axes<FixedAlignment>),
}

impl<'a> StackLayouter<'a> {
    /// Create a new stack layouter.
    fn new(
        span: Span,
        dir: Dir,
        mut regions: Regions<'a>,
        styles: StyleChain<'a>,
    ) -> Self {
        let axis = dir.axis();
        let expand = regions.expand;

        // Disable expansion along the block axis for children.
        regions.expand.set(axis, false);

        Self {
            span,
            dir,
            axis,
            regions,
            styles,
            expand,
            initial: regions.size,
            used: Gen::zero(),
            fr: Fr::zero(),
            items: vec![],
            finished: vec![],
        }
    }

    /// Add spacing along the spacing direction.
    fn layout_spacing(&mut self, spacing: Spacing) {
        match spacing {
            Spacing::Rel(v) => {
                // Resolve the spacing and limit it to the remaining space.
                let resolved = v
                    .resolve(self.styles)
                    .relative_to(self.regions.base().get(self.axis));
                let remaining = self.regions.size.get_mut(self.axis);
                let limited = resolved.min(*remaining);
                if self.dir.axis() == Axis::Y {
                    *remaining -= limited;
                }
                self.used.main += limited;
                self.items.push(StackItem::Absolute(resolved));
            }
            Spacing::Fr(v) => {
                self.fr += v;
                self.items.push(StackItem::Fractional(v));
            }
        }
    }

    /// Layout an arbitrary block.
    fn layout_block(
        &mut self,
        engine: &mut Engine,
        block: &Content,
        styles: StyleChain,
    ) -> SourceResult<()> {
        if self.regions.is_full() {
            self.finish_region()?;
        }

        // Block-axis alignment of the `AlignElement` is respected by stacks.
        let align = if let Some(align) = block.to_packed::<AlignElem>() {
            align.alignment(styles)
        } else if let Some(styled) = block.to_packed::<StyledElem>() {
            AlignElem::alignment_in(styles.chain(&styled.styles))
        } else {
            AlignElem::alignment_in(styles)
        }
        .resolve(styles);

        let fragment = block.layout(engine, styles, self.regions)?;
        let len = fragment.len();
        for (i, frame) in fragment.into_iter().enumerate() {
            // Grow our size, shrink the region and save the frame for later.
            let size = frame.size();
            if self.dir.axis() == Axis::Y {
                self.regions.size.y -= size.y;
            }

            let gen = match self.axis {
                Axis::X => Gen::new(size.y, size.x),
                Axis::Y => Gen::new(size.x, size.y),
            };

            self.used.main += gen.main;
            self.used.cross.set_max(gen.cross);

            self.items.push(StackItem::Frame(frame, align));

            if i + 1 < len {
                self.finish_region()?;
            }
        }

        Ok(())
    }

    /// Advance to the next region.
    fn finish_region(&mut self) -> SourceResult<()> {
        // Determine the size of the stack in this region depending on whether
        // the region expands.
        let mut size = self
            .expand
            .select(self.initial, self.used.into_axes(self.axis))
            .min(self.initial);

        // Expand fully if there are fr spacings.
        let full = self.initial.get(self.axis);
        let remaining = full - self.used.main;
        if self.fr.get() > 0.0 && full.is_finite() {
            self.used.main = full;
            size.set(self.axis, full);
        }

        if !size.is_finite() {
            bail!(self.span, "stack spacing is infinite");
        }

        let mut output = Frame::hard(size);
        let mut cursor = Abs::zero();
        let mut ruler: FixedAlignment = self.dir.start().into();

        // Place all frames.
        for item in self.items.drain(..) {
            match item {
                StackItem::Absolute(v) => cursor += v,
                StackItem::Fractional(v) => cursor += v.share(self.fr, remaining),
                StackItem::Frame(frame, align) => {
                    if self.dir.is_positive() {
                        ruler = ruler.max(align.get(self.axis));
                    } else {
                        ruler = ruler.min(align.get(self.axis));
                    }

                    // Align along the main axis.
                    let parent = size.get(self.axis);
                    let child = frame.size().get(self.axis);
                    let main = ruler.position(parent - self.used.main)
                        + if self.dir.is_positive() {
                            cursor
                        } else {
                            self.used.main - child - cursor
                        };

                    // Align along the cross axis.
                    let other = self.axis.other();
                    let cross = align
                        .get(other)
                        .position(size.get(other) - frame.size().get(other));

                    let pos = Gen::new(cross, main).to_point(self.axis);
                    cursor += child;
                    output.push_frame(pos, frame);
                }
            }
        }

        // Advance to the next region.
        self.regions.next();
        self.initial = self.regions.size;
        self.used = Gen::zero();
        self.fr = Fr::zero();
        self.finished.push(output);

        Ok(())
    }

    /// Finish layouting and return the resulting frames.
    fn finish(mut self) -> SourceResult<Fragment> {
        self.finish_region()?;
        Ok(Fragment::frames(self.finished))
    }
}

/// A container with a main and cross component.
#[derive(Default, Copy, Clone, Eq, PartialEq, Hash)]
struct Gen<T> {
    /// The main component.
    pub cross: T,
    /// The cross component.
    pub main: T,
}

impl<T> Gen<T> {
    /// Create a new instance from the two components.
    const fn new(cross: T, main: T) -> Self {
        Self { cross, main }
    }

    /// Convert to the specific representation, given the current main axis.
    fn into_axes(self, main: Axis) -> Axes<T> {
        match main {
            Axis::X => Axes::new(self.main, self.cross),
            Axis::Y => Axes::new(self.cross, self.main),
        }
    }
}

impl Gen<Abs> {
    /// The zero value.
    fn zero() -> Self {
        Self { cross: Abs::zero(), main: Abs::zero() }
    }

    /// Convert to a point.
    fn to_point(self, main: Axis) -> Point {
        self.into_axes(main).to_point()
    }
}
