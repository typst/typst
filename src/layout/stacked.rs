use smallvec::smallvec;
use super::*;

#[derive(Debug, Clone)]
pub struct StackLayouter {
    ctx: StackContext,
    layouts: MultiLayout,

    space: Space,
    sub: Subspace,
}

#[derive(Debug, Clone)]
struct Space {
    index: usize,
    hard: bool,
    actions: LayoutActionList,
    combined_dimensions: Size2D,
}

impl Space {
    fn new(index: usize, hard: bool) -> Space {
        Space {
            index,
            hard,
            actions: LayoutActionList::new(),
            combined_dimensions: Size2D::zero(),
        }
    }
}

#[derive(Debug, Clone)]
struct Subspace {
    origin: Size2D,
    anchor: Size2D,
    factor: i32,

    boxes: Vec<(Size, Size, Layout)>,

    usable: Size2D,
    dimensions: Size2D,

    space: SpaceState,
}

impl Subspace {
    fn new(origin: Size2D, usable: Size2D, axes: LayoutAxes) -> Subspace {
        Subspace {
            origin,
            anchor: axes.anchor(usable),
            factor: axes.secondary.axis.factor(),
            boxes: vec![],
            usable: axes.generalize(usable),
            dimensions: Size2D::zero(),
            space: SpaceState::Forbidden,
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
            space: Space::new(0, true),
            sub: Subspace::new(space.start(), space.usable(), axes),
        }
    }

    pub fn add(&mut self, layout: Layout) -> LayoutResult<()> {
        if let SpaceState::Soft(space) = self.sub.space {
            self.add_space(space, SpaceKind::Hard);
        }

        let size = self.ctx.axes.generalize(layout.dimensions);

        let mut new_dimensions = Size2D {
            x: crate::size::max(self.sub.dimensions.x, size.x),
            y: self.sub.dimensions.y + size.y
        };

        while !self.sub.usable.fits(new_dimensions) {
            if self.space_is_last() && self.space_is_empty() {
                Err(LayoutError::NotEnoughSpace("failed to add box to stack"))?;
            }

            self.finish_space(true);
            new_dimensions = size;
        }

        let offset = self.sub.dimensions.y;
        let anchor = self.ctx.axes.primary.anchor(size.x);

        self.sub.boxes.push((offset, anchor, layout));
        self.sub.dimensions = new_dimensions;
        self.sub.space = SpaceState::Allowed;

        Ok(())
    }

    pub fn add_multiple(&mut self, layouts: MultiLayout) -> LayoutResult<()> {
        for layout in layouts {
            self.add(layout)?;
        }
        Ok(())
    }

    pub fn add_space(&mut self, space: Size, kind: SpaceKind) {
        if kind == SpaceKind::Soft {
            if self.sub.space != SpaceState::Forbidden {
                self.sub.space = SpaceState::Soft(space);
            }
        } else {
            if self.sub.dimensions.y + space > self.sub.usable.y {
                self.sub.dimensions.y = self.sub.usable.y;
            } else {
                self.sub.dimensions.y += space;
            }

            if kind == SpaceKind::Hard {
                self.sub.space = SpaceState::Forbidden;
            }
        }
    }

    pub fn set_axes(&mut self, axes: LayoutAxes) {
        if axes != self.ctx.axes {
            self.finish_subspace();
            let (origin, usable) = self.remaining_subspace();
            self.ctx.axes = axes;
            self.sub = Subspace::new(origin, usable, axes);
        }
    }

    pub fn set_spaces(&mut self, spaces: LayoutSpaces, replace_empty: bool) {
        if replace_empty && self.space_is_empty() {
            self.ctx.spaces = spaces;
            self.start_space(0, self.space.hard);
        } else {
            self.ctx.spaces.truncate(self.space.index + 1);
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
        self.space.combined_dimensions == Size2D::zero()
            && self.space.actions.is_empty()
            && self.sub.dimensions == Size2D::zero()
    }

    pub fn space_is_last(&self) -> bool {
        self.space.index == self.ctx.spaces.len() - 1
    }

    pub fn finish(mut self) -> MultiLayout {
        if self.space.hard || !self.space_is_empty() {
            self.finish_space(false);
        }
        self.layouts
    }

    pub fn finish_space(&mut self, hard: bool) {
        self.finish_subspace();

        let space = self.ctx.spaces[self.space.index];

        self.layouts.add(Layout {
            dimensions: match self.ctx.expand {
                true => space.dimensions,
                false => self.space.combined_dimensions.padded(space.padding),
            },
            actions: self.space.actions.to_vec(),
            debug_render: true,
        });

        self.start_space(self.next_space(), hard);
    }

    fn start_space(&mut self, space: usize, hard: bool) {
        self.space = Space::new(space, hard);

        let space = self.ctx.spaces[space];
        self.sub = Subspace::new(space.start(), space.usable(), self.ctx.axes);
    }

    fn next_space(&self) -> usize {
        (self.space.index + 1).min(self.ctx.spaces.len() - 1)
    }

    fn finish_subspace(&mut self) {
        let factor = self.ctx.axes.secondary.axis.factor();
        let anchor =
            self.ctx.axes.anchor(self.sub.usable)
            - self.ctx.axes.anchor(Size2D::with_y(self.sub.dimensions.y));

        for (offset, layout_anchor, layout) in self.sub.boxes.drain(..) {
            let pos = self.sub.origin
                + self.ctx.axes.specialize(
                    anchor + Size2D::new(-layout_anchor, factor * offset)
                );

            self.space.actions.add_layout(pos, layout);
        }

        if self.ctx.axes.primary.needs_expansion() {
            self.sub.dimensions.x = self.sub.usable.x;
        }

        if self.ctx.axes.secondary.needs_expansion() {
            self.sub.dimensions.y = self.sub.usable.y;
        }

        let space = self.ctx.spaces[self.space.index];
        let origin = self.sub.origin;
        let dimensions = self.ctx.axes.specialize(self.sub.dimensions);
        self.space.combined_dimensions.max_eq(origin - space.start() + dimensions);
    }

    fn remaining_subspace(&self) -> (Size2D, Size2D) {
        let new_origin = self.sub.origin + match self.ctx.axes.secondary.axis.is_positive() {
            true => self.ctx.axes.specialize(Size2D::with_y(self.sub.dimensions.y)),
            false => Size2D::zero(),
        };

        let new_usable = self.ctx.axes.specialize(Size2D {
            x: self.sub.usable.x,
            y: self.sub.usable.y - self.sub.dimensions.y - self.sub.space.soft_or_zero(),
        });

        (new_origin, new_usable)
    }
}
