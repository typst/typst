use super::*;
use crate::geom::Linear;

/// A node that stacks and aligns its children.
///
/// # Alignment
/// Individual layouts can be aligned at `Start`, `Center` or  `End` along both
/// axes. These alignments are with processed with respect to the size of the
/// finished layout and not the total usable size. This means that a later
/// layout can have influence on the position of an earlier one. Consider the
/// following example.
/// ```typst
/// [align: right][A word.]
/// [align: left][A sentence with a couple more words.]
/// ```
/// The resulting layout looks like this:
/// ```text
/// |--------------------------------------|
/// |                              A word. |
/// |                                      |
/// | A sentence with a couple more words. |
/// |--------------------------------------|
/// ```
/// The position of the first aligned box thus depends on the length of the
/// sentence in the second box.
#[derive(Debug, Clone, PartialEq)]
pub struct Stack {
    pub dirs: Gen2<Dir>,
    pub children: Vec<LayoutNode>,
    pub aligns: Gen2<GenAlign>,
    pub expand: Spec2<bool>,
}

#[async_trait(?Send)]
impl Layout for Stack {
    async fn layout(
        &self,
        ctx: &mut LayoutContext,
        constraints: LayoutConstraints,
    ) -> Vec<LayoutItem> {
        let mut layouter = StackLayouter::new(StackContext {
            dirs: self.dirs,
            spaces: constraints.spaces,
            repeat: constraints.repeat,
            expand: self.expand,
        });

        for child in &self.children {
            let items = child
                .layout(ctx, LayoutConstraints {
                    spaces: layouter.remaining(),
                    repeat: constraints.repeat,
                })
                .await;

            for item in items {
                match item {
                    LayoutItem::Spacing(amount) => layouter.push_spacing(amount),
                    LayoutItem::Box(boxed, aligns) => layouter.push_box(boxed, aligns),
                }
            }
        }

        layouter
            .finish()
            .into_iter()
            .map(|boxed| LayoutItem::Box(boxed, self.aligns))
            .collect()
    }
}

impl From<Stack> for LayoutNode {
    fn from(stack: Stack) -> Self {
        Self::dynamic(stack)
    }
}

/// Performs the stack layouting.
pub(super) struct StackLayouter {
    /// The context used for stack layouting.
    pub ctx: StackContext,
    /// The finished layouts.
    pub layouts: Vec<BoxLayout>,
    /// The in-progress space.
    pub space: Space,
}

/// The context for stack layouting.
#[derive(Debug, Clone)]
pub(super) struct StackContext {
    /// The layouting directions.
    pub dirs: Gen2<Dir>,
    /// The spaces to layout into.
    pub spaces: Vec<LayoutSpace>,
    /// Whether to spill over into copies of the last space or finish layouting
    /// when the last space is used up.
    pub repeat: bool,
    /// Whether to expand the size of the resulting layout to the full size of
    /// this space or to shrink it to fit the content.
    pub expand: Spec2<bool>,
}

impl StackLayouter {
    /// Create a new stack layouter.
    pub fn new(ctx: StackContext) -> Self {
        let space = ctx.spaces[0];
        Self {
            ctx,
            layouts: vec![],
            space: Space::new(0, true, space.size),
        }
    }

    /// Add a layout to the stack.
    pub fn push_box(&mut self, layout: BoxLayout, aligns: Gen2<GenAlign>) {
        // If the alignment cannot be fitted in this space, finish it.
        //
        // TODO: Issue warning for non-fitting alignment in non-repeating
        //       context.
        if aligns.main < self.space.allowed_align && self.ctx.repeat {
            self.finish_space(true);
        }

        // TODO: Issue warning about overflow if there is overflow in a
        //       non-repeating context.
        if !self.space.usable.fits(layout.size) && self.ctx.repeat {
            self.skip_to_fitting_space(layout.size);
        }

        // Change the usable space and size of the space.
        self.update_metrics(layout.size.switch(self.ctx.dirs));

        // Add the box to the vector and remember that spacings are allowed
        // again.
        self.space.layouts.push((layout, aligns));
        self.space.allowed_align = aligns.main;
    }

    /// Add spacing to the stack.
    pub fn push_spacing(&mut self, mut spacing: f64) {
        // Reduce the spacing such that it definitely fits.
        let axis = self.ctx.dirs.main.axis();
        spacing = spacing.min(self.space.usable.get(axis));

        let size = Gen2::new(spacing, 0.0);
        self.update_metrics(size);
        self.space.layouts.push((
            BoxLayout::new(size.switch(self.ctx.dirs).to_size()),
            Gen2::default(),
        ));
    }

    fn update_metrics(&mut self, added: Gen2<f64>) {
        let mut used = self.space.used.switch(self.ctx.dirs);
        used.cross = used.cross.max(added.cross);
        used.main += added.main;
        self.space.used = used.switch(self.ctx.dirs).to_size();
        *self.space.usable.get_mut(self.ctx.dirs.main.axis()) -= added.main;
    }

    /// Update the layouting spaces.
    ///
    /// If `replace_empty` is true, the current space is replaced if there are
    /// no boxes laid out into it yet. Otherwise, the followup spaces are
    /// replaced.
    pub fn set_spaces(&mut self, spaces: Vec<LayoutSpace>, replace_empty: bool) {
        if replace_empty && self.space_is_empty() {
            self.ctx.spaces = spaces;
            self.start_space(0, self.space.hard);
        } else {
            self.ctx.spaces.truncate(self.space.index + 1);
            self.ctx.spaces.extend(spaces);
        }
    }

    /// Move to the first space that can fit the given size or do nothing
    /// if no space is capable of that.
    pub fn skip_to_fitting_space(&mut self, size: Size) {
        let start = self.next_space();
        for (index, space) in self.ctx.spaces[start ..].iter().enumerate() {
            if space.size.fits(size) {
                self.finish_space(true);
                self.start_space(start + index, true);
                break;
            }
        }
    }

    /// The remaining inner spaces. If something is laid out into these spaces,
    /// it will fit into this stack.
    pub fn remaining(&self) -> Vec<LayoutSpace> {
        let mut spaces = vec![LayoutSpace {
            base: self.space.size,
            size: self.space.usable,
        }];

        spaces.extend(&self.ctx.spaces[self.next_space() ..]);
        spaces
    }

    /// The remaining usable size.
    pub fn usable(&self) -> Size {
        self.space.usable
    }

    /// Whether the current layout space is empty.
    pub fn space_is_empty(&self) -> bool {
        self.space.used == Size::ZERO && self.space.layouts.is_empty()
    }

    /// Whether the current layout space is the last in the followup list.
    pub fn space_is_last(&self) -> bool {
        self.space.index == self.ctx.spaces.len() - 1
    }

    /// Finish everything up and return the final collection of boxes.
    pub fn finish(mut self) -> Vec<BoxLayout> {
        if self.space.hard || !self.space_is_empty() {
            self.finish_space(false);
        }
        self.layouts
    }

    /// Finish active current space and start a new one.
    pub fn finish_space(&mut self, hard: bool) {
        let dirs = self.ctx.dirs;

        // ------------------------------------------------------------------ //
        // Step 1: Determine the full size of the space.
        // (Mostly done already while collecting the boxes, but here we
        //  expand if necessary.)

        let space = self.ctx.spaces[self.space.index];
        let layout_size = {
            let mut used_size = self.space.used;
            if self.ctx.expand.horizontal {
                used_size.width = space.size.width;
            }
            if self.ctx.expand.vertical {
                used_size.height = space.size.height;
            }
            used_size
        };

        let mut layout = BoxLayout::new(layout_size);

        // ------------------------------------------------------------------ //
        // Step 2: Forward pass. Create a bounding box for each layout in which
        // it will be aligned. Then, go forwards through the boxes and remove
        // what is taken by previous layouts from the following layouts.

        let mut bounds = vec![];
        let mut bound = Rect {
            x0: 0.0,
            y0: 0.0,
            x1: layout_size.width,
            y1: layout_size.height,
        };

        for (layout, _) in &self.space.layouts {
            // First, store the bounds calculated so far (which were reduced
            // by the predecessors of this layout) as the initial bounding box
            // of this layout.
            bounds.push(bound);

            // Then, reduce the bounding box for the following layouts. This
            // layout uses up space from the origin to the end. Thus, it reduces
            // the usable space for following layouts at its origin by its
            // main-axis extent.
            *bound.get_mut(dirs.main.start()) +=
                dirs.main.factor() * layout.size.get(dirs.main.axis());
        }

        // ------------------------------------------------------------------ //
        // Step 3: Backward pass. Reduce the bounding boxes from the previous
        // layouts by what is taken by the following ones.

        let mut main_extent = 0.0;
        for (child, bound) in self.space.layouts.iter().zip(&mut bounds).rev() {
            let (layout, _) = child;

            // Reduce the bounding box of this layout by the following one's
            // main-axis extents.
            *bound.get_mut(dirs.main.end()) -= dirs.main.factor() * main_extent;

            // And then, include this layout's main-axis extent.
            main_extent += layout.size.get(dirs.main.axis());
        }

        // ------------------------------------------------------------------ //
        // Step 4: Align each layout in its bounding box and collect everything
        // into a single finished layout.

        let children = std::mem::take(&mut self.space.layouts);
        for ((child, aligns), bound) in children.into_iter().zip(bounds) {
            // Align the child in its own bounds.
            let local =
                bound.size().anchor(dirs, aligns) - child.size.anchor(dirs, aligns);

            // Make the local position in the bounds global.
            let pos = bound.origin() + local;
            layout.push_layout(pos, child);
        }

        self.layouts.push(layout);

        // ------------------------------------------------------------------ //
        // Step 5: Start the next space.

        self.start_space(self.next_space(), hard)
    }

    fn start_space(&mut self, index: usize, hard: bool) {
        let space = self.ctx.spaces[index];
        self.space = Space::new(index, hard, space.size);
    }

    fn next_space(&self) -> usize {
        (self.space.index + 1).min(self.ctx.spaces.len() - 1)
    }
}

/// A layout space composed of subspaces which can have different directions and
/// alignments.
#[derive(Debug)]
pub(super) struct Space {
    /// The index of this space in `ctx.spaces`.
    index: usize,
    /// Whether to include a layout for this space even if it would be empty.
    hard: bool,
    /// The so-far accumulated layouts.
    layouts: Vec<(BoxLayout, Gen2<GenAlign>)>,
    /// The full size of this space.
    size: Size,
    /// The used size of this space.
    used: Size,
    /// The remaining space.
    usable: Size,
    /// Which alignments for new boxes are still allowed.
    pub(super) allowed_align: GenAlign,
}

impl Space {
    fn new(index: usize, hard: bool, size: Size) -> Self {
        Self {
            index,
            hard,
            layouts: vec![],
            size,
            used: Size::ZERO,
            usable: size,
            allowed_align: GenAlign::Start,
        }
    }
}
