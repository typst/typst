use super::*;

/// Layouts syntax trees into boxes.
pub fn layout_tree(tree: &SyntaxTree, ctx: LayoutContext) -> LayoutResult<MultiLayout> {
    let mut layouter = TreeLayouter::new(ctx);
    layouter.layout(tree)?;
    layouter.finish()
}

#[derive(Debug, Clone)]
struct TreeLayouter<'a, 'p> {
    ctx: LayoutContext<'a, 'p>,
    stack: StackLayouter,
    flex: FlexLayouter,
    style: Cow<'a, TextStyle>,
    alignment: Alignment,
    set_newline: bool,
}

impl<'a, 'p> TreeLayouter<'a, 'p> {
    /// Create a new layouter.
    fn new(ctx: LayoutContext<'a, 'p>) -> TreeLayouter<'a, 'p> {
        TreeLayouter {
            ctx,
            stack: StackLayouter::new(StackContext::from_layout_ctx(ctx)),
            flex: FlexLayouter::new(FlexContext {
                space: ctx.space.usable_space(),
                followup_spaces: ctx.followup_spaces.map(|s| s.usable_space()),
                shrink_to_fit: true,
                .. FlexContext::from_layout_ctx(ctx, flex_spacing(&ctx.style))
            }),
            style: Cow::Borrowed(ctx.style),
            alignment: ctx.alignment,
            set_newline: false,
        }
    }

    /// Layout the tree into a box.
    fn layout(&mut self, tree: &SyntaxTree) -> LayoutResult<()> {
        for node in &tree.nodes {
            match node {
                Node::Text(text) => {
                    let layout = self.layout_text(text)?;
                    self.flex.add(layout);
                    self.set_newline = true;
                }

                Node::Space => {
                    // Only add a space if there was any content before.
                    if !self.flex.is_empty() {
                        let layout = self.layout_text(" ")?;
                        self.flex.add_glue(layout.dimensions);
                    }
                }

                // Finish the current flex layouting process.
                Node::Newline => {
                    self.finish_flex()?;

                    if self.set_newline {
                        let space = paragraph_spacing(&self.style);
                        self.stack.add_space(space)?;
                        self.set_newline = false;
                    }

                    self.start_new_flex();
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
        self.finish_flex()?;
        self.stack.finish()
    }

    /// Layout a function.
    fn layout_func(&mut self, func: &FuncCall) -> LayoutResult<()> {
        // Finish the current flex layout on a copy to find out how
        // much space would be remaining if we finished.

        let mut lookahead_stack = self.stack.clone();
        let layouts = self.flex.clone().finish()?;
        lookahead_stack.add_many(layouts)?;
        let remaining = lookahead_stack.remaining();

        let mut ctx = self.ctx;
        ctx.style = &self.style;
        ctx.flow = Flow::Vertical;
        ctx.shrink_to_fit = true;
        ctx.space.dimensions = remaining;
        ctx.space.padding = SizeBox::zero();
        if let Some(space) = ctx.followup_spaces.as_mut() {
            *space = space.usable_space();
        }

        let commands = func.body.layout(ctx)?;

        for command in commands {
            match command {
                Command::Layout(tree) => self.layout(tree)?,

                Command::Add(layout) => {
                    self.finish_flex()?;
                    self.stack.add(layout)?;
                    self.set_newline = true;
                    self.start_new_flex();
                }

                Command::AddMany(layouts) => {
                    self.finish_flex()?;
                    self.stack.add_many(layouts)?;
                    self.set_newline = true;
                    self.start_new_flex();
                }

                Command::SetAlignment(alignment) => {
                    self.finish_flex()?;
                    self.alignment = alignment;
                    self.start_new_flex();
                }

                Command::SetStyle(style) => *self.style.to_mut() = style,

                Command::FinishLayout => {
                    self.finish_flex()?;
                    self.stack.finish_layout(true)?;
                    self.start_new_flex();
                }

                Command::FinishFlexRun => self.flex.add_break(),
            }
        }

        Ok(())
    }

    /// Add text to the flex layout. If `glue` is true, the text will be a glue
    /// part in the flex layouter. For details, see [`FlexLayouter`].
    fn layout_text(&mut self, text: &str) -> LayoutResult<Layout> {
        let ctx = TextContext {
            loader: &self.ctx.loader,
            style: &self.style,
        };

        layout_text(text, ctx)
    }

    /// Finish the current flex layout and add it the stack.
    fn finish_flex(&mut self) -> LayoutResult<()> {
        if self.flex.is_empty() {
            return Ok(());
        }

        let layouts = self.flex.finish()?;
        self.stack.add_many(layouts)?;

        Ok(())
    }

    /// Start a new flex layout.
    fn start_new_flex(&mut self) {
        let mut ctx = self.flex.ctx();
        ctx.space.dimensions = self.stack.remaining();
        ctx.alignment = self.alignment;
        ctx.flex_spacing = flex_spacing(&self.style);

        self.flex = FlexLayouter::new(ctx);
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
