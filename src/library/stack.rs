use std::fmt::{self, Debug, Formatter};

use super::prelude::*;
use super::Spacing;

/// `stack`: Stack children along an axis.
pub fn stack(_: &mut EvalContext, args: &mut Args) -> TypResult<Value> {
    enum Child {
        Spacing(Spacing),
        Any(Template),
    }

    castable! {
        Child,
        Expected: "linear, fractional or template",
        Value::Length(v) => Self::Spacing(Spacing::Linear(v.into())),
        Value::Relative(v) => Self::Spacing(Spacing::Linear(v.into())),
        Value::Linear(v) => Self::Spacing(Spacing::Linear(v)),
        Value::Fractional(v) => Self::Spacing(Spacing::Fractional(v)),
        Value::Template(v) => Self::Any(v),
    }

    let dir = args.named("dir")?.unwrap_or(Dir::TTB);
    let spacing = args.named("spacing")?;
    let list: Vec<Child> = args.all().collect();

    Ok(Value::Template(Template::from_block(move |style| {
        let mut children = vec![];
        let mut delayed = None;

        // Build the list of stack children.
        for child in &list {
            match child {
                Child::Spacing(v) => {
                    children.push(StackChild::Spacing(*v));
                    delayed = None;
                }
                Child::Any(template) => {
                    if let Some(v) = delayed {
                        children.push(StackChild::Spacing(v));
                    }

                    let node = template.to_flow(style).pack();
                    children.push(StackChild::Node(node));
                    delayed = spacing;
                }
            }
        }

        StackNode { dir, children }
    })))
}

/// A node that stacks its children.
#[derive(Debug, Hash)]
pub struct StackNode {
    /// The stacking direction.
    pub dir: Dir,
    /// The children to be stacked.
    pub children: Vec<StackChild>,
}

impl Layout for StackNode {
    fn layout(
        &self,
        ctx: &mut LayoutContext,
        regions: &Regions,
    ) -> Vec<Constrained<Rc<Frame>>> {
        StackLayouter::new(self, regions.clone()).layout(ctx)
    }
}

/// A child of a stack node.
#[derive(Hash)]
pub enum StackChild {
    /// Spacing between other nodes.
    Spacing(Spacing),
    /// Any block node and how to align it in the stack.
    Node(PackedNode),
}

impl Debug for StackChild {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::Spacing(v) => write!(f, "Spacing({:?})", v),
            Self::Node(node) => node.fmt(f),
        }
    }
}

/// Performs stack layout.
struct StackLayouter<'a> {
    /// The stack node to layout.
    stack: &'a StackNode,
    /// The axis of the block direction.
    axis: SpecAxis,
    /// Whether the stack should expand to fill the region.
    expand: Spec<bool>,
    /// The region to layout into.
    regions: Regions,
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
    Frame(Rc<Frame>),
}

impl<'a> StackLayouter<'a> {
    /// Create a new stack layouter.
    fn new(stack: &'a StackNode, mut regions: Regions) -> Self {
        // Disable expansion along the block axis for children.
        let axis = stack.dir.axis();
        let expand = regions.expand;
        regions.expand.set(axis, false);

        Self {
            stack,
            axis,
            expand,
            full: regions.current,
            regions,
            used: Gen::zero(),
            fr: Fractional::zero(),
            items: vec![],
            finished: vec![],
        }
    }

    /// Layout all children.
    fn layout(mut self, ctx: &mut LayoutContext) -> Vec<Constrained<Rc<Frame>>> {
        for child in &self.stack.children {
            match *child {
                StackChild::Spacing(Spacing::Linear(v)) => {
                    self.layout_absolute(v);
                }
                StackChild::Spacing(Spacing::Fractional(v)) => {
                    self.items.push(StackItem::Fractional(v));
                    self.fr += v;
                }
                StackChild::Node(ref node) => {
                    self.layout_node(ctx, node);
                }
            }
        }

        self.finish_region();
        self.finished
    }

    /// Layout absolute spacing.
    fn layout_absolute(&mut self, amount: Linear) {
        // Resolve the linear, limiting it to the remaining available space.
        let remaining = self.regions.current.get_mut(self.axis);
        let resolved = amount.resolve(self.full.get(self.axis));
        let limited = resolved.min(*remaining);
        *remaining -= limited;
        self.used.block += limited;
        self.items.push(StackItem::Absolute(resolved));
    }

    /// Layout a node.
    fn layout_node(&mut self, ctx: &mut LayoutContext, node: &PackedNode) {
        let frames = node.layout(ctx, &self.regions);
        let len = frames.len();
        for (i, frame) in frames.into_iter().enumerate() {
            // Grow our size, shrink the region and save the frame for later.
            let size = frame.item.size.to_gen(self.axis);
            self.used.block += size.block;
            self.used.inline.set_max(size.inline);
            *self.regions.current.get_mut(self.axis) -= size.block;
            self.items.push(StackItem::Frame(frame.item));

            if i + 1 < len {
                self.finish_region();
            }
        }
    }

    /// Finish the frame for one region.
    fn finish_region(&mut self) {
        // Determine the size of the stack in this region dependening on whether
        // the region expands.
        let used = self.used.to_size(self.axis);
        let mut size = Size::new(
            if self.expand.x { self.full.w } else { used.w },
            if self.expand.y { self.full.h } else { used.h },
        );

        // Expand fully if there are fr spacings.
        let full = self.full.get(self.axis);
        if !self.fr.is_zero() && full.is_finite() {
            size.set(self.axis, full);
        }

        let mut output = Frame::new(size, size.h);
        let mut before = Length::zero();

        // Place all frames.
        for item in self.items.drain(..) {
            match item {
                StackItem::Absolute(v) => before += v,
                StackItem::Fractional(v) => {
                    let ratio = v / self.fr;
                    let remaining = self.full.get(self.axis) - self.used.block;
                    if remaining.is_finite() && ratio.is_finite() {
                        before += ratio * remaining;
                    }
                }
                StackItem::Frame(frame) => {
                    let parent = size.get(self.axis);
                    let child = frame.size.get(self.axis);
                    let block = if self.stack.dir.is_positive() {
                        before
                    } else {
                        parent - (before + child)
                    };

                    before += child;

                    let pos = Gen::new(Length::zero(), block).to_point(self.axis);
                    output.push_frame(pos, frame);
                }
            }
        }

        // Generate tight constraints for now.
        let mut cts = Constraints::new(self.expand);
        cts.exact = self.full.to_spec().map(Some);
        cts.base = self.regions.base.to_spec().map(Some);

        // Advance to the next region.
        self.regions.next();
        self.full = self.regions.current;
        self.used = Gen::zero();
        self.fr = Fractional::zero();
        self.finished.push(output.constrain(cts));
    }
}
