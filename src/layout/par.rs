use super::*;

/// A node that arranges its children into a paragraph.
#[derive(Debug, Clone, PartialEq)]
pub struct Par {
    /// The `main` and `cross` directions of this paragraph.
    ///
    /// The children are placed in lines along the `cross` direction. The lines
    /// are stacked along the `main` direction.
    pub dirs: Gen<Dir>,
    /// How to align this paragraph in _its_ parent.
    pub aligns: Gen<Align>,
    /// Whether to expand the cross axis to fill the area or to fit the content.
    pub cross_expansion: Expansion,
    /// The spacing to insert after each line.
    pub line_spacing: Length,
    /// The nodes to be arranged in a paragraph.
    pub children: Vec<LayoutNode>,
}

impl Layout for Par {
    fn layout(&self, ctx: &mut LayoutContext, areas: &Areas) -> Vec<Layouted> {
        let mut layouter = ParLayouter::new(self, areas.clone());
        for child in &self.children {
            for layouted in child.layout(ctx, &layouter.areas) {
                match layouted {
                    Layouted::Spacing(spacing) => layouter.spacing(spacing),
                    Layouted::Boxed(boxed, aligns) => layouter.boxed(boxed, aligns.cross),
                }
            }
        }
        layouter.finish()
    }
}

impl From<Par> for LayoutNode {
    fn from(par: Par) -> Self {
        Self::dynamic(par)
    }
}

struct ParLayouter<'a> {
    par: &'a Par,
    main: SpecAxis,
    cross: SpecAxis,
    dirs: Gen<Dir>,
    areas: Areas,
    layouted: Vec<Layouted>,
    lines: Vec<(Length, BoxLayout, Align)>,
    lines_size: Gen<Length>,
    run: Vec<(Length, BoxLayout, Align)>,
    run_size: Gen<Length>,
    run_ruler: Align,
}

impl<'a> ParLayouter<'a> {
    fn new(par: &'a Par, areas: Areas) -> Self {
        Self {
            par,
            main: par.dirs.main.axis(),
            cross: par.dirs.cross.axis(),
            dirs: par.dirs,
            areas,
            layouted: vec![],
            lines: vec![],
            lines_size: Gen::ZERO,
            run: vec![],
            run_size: Gen::ZERO,
            run_ruler: Align::Start,
        }
    }

    fn spacing(&mut self, amount: Length) {
        let cross_full = self.areas.current.rem.get(self.cross);
        self.run_size.cross = (self.run_size.cross + amount).min(cross_full);
    }

    fn boxed(&mut self, layout: BoxLayout, align: Align) {
        if self.run_ruler > align {
            self.finish_run();
        }

        let fits = {
            let mut usable = self.areas.current.rem;
            *usable.get_mut(self.cross) -= self.run_size.cross;
            usable.fits(layout.size)
        };

        if !fits {
            self.finish_run();

            while !self.areas.current.rem.fits(layout.size) {
                if self.areas.in_full_last() {
                    // TODO: Diagnose once the necessary spans exist.
                    let _ = warning!("cannot fit box into any area");
                    break;
                } else {
                    self.finish_area();
                }
            }
        }

        let size = layout.size.switch(self.dirs);
        self.run.push((self.run_size.cross, layout, align));

        self.run_size.cross += size.cross;
        self.run_size.main = self.run_size.main.max(size.main);
        self.run_ruler = align;
    }

    fn finish_run(&mut self) {
        let size = Gen::new(self.run_size.main, match self.par.cross_expansion {
            Expansion::Fill => self.areas.current.full.get(self.cross),
            Expansion::Fit => self.run_size.cross,
        });

        let mut output = BoxLayout::new(size.switch(self.dirs).to_size());

        for (before, layout, align) in std::mem::take(&mut self.run) {
            let child_cross_size = layout.size.get(self.cross);

            // Position along the cross axis.
            let cross = align.apply(if self.dirs.cross.is_positive() {
                let after_with_self = self.run_size.cross - before;
                before .. size.cross - after_with_self
            } else {
                let before_with_self = before + child_cross_size;
                let after = self.run_size.cross - (before + child_cross_size);
                size.cross - before_with_self .. after
            });

            let pos = Gen::new(Length::ZERO, cross).switch(self.dirs).to_point();
            output.push_layout(pos, layout);
        }

        self.lines.push((self.lines_size.main, output, self.run_ruler));

        let main_offset = size.main + self.par.line_spacing;
        *self.areas.current.rem.get_mut(self.main) -= main_offset;
        self.lines_size.main += main_offset;
        self.lines_size.cross = self.lines_size.cross.max(size.cross);

        self.run_size = Gen::ZERO;
        self.run_ruler = Align::Start;
    }

    fn finish_area(&mut self) {
        let size = self.lines_size;
        let mut output = BoxLayout::new(size.switch(self.dirs).to_size());

        for (before, run, cross_align) in std::mem::take(&mut self.lines) {
            let child_size = run.size.switch(self.dirs);

            // Position along the main axis.
            let main = if self.dirs.main.is_positive() {
                before
            } else {
                size.main - (before + child_size.main)
            };

            // Align along the cross axis.
            let cross = cross_align.apply(if self.dirs.cross.is_positive() {
                Length::ZERO .. size.cross - child_size.cross
            } else {
                size.cross - child_size.cross .. Length::ZERO
            });

            let pos = Gen::new(main, cross).switch(self.dirs).to_point();
            output.push_layout(pos, run);
        }

        self.layouted.push(Layouted::Boxed(output, self.par.aligns));

        self.areas.next();
        self.lines_size = Gen::ZERO;
    }

    fn finish(mut self) -> Vec<Layouted> {
        self.finish_run();
        self.finish_area();
        self.layouted
    }
}
