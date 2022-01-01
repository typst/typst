//! Side-by-side layout of nodes along an axis.

use super::prelude::*;
use super::{AlignNode, SpacingKind, SpacingNode};

/// `stack`: Stack children along an axis.
pub fn stack(_: &mut EvalContext, args: &mut Args) -> TypResult<Value> {
    Ok(Value::block(StackNode {
        dir: args.named("dir")?.unwrap_or(Dir::TTB),
        spacing: args.named("spacing")?,
        children: args.all().collect(),
    }))
}

/// A node that stacks its children.
#[derive(Debug, Hash)]
pub struct StackNode {
    /// The stacking direction.
    pub dir: Dir,
    /// The spacing between non-spacing children.
    pub spacing: Option<SpacingKind>,
    /// The children to be stacked.
    pub children: Vec<StackChild>,
}

impl Layout for StackNode {
    fn layout(
        &self,
        ctx: &mut LayoutContext,
        regions: &Regions,
        styles: StyleChain,
    ) -> Vec<Constrained<Rc<Frame>>> {
        StackLayouter::new(self, regions.clone(), styles).layout(ctx)
    }
}

/// A child of a stack node.
#[derive(Hash)]
pub enum StackChild {
    /// Spacing between other nodes.
    Spacing(SpacingNode),
    /// An arbitrary node.
    Node(PackedNode),
}

impl From<SpacingKind> for StackChild {
    fn from(kind: SpacingKind) -> Self {
        Self::Spacing(SpacingNode { kind, styles: StyleMap::new() })
    }
}

impl Debug for StackChild {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::Spacing(node) => node.fmt(f),
            Self::Node(node) => node.fmt(f),
        }
    }
}

castable! {
    StackChild,
    Expected: "linear, fractional or template",
    Value::Length(v) => SpacingKind::Linear(v.into()).into(),
    Value::Relative(v) => SpacingKind::Linear(v.into()).into(),
    Value::Linear(v) => SpacingKind::Linear(v).into(),
    Value::Fractional(v) => SpacingKind::Fractional(v).into(),
    Value::Node(v) => Self::Node(v.into_block()),
}

/// Performs stack layout.
struct StackLayouter<'a> {
    /// The flow node to layout.
    children: &'a [StackChild],
    /// The stacking direction.
    dir: Dir,
    /// The axis of the stacking direction.
    axis: SpecAxis,
    /// The spacing between non-spacing children.
    spacing: Option<SpacingKind>,
    /// The regions to layout children into.
    regions: Regions,
    /// The inherited styles.
    styles: StyleChain<'a>,
    /// Whether the stack should expand to fill the region.
    expand: Spec<bool>,
    /// The full size of `regions.current` that was available before we started
    /// subtracting.
    full: Size,
    /// The generic size used by the frames for the current region.
    used: Gen<Length>,
    /// The sum of fractional ratios in the current region.
    fr: Fractional,
    /// Spacing and layouted nodes.
    items: Vec<StackItem>,
    /// Finished frames for previous regions.
    finished: Vec<Constrained<Rc<Frame>>>,
}

/// A prepared item in a stack layout.
enum StackItem {
    /// Absolute spacing between other items.
    Absolute(Length),
    /// Fractional spacing between other items.
    Fractional(Fractional),
    /// A layouted child node.
    Frame(Rc<Frame>, Align),
}

impl<'a> StackLayouter<'a> {
    /// Create a new stack layouter.
    fn new(stack: &'a StackNode, mut regions: Regions, styles: StyleChain<'a>) -> Self {
        let dir = stack.dir;
        let axis = dir.axis();
        let expand = regions.expand;
        let full = regions.current;

        // Disable expansion along the block axis for children.
        regions.expand.set(axis, false);

        Self {
            children: &stack.children,
            dir,
            axis,
            spacing: stack.spacing,
            regions,
            styles,
            expand,
            full,
            used: Gen::zero(),
            fr: Fractional::zero(),
            items: vec![],
            finished: vec![],
        }
    }

    /// Layout all children.
    fn layout(mut self, ctx: &mut LayoutContext) -> Vec<Constrained<Rc<Frame>>> {
        // Spacing to insert before the next node.
        let mut deferred = None;

        for child in self.children {
            match child {
                StackChild::Spacing(node) => {
                    self.layout_spacing(node.kind);
                    deferred = None;
                }
                StackChild::Node(node) => {
                    if let Some(kind) = deferred {
                        self.layout_spacing(kind);
                    }

                    if self.regions.is_full() {
                        self.finish_region();
                    }

                    self.layout_node(ctx, node);
                    deferred = self.spacing;
                }
            }
        }

        self.finish_region();
        self.finished
    }

    /// Layout spacing.
    fn layout_spacing(&mut self, spacing: SpacingKind) {
        match spacing {
            SpacingKind::Linear(v) => self.layout_absolute(v),
            SpacingKind::Fractional(v) => {
                self.items.push(StackItem::Fractional(v));
                self.fr += v;
            }
        }
    }

    /// Layout absolute spacing.
    fn layout_absolute(&mut self, amount: Linear) {
        // Resolve the linear, limiting it to the remaining available space.
        let remaining = self.regions.current.get_mut(self.axis);
        let resolved = amount.resolve(self.regions.base.get(self.axis));
        let limited = resolved.min(*remaining);
        *remaining -= limited;
        self.used.main += limited;
        self.items.push(StackItem::Absolute(resolved));
    }

    /// Layout a node.
    fn layout_node(&mut self, ctx: &mut LayoutContext, node: &PackedNode) {
        // Align nodes' block-axis alignment is respected by the stack node.
        let align = node
            .downcast::<AlignNode>()
            .and_then(|node| node.aligns.get(self.axis))
            .unwrap_or(self.dir.start().into());

        let frames = node.layout(ctx, &self.regions, self.styles);
        let len = frames.len();
        for (i, frame) in frames.into_iter().enumerate() {
            // Grow our size, shrink the region and save the frame for later.
            let size = frame.item.size.to_gen(self.axis);
            self.used.main += size.main;
            self.used.cross.set_max(size.cross);
            *self.regions.current.get_mut(self.axis) -= size.main;
            self.items.push(StackItem::Frame(frame.item, align));

            if i + 1 < len {
                self.finish_region();
            }
        }
    }

    /// Finish the frame for one region.
    fn finish_region(&mut self) {
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
                StackItem::Absolute(v) => {
                    cursor += v;
                }
                StackItem::Fractional(v) => {
                    cursor += v.resolve(self.fr, remaining);
                }
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

        // Generate tight constraints for now.
        let mut cts = Constraints::new(self.expand);
        cts.exact = self.full.map(Some);
        cts.base = self.regions.base.map(Some);

        // Advance to the next region.
        self.regions.next();
        self.full = self.regions.current;
        self.used = Gen::zero();
        self.fr = Fractional::zero();
        self.finished.push(output.constrain(cts));
    }
}
