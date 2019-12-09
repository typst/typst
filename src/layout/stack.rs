use smallvec::smallvec;
use super::*;

/// The stack layouter arranges boxes stacked onto each other.
///
/// The boxes are laid out in the direction of the secondary layouting axis and
/// are aligned along both axes.
#[derive(Debug, Clone)]
pub struct StackLayouter {
    /// The context for layouter.
    ctx: StackContext,
    /// The output layouts.
    layouts: MultiLayout,
    /// The currently active layout space.
    space: Space,
    /// The remaining subspace of the active space. Whenever the layouting axes
    /// change a new subspace is started.
    sub: Subspace,
}

/// The context for stack layouting.
///
/// See [`LayoutContext`] for details about the fields.
#[derive(Debug, Clone)]
pub struct StackContext {
    pub spaces: LayoutSpaces,
    pub axes: LayoutAxes,
    pub alignment: LayoutAlignment,
}

/// A layout space composed of subspaces which can have different axes and
/// alignments.
#[derive(Debug, Clone)]
struct Space {
    /// The index of this space in the list of spaces.
    index: usize,
    /// Whether to add the layout for this space even if it would be empty.
    hard: bool,
    /// The so-far accumulated subspaces.
    subs: Vec<Subspace>,
}

/// A part of a space with fixed axes and secondary alignment.
#[derive(Debug, Clone)]
struct Subspace {
    /// The axes along which contents in this subspace are laid out.
    axes: LayoutAxes,
    /// The secondary alignment of this subspace.
    alignment: Alignment,
    /// The beginning of this subspace in the parent space (specialized).
    origin: Size2D,
    /// The total usable space of this subspace (generalized).
    usable: Size2D,
    /// The used size of this subspace (generalized), with
    /// - `x` being the maximum of the primary size of all boxes.
    /// - `y` being the total extent of all boxes and space in the secondary
    ///   direction.
    size: Size2D,
    /// The so-far accumulated layouts.
    layouts: Vec<LayoutEntry>,
    /// The last added spacing if the last added thing was spacing.
    last_spacing: LastSpacing,
}

/// A single layout in a subspace.
#[derive(Debug, Clone)]
struct LayoutEntry {
    /// The offset of this box on the secondary axis.
    offset: Size,
    /// The layout itself.
    layout: Layout,
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
            sub: Subspace::new(axes, Alignment::Origin, space.start(), space.usable()),
        }
    }

    /// Add a layout to the stack.
    pub fn add(&mut self, layout: Layout) -> LayoutResult<()> {
        if layout.alignment.secondary != self.sub.alignment {
            self.finish_subspace(layout.alignment.secondary);
        }

        // Add a cached soft space if there is one.
        if let LastSpacing::Soft(space, _) = self.sub.last_spacing {
            self.add_spacing(space, SpacingKind::Hard);
        }

        // The new primary size is the maximum of the current one and the
        // layout's one while the secondary size grows by the layout's size.
        let size = self.ctx.axes.generalize(layout.dimensions);
        let mut new_size = Size2D {
            x: crate::size::max(self.sub.size.x, size.x),
            y: self.sub.size.y + size.y
        };

        // Find the first (sub-)space that fits the layout.
        while !self.sub.usable.fits(new_size) {
            if self.space_is_last() && self.space_is_empty() {
                error!("box of size {} does not fit into remaining stack of size {}",
                    size, self.sub.usable - Size2D::with_y(self.sub.size.y));
            }

            self.finish_space(true);
            new_size = size;
        }

        // The secondary offset from the start of layouts is given by the
        // current primary size of the subspace.
        let offset = self.sub.size.y;
        self.sub.layouts.push(LayoutEntry {
            offset,
            layout,
        });

        // The new size of the subspace is the previously calculated
        // combination.
        self.sub.size = new_size;

        // Since the last item was a box, last spacing is reset to `None`.
        self.sub.last_spacing = LastSpacing::None;

        Ok(())
    }

    /// Add multiple layouts to the stack.
    ///
    /// This function simply calls `add` for each layout.
    pub fn add_multiple(&mut self, layouts: MultiLayout) -> LayoutResult<()> {
        for layout in layouts {
            self.add(layout)?;
        }
        Ok(())
    }

    /// Add secondary spacing to the stack.
    pub fn add_spacing(&mut self, space: Size, kind: SpacingKind) {
        match kind {
            // A hard space is directly added to the sub's size.
            SpacingKind::Hard => {
                if self.sub.size.y + space > self.sub.usable.y {
                    self.sub.size.y = self.sub.usable.y;
                } else {
                    self.sub.size.y += space;
                }

                self.sub.last_spacing = LastSpacing::Hard;
            }

            // A hard space is cached if it is not consumed by a hard space or
            // previous soft space with higher level.
            SpacingKind::Soft(level) => {
                let consumes = match self.sub.last_spacing {
                    LastSpacing::None => true,
                    LastSpacing::Soft(_, prev) if level < prev => true,
                    _ => false,
                };

                if consumes {
                    self.sub.last_spacing = LastSpacing::Soft(space, level);
                }
            }
        }
    }

    /// Change the layouting axes used by this layouter.
    ///
    /// This starts a new subspace (if the axes are actually different from the
    /// current ones).
    pub fn set_axes(&mut self, axes: LayoutAxes) {
        if axes != self.ctx.axes {
            self.finish_subspace(Alignment::Origin);

            let (origin, usable) = self.remaining_subspace();
            self.sub = Subspace::new(axes, Alignment::Origin, origin, usable);
            self.ctx.axes = axes;
        }
    }

    /// Change the layouting spaces to use.
    ///
    /// If `replace_empty` is true, the current space is replaced if there are
    /// no boxes laid into it yet. Otherwise, only the followup spaces are
    /// replaced.
    pub fn set_spaces(&mut self, spaces: LayoutSpaces, replace_empty: bool) {
        if replace_empty && self.space_is_empty() {
            self.ctx.spaces = spaces;
            self.start_space(0, self.space.hard);
        } else {
            self.ctx.spaces.truncate(self.space.index + 1);
            self.ctx.spaces.extend(spaces);
        }
    }

    /// The remaining unpadded, unexpanding spaces. If a multi-layout is laid
    /// out into these spaces, it will fit into this stack.
    pub fn remaining(&self) -> LayoutSpaces {
        let mut spaces = smallvec![LayoutSpace {
            dimensions: self.remaining_subspace().1,
            padding: SizeBox::zero(),
            expand: (false, false),
        }];

        for space in &self.ctx.spaces[self.next_space()..] {
            spaces.push(space.usable_space());
        }

        spaces
    }

    /// The usable size along the primary axis.
    pub fn primary_usable(&self) -> Size {
        self.sub.usable.x
    }

    /// Whether the current layout space (not subspace) is empty.
    pub fn space_is_empty(&self) -> bool {
        self.subspace_is_empty() && self.space.subs.is_empty()
    }

    /// Whether the current layout space is the last is the followup list.
    pub fn space_is_last(&self) -> bool {
        self.space.index == self.ctx.spaces.len() - 1
    }

    /// Compute the finished multi-layout.
    pub fn finish(mut self) -> MultiLayout {
        if self.space.hard || !self.space_is_empty() {
            self.finish_space(false);
        }
        self.layouts
    }

    /// Finish the current space and start a new one.
    pub fn finish_space(&mut self, hard: bool) {
        self.finish_subspace(Alignment::Origin);

        println!();
        println!("FINISHING SPACE:");
        println!();

        let space = self.ctx.spaces[self.space.index];
        let mut subs = std::mem::replace(&mut self.space.subs, vec![]);

        // ---------------------------------------------------------------------
        // Compute the size of the whole space.
        let usable = space.usable();
        let mut max = Size2D {
            x: if space.expand.0 { usable.x } else { Size::zero() },
            y: if space.expand.1 { usable.y } else { Size::zero() },
        };

        // The total size is determined by the maximum position + extent of one
        // of the boxes.
        for sub in &subs {
            max.max_eq(sub.origin + sub.axes.specialize(sub.size));
        }

        let dimensions = max.padded(space.padding);

        println!("WITH DIMENSIONS: {}", dimensions);

        println!("SUBS: {:#?}", subs);

        // ---------------------------------------------------------------------
        // Justify the boxes according to their alignment and give each box
        // the appropriate origin and usable space.

        // use Alignment::*;

        for sub in &mut subs {
            // The usable width should not exceed the total usable width
            // (previous value) or the maximum width of the layout as a whole.
            sub.usable.x = crate::size::min(
                sub.usable.x,
                sub.axes.specialize(max - sub.origin).x,
            );

            sub.usable.y = sub.size.y;
        }

        // if space.expand.1 {
        //     let height = subs.iter().map(|sub| sub.size.y).sum();
        //     let centers = subs.iter()
        //         .filter(|sub| sub.alignment == Alignment::Center)
        //         .count()
        //         .max(1);

        //     let grow = max.y - height;
        //     let center_grow = grow / (centers as i32);

        //     println!("center grow = {}", center_grow);

        //     let mut offset = Size::zero();
        //     for sub in &mut subs {
        //         sub.origin.y += offset;
        //         if sub.alignment == Center {
        //             sub.usable.y += center_grow;
        //             offset += center_grow;
        //         }
        //     }

        //     if let Some(last) = subs.last_mut() {
        //         last.usable.y += grow - offset;
        //     }
        // }

        // ---------------------------------------------------------------------
        // Do the thing

        // Add a debug box with this boxes size.
        let mut actions = LayoutActions::new();
        actions.add(LayoutAction::DebugBox(dimensions));

        for sub in subs {
            let LayoutAxes { primary, secondary } = sub.axes;

            // The factor is +1 if the axis is positive and -1 otherwise.
            let factor = sub.axes.secondary.factor();

            // The anchor is the position of the origin-most point of the
            // layout.
            let anchor =
                sub.usable.y.anchor(sub.alignment, secondary.is_positive())
                - factor * sub.size.y.anchor(sub.alignment, true);

            for entry in sub.layouts {
                let layout = entry.layout;
                let alignment = layout.alignment.primary;
                let size = sub.axes.generalize(layout.dimensions);

                let x =
                    sub.usable.x.anchor(alignment, primary.is_positive())
                    - size.x.anchor(alignment, primary.is_positive());

                let y = anchor
                    + factor * entry.offset
                    - size.y.anchor(Alignment::Origin, secondary.is_positive());

                let pos = sub.origin + sub.axes.specialize(Size2D::new(x, y));
                actions.add_layout(pos, layout);
            }
        }

        // ---------------------------------------------------------------------

        self.layouts.push(Layout {
            dimensions,
            baseline: None,
            alignment: self.ctx.alignment,
            actions: actions.to_vec(),
        });

        self.start_space(self.next_space(), hard);
    }

    /// Start a new space with the given index.
    fn start_space(&mut self, space: usize, hard: bool) {
        // Start the space.
        self.space = Space::new(space, hard);

        // Start the subspace.
        let space = self.ctx.spaces[space];
        let axes = self.ctx.axes;
        self.sub = Subspace::new(axes, Alignment::Origin, space.start(), space.usable());
    }

    /// The index of the next space.
    fn next_space(&self) -> usize {
        (self.space.index + 1).min(self.ctx.spaces.len() - 1)
    }

    /// Finish the current subspace.
    fn finish_subspace(&mut self, new_alignment: Alignment) {
        let empty = self.subspace_is_empty();

        let axes = self.ctx.axes;
        let (origin, usable) = self.remaining_subspace();
        let new_sub = Subspace::new(axes, new_alignment, origin, usable);
        let sub = std::mem::replace(&mut self.sub, new_sub);

        if !empty {
            self.space.subs.push(sub);
        }
    }

    /// The remaining sub
    fn remaining_subspace(&self) -> (Size2D, Size2D) {
        let offset = self.sub.size.y + self.sub.last_spacing.soft_or_zero();

        let new_origin = self.sub.origin + match self.ctx.axes.secondary.is_positive() {
            true => self.ctx.axes.specialize(Size2D::with_y(offset)),
            false => Size2D::zero(),
        };

        let new_usable = self.ctx.axes.specialize(Size2D {
            x: self.sub.usable.x,
            y: self.sub.usable.y - offset,
        });

        (new_origin, new_usable)
    }

    /// Whether the current layout space (not subspace) is empty.
    fn subspace_is_empty(&self) -> bool {
        self.sub.layouts.is_empty() && self.sub.size == Size2D::zero()
    }
}

impl Space {
    fn new(index: usize, hard: bool) -> Space {
        Space {
            index,
            hard,
            subs: vec![],
        }
    }
}

impl Subspace {
    fn new(axes: LayoutAxes, alignment: Alignment, origin: Size2D, usable: Size2D) -> Subspace {
        Subspace {
            axes,
            alignment,
            origin,
            usable: axes.generalize(usable),
            size: Size2D::zero(),
            layouts: vec![],
            last_spacing: LastSpacing::Hard,
        }
    }
}
