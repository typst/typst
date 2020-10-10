use super::*;

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
    pub dirs: Gen<Dir>,
    pub children: Vec<LayoutNode>,
    pub aligns: Gen<Align>,
    pub expand: Spec<bool>,
}

#[async_trait(?Send)]
impl Layout for Stack {
    async fn layout(
        &self,
        ctx: &mut LayoutContext,
        constraints: LayoutConstraints,
    ) -> Vec<LayoutItem> {
        let mut items = vec![];

        let size = constraints.spaces[0].size;
        let mut space = StackSpace::new(self.dirs, self.expand, size);
        let mut i = 0;

        for child in &self.children {
            let child_constraints = LayoutConstraints {
                spaces: {
                    let mut remaining = vec![LayoutSpace {
                        base: space.full_size,
                        size: space.usable,
                    }];
                    let next = (i + 1).min(constraints.spaces.len() - 1);
                    remaining.extend(&constraints.spaces[next ..]);
                    remaining
                },
                repeat: constraints.repeat,
            };

            for item in child.layout(ctx, child_constraints).await {
                match item {
                    LayoutItem::Spacing(spacing) => space.push_spacing(spacing),
                    LayoutItem::Box(mut boxed, aligns) => {
                        let mut last = false;
                        while let Err(back) = space.push_box(boxed, aligns) {
                            boxed = back;
                            if last {
                                break;
                            }

                            items.push(LayoutItem::Box(space.finish(), self.aligns));

                            if i + 1 < constraints.spaces.len() {
                                i += 1;
                            } else {
                                last = true;
                            }

                            let size = constraints.spaces[i].size;
                            space = StackSpace::new(self.dirs, self.expand, size);
                        }
                    }
                }
            }
        }

        items.push(LayoutItem::Box(space.finish(), self.aligns));
        items
    }
}

struct StackSpace {
    dirs: Gen<Dir>,
    expand: Spec<bool>,
    boxes: Vec<(BoxLayout, Gen<Align>)>,
    full_size: Size,
    usable: Size,
    used: Size,
    ruler: Align,
}

impl StackSpace {
    fn new(dirs: Gen<Dir>, expand: Spec<bool>, size: Size) -> Self {
        Self {
            dirs,
            expand,
            boxes: vec![],
            full_size: size,
            usable: size,
            used: Size::ZERO,
            ruler: Align::Start,
        }
    }

    fn push_box(
        &mut self,
        boxed: BoxLayout,
        aligns: Gen<Align>,
    ) -> Result<(), BoxLayout> {
        let main = self.dirs.main.axis();
        let cross = self.dirs.cross.axis();
        if aligns.main < self.ruler || !self.usable.fits(boxed.size) {
            return Err(boxed);
        }

        let size = boxed.size.switch(self.dirs);
        *self.used.get_mut(cross) = self.used.get(cross).max(size.cross);
        *self.used.get_mut(main) += size.main;
        *self.usable.get_mut(main) -= size.main;
        self.boxes.push((boxed, aligns));
        self.ruler = aligns.main;

        Ok(())
    }

    fn push_spacing(&mut self, spacing: Length) {
        let main = self.dirs.main.axis();
        let max = self.usable.get(main);
        let trimmed = spacing.min(max);
        *self.used.get_mut(main) += trimmed;
        *self.usable.get_mut(main) -= trimmed;

        let size = Gen::new(trimmed, Length::ZERO).switch(self.dirs);
        self.boxes.push((BoxLayout::new(size.to_size()), Gen::default()));
    }

    fn finish(mut self) -> BoxLayout {
        let dirs = self.dirs;
        let main = dirs.main.axis();

        if self.expand.horizontal {
            self.used.width = self.full_size.width;
        }

        if self.expand.vertical {
            self.used.height = self.full_size.height;
        }

        let mut sum = Length::ZERO;
        let mut sums = Vec::with_capacity(self.boxes.len() + 1);

        for (boxed, _) in &self.boxes {
            sums.push(sum);
            sum += boxed.size.get(main);
        }

        sums.push(sum);

        let mut layout = BoxLayout::new(self.used);
        let used = self.used.switch(dirs);

        for (i, (boxed, aligns)) in self.boxes.into_iter().enumerate() {
            let size = boxed.size.switch(dirs);

            let before = sums[i];
            let after = sum - sums[i + 1];
            let main_len = used.main - size.main;
            let main_range = if dirs.main.is_positive() {
                before .. main_len - after
            } else {
                main_len - before .. after
            };

            let cross_len = used.cross - size.cross;
            let cross_range = if dirs.cross.is_positive() {
                Length::ZERO .. cross_len
            } else {
                cross_len .. Length::ZERO
            };

            let main = aligns.main.apply(main_range);
            let cross = aligns.cross.apply(cross_range);
            let pos = Gen::new(main, cross).switch(dirs).to_point();

            layout.push_layout(pos, boxed);
        }

        layout
    }
}

impl From<Stack> for LayoutNode {
    fn from(stack: Stack) -> Self {
        Self::dynamic(stack)
    }
}
