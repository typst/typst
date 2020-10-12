use super::*;

/// A node that stacks and aligns its children.
#[derive(Debug, Clone, PartialEq)]
pub struct Stack {
    /// The `main` and `cross` directions of this stack.
    ///
    /// The children are stacked along the `main` direction. The `cross`
    /// direction is required for aligning the children.
    pub dirs: Gen<Dir>,
    /// How to align this stack in _its_ parent.
    pub aligns: Gen<Align>,
    /// Whether to expand the axes to fill the area or to fit the content.
    pub expansion: Gen<Expansion>,
    /// The nodes to be stacked.
    pub children: Vec<LayoutNode>,
}

impl Layout for Stack {
    fn layout(&self, ctx: &mut LayoutContext, areas: &Areas) -> Vec<Layouted> {
        let mut layouter = StackLayouter::new(self, areas.clone());
        for child in &self.children {
            for layouted in child.layout(ctx, &layouter.areas) {
                match layouted {
                    Layouted::Spacing(spacing) => layouter.spacing(spacing),
                    Layouted::Boxed(boxed, aligns) => layouter.boxed(boxed, aligns),
                }
            }
        }
        layouter.finish()
    }
}

impl From<Stack> for LayoutNode {
    fn from(stack: Stack) -> Self {
        Self::dynamic(stack)
    }
}

struct StackLayouter<'a> {
    stack: &'a Stack,
    main: SpecAxis,
    dirs: Gen<Dir>,
    areas: Areas,
    layouted: Vec<Layouted>,
    boxes: Vec<(Length, BoxLayout, Gen<Align>)>,
    used: Gen<Length>,
    ruler: Align,
}

impl<'a> StackLayouter<'a> {
    fn new(stack: &'a Stack, areas: Areas) -> Self {
        Self {
            stack,
            main: stack.dirs.main.axis(),
            dirs: stack.dirs,
            areas,
            layouted: vec![],
            boxes: vec![],
            used: Gen::ZERO,
            ruler: Align::Start,
        }
    }

    fn spacing(&mut self, amount: Length) {
        let main_rest = self.areas.current.rem.get_mut(self.main);
        let capped = amount.min(*main_rest);
        *main_rest -= capped;
        self.used.main += capped;
    }

    fn boxed(&mut self, layout: BoxLayout, aligns: Gen<Align>) {
        if self.ruler > aligns.main {
            self.finish_area();
        }

        while !self.areas.current.rem.fits(layout.size) {
            if self.areas.in_full_last() {
                // TODO: Diagnose once the necessary spans exist.
                let _ = warning!("cannot fit box into any area");
                break;
            } else {
                self.finish_area();
            }
        }

        let size = layout.size.switch(self.dirs);
        self.boxes.push((self.used.main, layout, aligns));

        *self.areas.current.rem.get_mut(self.main) -= size.main;
        self.used.main += size.main;
        self.used.cross = self.used.cross.max(size.cross);
        self.ruler = aligns.main;
    }

    fn finish_area(&mut self) {
        let size = {
            let full = self.areas.current.full.switch(self.dirs);
            Gen::new(
                match self.stack.expansion.main {
                    Expansion::Fill => full.main,
                    Expansion::Fit => self.used.main.min(full.main),
                },
                match self.stack.expansion.cross {
                    Expansion::Fill => full.cross,
                    Expansion::Fit => self.used.cross.min(full.cross),
                },
            )
        };

        let mut output = BoxLayout::new(size.switch(self.dirs).to_size());

        for (before, layout, aligns) in std::mem::take(&mut self.boxes) {
            let child_size = layout.size.switch(self.dirs);

            // Align along the main axis.
            let main = aligns.main.apply(if self.dirs.main.is_positive() {
                let after_with_self = self.used.main - before;
                before .. size.main - after_with_self
            } else {
                let before_with_self = before + child_size.main;
                let after = self.used.main - (before + child_size.main);
                size.main - before_with_self .. after
            });

            // Align along the cross axis.
            let cross = aligns.cross.apply(if self.dirs.cross.is_positive() {
                Length::ZERO .. size.cross - child_size.cross
            } else {
                size.cross - child_size.cross .. Length::ZERO
            });

            let pos = Gen::new(main, cross).switch(self.dirs).to_point();
            output.push_layout(pos, layout);
        }

        self.layouted.push(Layouted::Boxed(output, self.stack.aligns));

        self.areas.next();
        self.used = Gen::ZERO;
        self.ruler = Align::Start;
    }

    fn finish(mut self) -> Vec<Layouted> {
        self.finish_area();
        self.layouted
    }
}
