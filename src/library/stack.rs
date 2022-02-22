//! Side-by-side layout of nodes along an axis.

use super::prelude::*;
use super::{AlignNode, SpacingKind};

/// Arrange nodes and spacing along an axis.
#[derive(Debug, Hash)]
pub struct StackNode {
    /// The stacking direction.
    pub dir: Dir,
    /// The spacing between non-spacing children.
    pub spacing: Option<SpacingKind>,
    /// The children to be stacked.
    pub children: Vec<StackChild>,
}

#[class]
impl StackNode {
    fn construct(_: &mut Context, args: &mut Args) -> TypResult<Template> {
        Ok(Template::block(Self {
            dir: args.named("dir")?.unwrap_or(Dir::TTB),
            spacing: args.named("spacing")?,
            children: args.all()?,
        }))
    }
}

impl Layout for StackNode {
    fn layout(
        &self,
        ctx: &mut Context,
        regions: &Regions,
        styles: StyleChain,
    ) -> TypResult<Vec<Arc<Frame>>> {
        let mut layouter = StackLayouter::new(self.dir, regions);

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

                    layouter.layout_node(ctx, node, styles)?;
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
    Spacing(SpacingKind),
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
    Expected: "linear, fractional or template",
    Value::Length(v) => Self::Spacing(SpacingKind::Linear(v.into())),
    Value::Relative(v) => Self::Spacing(SpacingKind::Linear(v.into())),
    Value::Linear(v) => Self::Spacing(SpacingKind::Linear(v)),
    Value::Fractional(v) => Self::Spacing(SpacingKind::Fractional(v)),
    Value::Template(v) => Self::Node(v.pack()),
}

/// Performs stack layout.
pub struct StackLayouter {
    /// The stacking direction.
    dir: Dir,
    /// The axis of the stacking direction.
    axis: SpecAxis,
    /// The regions to layout children into.
    regions: Regions,
    /// Whether the stack itself should expand to fill the region.
    expand: Spec<bool>,
    /// The full size of the current region that was available at the start.
    full: Size,
    /// The generic size used by the frames for the current region.
    used: Gen<Length>,
    /// The sum of fractional ratios in the current region.
    fr: Fractional,
    /// Already layouted items whose exact positions are not yet known due to
    /// fractional spacing.
    items: Vec<StackItem>,
    /// Finished frames for previous regions.
    finished: Vec<Arc<Frame>>,
}

/// A prepared item in a stack layout.
enum StackItem {
    /// Absolute spacing between other items.
    Absolute(Length),
    /// Fractional spacing between other items.
    Fractional(Fractional),
    /// A frame for a layouted child node.
    Frame(Arc<Frame>, Align),
}

impl StackLayouter {
    /// Create a new stack layouter.
    pub fn new(dir: Dir, regions: &Regions) -> Self {
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
            expand,
            full,
            used: Gen::zero(),
            fr: Fractional::zero(),
            items: vec![],
            finished: vec![],
        }
    }

    /// Add spacing along the spacing direction.
    pub fn layout_spacing(&mut self, spacing: SpacingKind) {
        match spacing {
            SpacingKind::Linear(v) => {
                // Resolve the linear and limit it to the remaining space.
                let resolved = v.resolve(self.regions.base.get(self.axis));
                let remaining = self.regions.first.get_mut(self.axis);
                let limited = resolved.min(*remaining);
                *remaining -= limited;
                self.used.main += limited;
                self.items.push(StackItem::Absolute(resolved));
            }
            SpacingKind::Fractional(v) => {
                self.fr += v;
                self.items.push(StackItem::Fractional(v));
            }
        }
    }

    /// Layout an arbitrary node.
    pub fn layout_node(
        &mut self,
        ctx: &mut Context,
        node: &LayoutNode,
        styles: StyleChain,
    ) -> TypResult<()> {
        if self.regions.is_full() {
            self.finish_region();
        }

        // Align nodes' block-axis alignment is respected by the stack node.
        let align = node
            .downcast::<AlignNode>()
            .and_then(|node| node.aligns.get(self.axis))
            .unwrap_or(self.dir.start().into());

        let frames = node.layout(ctx, &self.regions, styles)?;
        let len = frames.len();
        for (i, frame) in frames.into_iter().enumerate() {
            // Grow our size, shrink the region and save the frame for later.
            let size = frame.size.to_gen(self.axis);
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
                StackItem::Fractional(v) => cursor += v.resolve(self.fr, remaining),
                StackItem::Frame(frame, align) => {
                    if self.dir.is_positive() {
                        ruler = ruler.max(align);
                    } else {
                        ruler = ruler.min(align);
                    }

                    // Align along the block axis.
                    let parent = size.get(self.axis);
                    let child = frame.size.get(self.axis);
                    let block = ruler.resolve(parent - self.used.main)
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
        self.fr = Fractional::zero();
        self.finished.push(Arc::new(output));
    }

    /// Finish layouting and return the resulting frames.
    pub fn finish(mut self) -> Vec<Arc<Frame>> {
        self.finish_region();
        self.finished
    }
}
