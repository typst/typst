use super::{AlignNode, Spacing};
use crate::library::prelude::*;
use crate::library::text::ParNode;

/// Arrange nodes and spacing along an axis.
#[derive(Debug, Hash)]
pub struct StackNode {
    /// The stacking direction.
    pub dir: Dir,
    /// The spacing between non-spacing children.
    pub spacing: Option<Spacing>,
    /// The children to be stacked.
    pub children: Vec<StackChild>,
}

#[node]
impl StackNode {
    fn construct(_: &mut Vm, args: &mut Args) -> SourceResult<Content> {
        Ok(Content::block(Self {
            dir: args.named("dir")?.unwrap_or(Dir::TTB),
            spacing: args.named("spacing")?,
            children: args.all()?,
        }))
    }
}

impl Layout for StackNode {
    fn layout(
        &self,
        world: &dyn World,
        regions: &Regions,
        styles: StyleChain,
    ) -> SourceResult<Vec<Frame>> {
        let mut layouter = StackLayouter::new(self.dir, regions, styles);

        // Spacing to insert before the next node.
        let mut deferred = None;

        for child in &self.children {
            match child {
                StackChild::Spacing(kind) => {
                    layouter.layout_spacing(*kind);
                    deferred = None;
                }
                StackChild::Node(node) => {
                    if let Some(kind) = deferred {
                        layouter.layout_spacing(kind);
                    }

                    layouter.layout_node(world, node, styles)?;
                    deferred = self.spacing;
                }
            }
        }

        Ok(layouter.finish())
    }
}

/// A child of a stack node.
#[derive(Hash)]
pub enum StackChild {
    /// Spacing between other nodes.
    Spacing(Spacing),
    /// An arbitrary node.
    Node(LayoutNode),
}

impl Debug for StackChild {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::Spacing(kind) => kind.fmt(f),
            Self::Node(node) => node.fmt(f),
        }
    }
}

castable! {
    StackChild,
    Expected: "relative length, fraction, or content",
    Value::Length(v) => Self::Spacing(Spacing::Relative(v.into())),
    Value::Ratio(v) => Self::Spacing(Spacing::Relative(v.into())),
    Value::Relative(v) => Self::Spacing(Spacing::Relative(v)),
    Value::Fraction(v) => Self::Spacing(Spacing::Fractional(v)),
    Value::Content(v) => Self::Node(v.pack()),
}

/// Performs stack layout.
pub struct StackLayouter<'a> {
    /// The stacking direction.
    dir: Dir,
    /// The axis of the stacking direction.
    axis: SpecAxis,
    /// The regions to layout children into.
    regions: Regions,
    /// The inherited styles.
    styles: StyleChain<'a>,
    /// Whether the stack itself should expand to fill the region.
    expand: Spec<bool>,
    /// The full size of the current region that was available at the start.
    full: Size,
    /// The generic size used by the frames for the current region.
    used: Gen<Length>,
    /// The sum of fractions in the current region.
    fr: Fraction,
    /// Already layouted items whose exact positions are not yet known due to
    /// fractional spacing.
    items: Vec<StackItem>,
    /// Finished frames for previous regions.
    finished: Vec<Frame>,
}

/// A prepared item in a stack layout.
enum StackItem {
    /// Absolute spacing between other items.
    Absolute(Length),
    /// Fractional spacing between other items.
    Fractional(Fraction),
    /// A frame for a layouted child node.
    Frame(Frame, Align),
}

impl<'a> StackLayouter<'a> {
    /// Create a new stack layouter.
    pub fn new(dir: Dir, regions: &Regions, styles: StyleChain<'a>) -> Self {
        let axis = dir.axis();
        let expand = regions.expand;
        let full = regions.first;

        // Disable expansion along the block axis for children.
        let mut regions = regions.clone();
        regions.expand.set(axis, false);

        Self {
            dir,
            axis,
            regions,
            styles,
            expand,
            full,
            used: Gen::zero(),
            fr: Fraction::zero(),
            items: vec![],
            finished: vec![],
        }
    }

    /// Add spacing along the spacing direction.
    pub fn layout_spacing(&mut self, spacing: Spacing) {
        match spacing {
            Spacing::Relative(v) => {
                // Resolve the spacing and limit it to the remaining space.
                let resolved =
                    v.resolve(self.styles).relative_to(self.regions.base.get(self.axis));
                let remaining = self.regions.first.get_mut(self.axis);
                let limited = resolved.min(*remaining);
                *remaining -= limited;
                self.used.main += limited;
                self.items.push(StackItem::Absolute(resolved));
            }
            Spacing::Fractional(v) => {
                self.fr += v;
                self.items.push(StackItem::Fractional(v));
            }
        }
    }

    /// Layout an arbitrary node.
    pub fn layout_node(
        &mut self,
        world: &dyn World,
        node: &LayoutNode,
        styles: StyleChain,
    ) -> SourceResult<()> {
        if self.regions.is_full() {
            self.finish_region();
        }

        // Block-axis alignment of the `AlignNode` is respected
        // by the stack node.
        let align = node
            .downcast::<AlignNode>()
            .and_then(|node| node.aligns.get(self.axis))
            .map(|align| align.resolve(styles))
            .unwrap_or_else(|| {
                if let Some(Content::Styled(styled)) = node.downcast::<Content>() {
                    let map = &styled.1;
                    if map.contains(ParNode::ALIGN) {
                        return StyleChain::with_root(&styled.1).get(ParNode::ALIGN);
                    }
                }

                self.dir.start().into()
            });

        let frames = node.layout(world, &self.regions, styles)?;
        let len = frames.len();
        for (i, mut frame) in frames.into_iter().enumerate() {
            // Set the generic block role.
            frame.apply_role(Role::GenericBlock);

            // Grow our size, shrink the region and save the frame for later.
            let size = frame.size().to_gen(self.axis);
            self.used.main += size.main;
            self.used.cross.set_max(size.cross);
            *self.regions.first.get_mut(self.axis) -= size.main;
            self.items.push(StackItem::Frame(frame, align));

            if i + 1 < len {
                self.finish_region();
            }
        }

        Ok(())
    }

    /// Advance to the next region.
    pub fn finish_region(&mut self) {
        // Determine the size of the stack in this region dependening on whether
        // the region expands.
        let used = self.used.to_spec(self.axis);
        let mut size = self.expand.select(self.full, used);

        // Expand fully if there are fr spacings.
        let full = self.full.get(self.axis);
        let remaining = full - self.used.main;
        if self.fr.get() > 0.0 && full.is_finite() {
            self.used.main = full;
            size.set(self.axis, full);
        }

        let mut output = Frame::new(size);
        let mut cursor = Length::zero();
        let mut ruler: Align = self.dir.start().into();

        // Place all frames.
        for item in self.items.drain(..) {
            match item {
                StackItem::Absolute(v) => cursor += v,
                StackItem::Fractional(v) => cursor += v.share(self.fr, remaining),
                StackItem::Frame(frame, align) => {
                    if self.dir.is_positive() {
                        ruler = ruler.max(align);
                    } else {
                        ruler = ruler.min(align);
                    }

                    // Align along the block axis.
                    let parent = size.get(self.axis);
                    let child = frame.size().get(self.axis);
                    let block = ruler.position(parent - self.used.main)
                        + if self.dir.is_positive() {
                            cursor
                        } else {
                            self.used.main - child - cursor
                        };

                    let pos = Gen::new(Length::zero(), block).to_point(self.axis);
                    cursor += child;
                    output.push_frame(pos, frame);
                }
            }
        }

        // Advance to the next region.
        self.regions.next();
        self.full = self.regions.first;
        self.used = Gen::zero();
        self.fr = Fraction::zero();
        self.finished.push(output);
    }

    /// Finish layouting and return the resulting frames.
    pub fn finish(mut self) -> Vec<Frame> {
        self.finish_region();
        self.finished
    }
}
