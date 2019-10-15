use super::*;

/// Layouts syntax trees into boxes.
pub fn layout_tree(tree: &SyntaxTree, ctx: LayoutContext) -> LayoutResult<MultiLayout> {
    let mut layouter = TreeLayouter::new(ctx);
    layouter.layout(tree)?;
    layouter.finish()
}

struct TreeLayouter<'a, 'p> {
    ctx: LayoutContext<'a, 'p>,
    stack: StackLayouter,
    flex: FlexLayouter,
    style: Cow<'a, TextStyle>,
}

impl<'a, 'p> TreeLayouter<'a, 'p> {
    /// Create a new layouter.
    fn new(ctx: LayoutContext<'a, 'p>) -> TreeLayouter<'a, 'p> {
        TreeLayouter {
            ctx,
            stack: StackLayouter::new(StackContext { space: ctx.space }),
            flex: FlexLayouter::new(FlexContext {
                space: LayoutSpace {
                    dimensions: ctx.space.usable(),
                    padding: SizeBox::zero(),
                    alignment: ctx.space.alignment,
                    shrink_to_fit: true,
                },
                flex_spacing: flex_spacing(&ctx.style),
            }),
            style: Cow::Borrowed(ctx.style),
        }
    }

    /// Layout the tree into a box.
    fn layout(&mut self, tree: &SyntaxTree) -> LayoutResult<()> {
        for node in &tree.nodes {
            match node {
                Node::Text(text) => self.layout_text(text, false)?,

                Node::Space => {
                    // Only add a space if there was any content before.
                    if !self.flex.is_empty() {
                        self.layout_text(" ", true)?;
                    }
                }

                // Finish the current flex layouting process.
                Node::Newline => {
                    self.layout_flex()?;

                    let space = paragraph_spacing(&self.style);
                    self.stack.add_space(space)?;
                }

                // Toggle the text styles.
                Node::ToggleItalics => self.style.to_mut().toggle_class(FontClass::Italic),
                Node::ToggleBold => self.style.to_mut().toggle_class(FontClass::Bold),
                Node::ToggleMonospace => self.style.to_mut().toggle_class(FontClass::Monospace),

                Node::Func(func) => self.layout_func(func)?,
            }
        }

        Ok(())
    }

    /// Finish the layout.
    fn finish(mut self) -> LayoutResult<MultiLayout> {
        // If there are remainings, add them to the layout.
        if !self.flex.is_empty() {
            self.layout_flex()?;
        }

        Ok(MultiLayout {
            layouts: vec![self.stack.finish()],
        })
    }

    /// Add text to the flex layout. If `glue` is true, the text will be a glue
    /// part in the flex layouter. For details, see [`FlexLayouter`].
    fn layout_text(&mut self, text: &str, glue: bool) -> LayoutResult<()> {
        let ctx = TextContext {
            loader: &self.ctx.loader,
            style: &self.style,
        };

        let layout = layout_text(text, ctx)?;

        if glue {
            self.flex.add_glue(layout);
        } else {
            self.flex.add(layout);
        }

        Ok(())
    }

    /// Finish the current flex layout and add it the stack.
    fn layout_flex(&mut self) -> LayoutResult<()> {
        if self.flex.is_empty() {
            return Ok(());
        }

        let mut ctx = self.flex.ctx();
        ctx.space.dimensions = self.stack.remaining();
        ctx.flex_spacing = flex_spacing(&self.style);

        let next = FlexLayouter::new(ctx);
        let flex = std::mem::replace(&mut self.flex, next);
        let boxed = flex.finish()?;

        self.stack.add(boxed)
    }

    /// Layout a function.
    fn layout_func(&mut self, func: &FuncCall) -> LayoutResult<()> {
        let mut ctx = self.ctx;
        ctx.style = &self.style;
        ctx.space.dimensions = self.stack.remaining();
        ctx.space.padding = SizeBox::zero();
        ctx.space.shrink_to_fit = true;

        let commands = func.body.layout(ctx)?;

        for command in commands {
            match command {
                Command::Layout(tree) => self.layout(tree)?,
                Command::Add(layout) => self.stack.add(layout)?,
                Command::AddMany(layouts) => self.stack.add_many(layouts)?,
                Command::ToggleStyleClass(class) => self.style.to_mut().toggle_class(class),
            }
        }

        Ok(())
    }
}

fn flex_spacing(style: &TextStyle) -> Size {
    (style.line_spacing - 1.0) * Size::pt(style.font_size)
}

fn paragraph_spacing(style: &TextStyle) -> Size {
    let line_height = Size::pt(style.font_size);
    let space_factor = style.line_spacing * style.paragraph_spacing - 1.0;
    line_height * space_factor
}
