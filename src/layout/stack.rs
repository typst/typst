use super::*;

/// A node that stacks and align its children.
#[derive(Debug, Clone, PartialEq)]
pub struct Stack {
    /// The `main` and `cross` directions of this stack.
    ///
    /// The children are stacked along the `main` direction. The `cross`
    /// direction is required for aligning the children.
    pub flow: Flow,
    /// How to align this stack in _its_ parent.
    pub align: BoxAlign,
    /// Whether to expand the axes to fill the area or to fit the content.
    pub expansion: Gen<Expansion>,
    /// The nodes to be stacked.
    pub children: Vec<LayoutNode>,
}

impl Layout for Stack {
    fn layout(&self, ctx: &mut LayoutContext, areas: &Areas) -> Layouted {
        let mut layouter = StackLayouter::new(self, areas.clone());
        for child in &self.children {
            match child.layout(ctx, &layouter.areas) {
                Layouted::Spacing(spacing) => layouter.push_spacing(spacing),
                Layouted::Layout(layout, align) => layouter.push_layout(layout, align),
                Layouted::Layouts(layouts, align) => {
                    for layout in layouts {
                        layouter.push_layout(layout, align);
                    }
                }
            }
        }
        Layouted::Layouts(layouter.finish(), self.align)
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
    flow: Flow,
    areas: Areas,
    finished: Vec<BoxLayout>,
    layouts: Vec<(Length, BoxLayout, BoxAlign)>,
    used: Gen<Length>,
    ruler: Align,
}

impl<'a> StackLayouter<'a> {
    fn new(stack: &'a Stack, areas: Areas) -> Self {
        Self {
            stack,
            main: stack.flow.main.axis(),
            flow: stack.flow,
            areas,
            finished: vec![],
            layouts: vec![],
            used: Gen::ZERO,
            ruler: Align::Start,
        }
    }

    fn push_spacing(&mut self, amount: Length) {
        let main_rest = self.areas.current.rem.get_mut(self.main);
        let capped = amount.min(*main_rest);
        *main_rest -= capped;
        self.used.main += capped;
    }

    fn push_layout(&mut self, layout: BoxLayout, align: BoxAlign) {
        if self.ruler > align.main {
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

        let size = layout.size.switch(self.flow);
        self.layouts.push((self.used.main, layout, align));

        *self.areas.current.rem.get_mut(self.main) -= size.main;
        self.used.main += size.main;
        self.used.cross = self.used.cross.max(size.cross);
        self.ruler = align.main;
    }

    fn finish_area(&mut self) {
        let full_size = {
            let full = self.areas.current.full.switch(self.flow);
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

        let mut output = BoxLayout::new(full_size.switch(self.flow).to_size());

        for (before, layout, align) in std::mem::take(&mut self.layouts) {
            let child_size = layout.size.switch(self.flow);

            // Align along the main axis.
            let main = align.main.resolve(if self.flow.main.is_positive() {
                let after_with_self = self.used.main - before;
                before .. full_size.main - after_with_self
            } else {
                let before_with_self = before + child_size.main;
                let after = self.used.main - (before + child_size.main);
                full_size.main - before_with_self .. after
            });

            // Align along the cross axis.
            let cross = align.cross.resolve(if self.flow.cross.is_positive() {
                Length::ZERO .. full_size.cross - child_size.cross
            } else {
                full_size.cross - child_size.cross .. Length::ZERO
            });

            let pos = Gen::new(main, cross).switch(self.flow).to_point();
            output.push_layout(pos, layout);
        }

        self.finished.push(output);

        self.areas.next();
        self.used = Gen::ZERO;
        self.ruler = Align::Start;
    }

    fn finish(mut self) -> Vec<BoxLayout> {
        self.finish_area();
        self.finished
    }
}
