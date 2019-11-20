use smallvec::smallvec;
use super::*;

#[derive(Debug, Clone)]
pub struct StackLayouter {
    ctx: StackContext,
    layouts: MultiLayout,

    space: usize,
    hard: bool,
    actions: LayoutActionList,
    combined_dimensions: Size2D, // <- specialized

    sub: Subspace,
}

#[derive(Debug, Clone)]
struct Subspace {
    origin: Size2D, // <- specialized
    usable: Size2D,
    anchor: Size2D, // <- generic
    factor: i32,
    dimensions: Size2D, // <- generic
    space: Option<Size>,
}

impl Subspace {
    fn new(origin: Size2D, usable: Size2D, axes: LayoutAxes) -> Subspace {
        Subspace {
            origin,
            usable: axes.generalize(usable),
            anchor: axes.anchor(usable),
            factor: axes.secondary.axis.factor(),
            dimensions: Size2D::zero(),
            space: None,
        }
    }
}

/// The context for stack layouting.
///
/// See [`LayoutContext`] for details about the fields.
#[derive(Debug, Clone)]
pub struct StackContext {
    pub spaces: LayoutSpaces,
    pub axes: LayoutAxes,
    pub expand: bool,
}

impl StackLayouter {
    /// Create a new stack layouter.
    pub fn new(ctx: StackContext) -> StackLayouter {
        let axes = ctx.axes;
        let space = ctx.spaces[0];

        StackLayouter {
            ctx,
            layouts: MultiLayout::new(),

            space: 0,
            hard: true,
            actions: LayoutActionList::new(),
            combined_dimensions: Size2D::zero(),

            sub: Subspace::new(space.start(), space.usable(), axes),
        }
    }

    pub fn add(&mut self, layout: Layout) -> LayoutResult<()> {
        self.layout_space();

        let size = self.ctx.axes.generalize(layout.dimensions);
        let mut new_dimensions = merge(self.sub.dimensions, size);

        while !self.sub.usable.fits(new_dimensions) {
            if self.space_is_empty() {
                Err(LayoutError::NotEnoughSpace("cannot fit box into stack"))?;
            }

            self.finish_space(true);
            new_dimensions = merge(self.sub.dimensions, size);
        }

        let offset = self.sub.dimensions.y;
        let anchor = self.ctx.axes.anchor(size);

        let pos = self.sub.origin + self.ctx.axes.specialize(
            (self.sub.anchor - anchor)
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

    pub fn add_space(&mut self, space: Size, soft: bool) {
        self.sub.space = Some(space);
        if !soft {
            self.layout_space();
        }
    }

    pub fn set_axes(&mut self, axes: LayoutAxes) {
        if axes != self.ctx.axes {
            self.finish_subspace(axes);
        }
    }

    pub fn set_spaces(&mut self, spaces: LayoutSpaces, replace_empty: bool) {
        if replace_empty && self.space_is_empty() {
            self.ctx.spaces = spaces;
            self.start_space(0, self.hard);
        } else {
            self.ctx.spaces.truncate(self.space + 1);
            self.ctx.spaces.extend(spaces);
        }
    }

    pub fn remaining(&self) -> LayoutSpaces {
        let mut spaces = smallvec![LayoutSpace {
            dimensions: self.remaining_subspace().1,
            padding: SizeBox::zero(),
        }];

        for space in &self.ctx.spaces[self.next_space()..] {
            spaces.push(space.usable_space());
        }

        spaces
    }

    pub fn primary_usable(&self) -> Size {
        self.sub.usable.x
    }

    pub fn space_is_empty(&self) -> bool {
        self.combined_dimensions == Size2D::zero()
            && self.sub.dimensions == Size2D::zero()
            && self.actions.is_empty()
    }

    pub fn space_is_last(&self) -> bool {
        self.space == self.ctx.spaces.len() - 1
    }

    pub fn finish(mut self) -> MultiLayout {
        if self.hard || !self.space_is_empty() {
            self.finish_space(false);
        }
        self.layouts
    }

    pub fn finish_space(&mut self, hard: bool) {
        self.finish_subspace(self.ctx.axes);

        let space = self.ctx.spaces[self.space];
        let actions = std::mem::replace(&mut self.actions, LayoutActionList::new());

        self.layouts.add(Layout {
            dimensions: match self.ctx.expand {
                true => self.combined_dimensions.padded(space.padding),
                false => space.dimensions,
            },
            actions: actions.into_vec(),
            debug_render: true,
        });

        self.start_space(self.next_space(), hard);
    }

    fn start_space(&mut self, space: usize, hard: bool) {
        self.space = space;
        let space = self.ctx.spaces[space];

        self.hard = hard;
        self.combined_dimensions = Size2D::zero();
        self.sub = Subspace::new(space.start(), space.usable(), self.ctx.axes);
    }

    fn next_space(&self) -> usize {
        (self.space + 1).min(self.ctx.spaces.len() - 1)
    }

    fn finish_subspace(&mut self, new_axes: LayoutAxes) {
        if self.ctx.axes.primary.needs_expansion() {
            self.sub.dimensions.x = self.sub.usable.x;
        }

        if self.ctx.axes.secondary.needs_expansion() {
            self.sub.dimensions.y = self.sub.usable.y;
        }

        let (new_origin, new_usable) = self.remaining_subspace();

        let origin = self.sub.origin;
        let dimensions = self.ctx.axes.specialize(self.sub.dimensions);
        let space = self.ctx.spaces[self.space];
        self.combined_dimensions.max_eq(origin - space.start() + dimensions);

        self.ctx.axes = new_axes;
        self.sub = Subspace::new(new_origin, new_usable, new_axes);
    }

    fn remaining_subspace(&self) -> (Size2D, Size2D) {
        let used = self.ctx.axes.specialize(self.sub.usable);
        let dimensions = self.ctx.axes.specialize(self.sub.dimensions);

        let new_usable = self.ctx.axes.specialize(Size2D {
            x: self.sub.usable.x,
            y: self.sub.usable.y - self.sub.dimensions.y,
        });

        let new_origin = self.sub.origin
            + Size2D::with_y(self.ctx.axes.specialize(self.sub.dimensions).y);

        (new_origin, new_usable)
    }

    fn layout_space(&mut self) {
        if let Some(space) = self.sub.space.take() {
            if self.sub.dimensions.y + space > self.sub.usable.y {
                self.finish_space(false);
            } else {
                self.sub.dimensions.y += space;
            }
        }
    }
}

fn merge(a: Size2D, b: Size2D) -> Size2D {
    Size2D {
        x: crate::size::max(a.x, b.x),
        y: a.y + b.y
    }
}
