use super::*;

/// A node that arranges its children into a paragraph.
#[derive(Debug, Clone, PartialEq)]
pub struct NodePar {
    /// The `main` and `cross` directions of this paragraph.
    ///
    /// The children are placed in lines along the `cross` direction. The lines
    /// are stacked along the `main` direction.
    pub dirs: LayoutDirs,
    /// Whether to expand the cross axis to fill the area or to fit the content.
    pub cross_expansion: Expansion,
    /// The spacing to insert after each line.
    pub line_spacing: Length,
    /// The nodes to be arranged in a paragraph.
    pub children: Vec<Node>,
    /// How to align this paragraph in _its_ parent.
    pub align: ChildAlign,
}

impl Layout for NodePar {
    fn layout(&self, ctx: &mut LayoutContext, areas: &Areas) -> Layouted {
        let mut layouter = ParLayouter::new(self, areas.clone());
        for child in &self.children {
            match child.layout(ctx, &layouter.areas) {
                Layouted::Spacing(spacing) => layouter.push_spacing(spacing),
                Layouted::Frame(frame, align) => layouter.push_frame(frame, align.cross),
                Layouted::Frames(frames, align) => {
                    for frame in frames {
                        layouter.push_frame(frame, align.cross);
                    }
                }
            }
        }
        Layouted::Frames(layouter.finish(), self.align)
    }
}

impl From<NodePar> for NodeAny {
    fn from(par: NodePar) -> Self {
        Self::new(par)
    }
}

struct ParLayouter<'a> {
    par: &'a NodePar,
    main: SpecAxis,
    cross: SpecAxis,
    dirs: LayoutDirs,
    areas: Areas,
    finished: Vec<Frame>,
    lines: Vec<(Length, Frame, Align)>,
    lines_size: Gen<Length>,
    run: Vec<(Length, Frame, Align)>,
    run_size: Gen<Length>,
    run_ruler: Align,
}

impl<'a> ParLayouter<'a> {
    fn new(par: &'a NodePar, areas: Areas) -> Self {
        Self {
            par,
            main: par.dirs.main.axis(),
            cross: par.dirs.cross.axis(),
            dirs: par.dirs,
            areas,
            finished: vec![],
            lines: vec![],
            lines_size: Gen::ZERO,
            run: vec![],
            run_size: Gen::ZERO,
            run_ruler: Align::Start,
        }
    }

    fn push_spacing(&mut self, amount: Length) {
        let cross_max = self.areas.current.get(self.cross);
        self.run_size.cross = (self.run_size.cross + amount).min(cross_max);
    }

    fn push_frame(&mut self, frame: Frame, align: Align) {
        if self.run_ruler > align {
            self.finish_run();
        }

        let fits = {
            let mut usable = self.areas.current;
            *usable.get_mut(self.cross) -= self.run_size.cross;
            usable.fits(frame.size)
        };

        if !fits {
            self.finish_run();

            while !self.areas.current.fits(frame.size) {
                if self.areas.in_full_last() {
                    // TODO: Diagnose once the necessary spans exist.
                    break;
                } else {
                    self.finish_area();
                }
            }
        }

        let size = frame.size.switch(self.dirs);
        self.run.push((self.run_size.cross, frame, align));

        self.run_size.cross += size.cross;
        self.run_size.main = self.run_size.main.max(size.main);
        self.run_ruler = align;
    }

    fn finish_run(&mut self) {
        let full_size = {
            let full = self.areas.full.switch(self.dirs);
            Gen::new(
                self.run_size.main,
                self.par
                    .cross_expansion
                    .resolve(self.run_size.cross.min(full.cross), full.cross),
            )
        };

        let mut output = Frame::new(full_size.switch(self.dirs).to_size());

        for (before, frame, align) in std::mem::take(&mut self.run) {
            let child_cross_size = frame.size.get(self.cross);

            // Position along the cross axis.
            let cross = align.resolve(if self.dirs.cross.is_positive() {
                let after_with_self = self.run_size.cross - before;
                before .. full_size.cross - after_with_self
            } else {
                let before_with_self = before + child_cross_size;
                let after = self.run_size.cross - (before + child_cross_size);
                full_size.cross - before_with_self .. after
            });

            let pos = Gen::new(Length::ZERO, cross).switch(self.dirs).to_point();
            output.push_frame(pos, frame);
        }

        self.lines.push((self.lines_size.main, output, self.run_ruler));

        let main_offset = full_size.main + self.par.line_spacing;
        *self.areas.current.get_mut(self.main) -= main_offset;
        self.lines_size.main += main_offset;
        self.lines_size.cross = self.lines_size.cross.max(full_size.cross);

        self.run_size = Gen::ZERO;
        self.run_ruler = Align::Start;
    }

    fn finish_area(&mut self) {
        let size = self.lines_size;
        let mut output = Frame::new(size.switch(self.dirs).to_size());

        for (before, run, cross_align) in std::mem::take(&mut self.lines) {
            let child_size = run.size.switch(self.dirs);

            // Position along the main axis.
            let main = if self.dirs.main.is_positive() {
                before
            } else {
                size.main - (before + child_size.main)
            };

            // Align along the cross axis.
            let cross = cross_align.resolve(if self.dirs.cross.is_positive() {
                Length::ZERO .. size.cross - child_size.cross
            } else {
                size.cross - child_size.cross .. Length::ZERO
            });

            let pos = Gen::new(main, cross).switch(self.dirs).to_point();
            output.push_frame(pos, run);
        }

        self.finished.push(output);

        self.areas.next();
        self.lines_size = Gen::ZERO;
    }

    fn finish(mut self) -> Vec<Frame> {
        self.finish_run();
        self.finish_area();
        self.finished
    }
}
