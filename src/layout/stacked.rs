use smallvec::smallvec;
use super::*;

#[derive(Debug, Clone)]
pub struct StackLayouter {
    ctx: StackContext,
    layouts: MultiLayout,

    space: usize,
    hard: bool,
    start: Size2D,
    actions: LayoutActionList,
    combined_dimensions: Size2D,

    sub: Subspace,
}

/// The context for stack layouting.
///
/// See [`LayoutContext`] for details about the fields.
#[derive(Debug, Clone)]
pub struct StackContext {
    pub spaces: LayoutSpaces,
    pub axes: LayoutAxes,
    pub shrink_to_fit: bool,
}

#[derive(Debug, Clone)]
struct Subspace {
    usable: Size2D,
    anchor: Size2D,
    factor: i32,
    dimensions: Size2D,
}

impl Subspace {
    fn new(usable: Size2D, axes: LayoutAxes) -> Subspace {
        Subspace {
            usable,
            anchor: axes.anchor(usable),
            factor: axes.secondary.axis.factor(),
            dimensions: Size2D::zero(),
        }
    }
}

impl StackLayouter {
    /// Create a new stack layouter.
    pub fn new(ctx: StackContext) -> StackLayouter {
        let space = ctx.spaces[0];
        let usable = ctx.axes.generalize(space.usable());
        let axes = ctx.axes;

        StackLayouter {
            ctx,
            layouts: MultiLayout::new(),

            space: 0,
            hard: true,
            start: space.start(),
            actions: LayoutActionList::new(),
            combined_dimensions: Size2D::zero(),

            sub: Subspace::new(usable, axes),
        }
    }

    pub fn add(&mut self, layout: Layout) -> LayoutResult<()> {
        let size = self.ctx.axes.generalize(layout.dimensions);
        let mut new_dimensions = merge_sizes(self.sub.dimensions, size);

        while !self.sub.usable.fits(new_dimensions) {
            if self.space_is_empty() {
                Err(LayoutError::NotEnoughSpace("cannot fit box into stack"))?;
            }

            self.finish_layout(true);
            new_dimensions = merge_sizes(self.sub.dimensions, size);
        }

        let offset = self.sub.dimensions.y;
        let anchor = self.ctx.axes.anchor(size);

        let pos = self.ctx.axes.specialize(
            self.start
                + (self.sub.anchor - anchor)
                + Size2D::with_y(self.combined_dimensions.y + self.sub.factor * offset)
        );

        self.actions.add_layout(pos, layout);
        self.sub.dimensions = new_dimensions;

        Ok(())
    }

    pub fn add_multiple(&mut self, layouts: MultiLayout) -> LayoutResult<()> {
        for layout in layouts {
            self.add(layout)?;
        }
        Ok(())
    }

    pub fn add_space(&mut self, space: Size) {
        if self.sub.dimensions.y + space > self.sub.usable.y {
            self.finish_layout(false);
        } else {
            self.sub.dimensions.y += space;
        }
    }

    pub fn set_axes(&mut self, axes: LayoutAxes) {
        if axes != self.ctx.axes {
            self.finish_subspace();
            self.sub = Subspace::new(self.remaining_subspace(), axes);
            self.ctx.axes = axes;
        }
    }

    pub fn set_spaces(&mut self, spaces: LayoutSpaces, replace_empty: bool) {
        if replace_empty && self.space_is_empty() {
            let space = spaces[0];
            let usable = self.ctx.axes.generalize(space.usable());

            self.ctx.spaces = spaces;
            self.space = 0;
            self.start = space.start();
            self.sub = Subspace::new(usable, self.ctx.axes);
        } else {
            self.ctx.spaces.truncate(self.space + 1);
            self.ctx.spaces.extend(spaces);
        }
    }

    pub fn primary_usable(&self) -> Size {
        self.sub.usable.x
    }

    pub fn remaining(&self) -> LayoutSpaces {
        let mut spaces = smallvec![LayoutSpace {
            dimensions: self.ctx.axes.specialize(self.remaining_subspace()),
            padding: SizeBox::zero(),
        }];

        for space in &self.ctx.spaces[self.next_space()..] {
            spaces.push(space.usable_space());
        }

        spaces
    }

    pub fn space_is_empty(&self) -> bool {
        self.combined_dimensions == Size2D::zero()
        && self.sub.dimensions == Size2D::zero()
    }

    pub fn in_last_space(&self) -> bool {
        self.space == self.ctx.spaces.len() - 1
    }

    pub fn finish(mut self) -> MultiLayout {
        if self.hard || !self.space_is_empty() {
            self.finish_layout(false);
        }
        self.layouts
    }

    pub fn finish_layout(&mut self, hard: bool) {
        self.finish_subspace();

        let space = self.ctx.spaces[self.space];
        let actions = std::mem::replace(&mut self.actions, LayoutActionList::new());

        self.layouts.add(Layout {
            dimensions: match self.ctx.shrink_to_fit {
                true => self.combined_dimensions.padded(space.padding),
                false => space.dimensions,
            },
            actions: actions.into_vec(),
            debug_render: true,
        });

        self.space = self.next_space();
        let space = self.ctx.spaces[self.space];
        let usable = self.ctx.axes.generalize(space.usable());

        self.hard = hard;
        self.start = space.start();
        self.combined_dimensions = Size2D::zero();
        self.sub = Subspace::new(usable, self.ctx.axes);
    }

    fn finish_subspace(&mut self) {
        let sub_dim = self.ctx.axes.specialize(self.sub.dimensions);
        self.combined_dimensions = merge_sizes(self.combined_dimensions, sub_dim);
    }

    fn remaining_subspace(&self) -> Size2D {
        Size2D::new(self.sub.usable.x, self.sub.usable.y - self.sub.dimensions.y)
    }

    fn next_space(&self) -> usize {
        (self.space + 1).min(self.ctx.spaces.len() - 1)
    }
}

fn merge_sizes(a: Size2D, b: Size2D) -> Size2D {
    Size2D {
        x: crate::size::max(a.x, b.x),
        y: a.y + b.y
    }
}
