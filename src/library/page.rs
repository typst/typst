use crate::length::{Length, ScaleLength};
use crate::paper::{Paper, PaperClass};
use super::*;

/// `page`: Configure pages.
///
/// # Positional arguments
/// - The name of a paper, e.g. `a4` (optional).
///
/// # Keyword arguments
/// - `width`: The width of pages (length).
/// - `height`: The height of pages (length).
/// - `margins`: The margins for all sides (length or relative to side lengths).
/// - `left`: The left margin (length or relative to width).
/// - `right`: The right margin (length or relative to width).
/// - `top`: The top margin (length or relative to height).
/// - `bottom`: The bottom margin (length or relative to height).
/// - `flip`: Flips custom or paper-defined width and height (boolean).
pub fn page(call: FuncCall, _: &ParseState) -> Pass<SyntaxNode> {
    let mut f = Feedback::new();
    let mut args = call.args;
    let node = PageNode {
        paper: args.take::<Paper>(),
        width: args.take_with_key::<_, Length>("width", &mut f),
        height: args.take_with_key::<_, Length>("height", &mut f),
        margins: args.take_with_key::<_, ScaleLength>("margins", &mut f),
        left: args.take_with_key::<_, ScaleLength>("left", &mut f),
        right: args.take_with_key::<_, ScaleLength>("right", &mut f),
        top: args.take_with_key::<_, ScaleLength>("top", &mut f),
        bottom: args.take_with_key::<_, ScaleLength>("bottom", &mut f),
        flip: args.take_with_key::<_, bool>("flip", &mut f).unwrap_or(false),
    };
    args.unexpected(&mut f);
    Pass::node(node, f)
}

#[derive(Debug, Clone, PartialEq)]
struct PageNode {
    paper: Option<Paper>,
    width: Option<Length>,
    height: Option<Length>,
    margins: Option<ScaleLength>,
    left: Option<ScaleLength>,
    right: Option<ScaleLength>,
    top: Option<ScaleLength>,
    bottom: Option<ScaleLength>,
    flip: bool,
}

#[async_trait(?Send)]
impl Layout for PageNode {
    async fn layout<'a>(&'a self, ctx: LayoutContext<'_>) -> Pass<Commands<'a>> {
        let mut style = ctx.style.page;

        if let Some(paper) = self.paper {
            style.class = paper.class;
            style.size = paper.size();
        } else if self.width.is_some() || self.height.is_some() {
            style.class = PaperClass::Custom;
        }

        self.width.with(|v| style.size.x = v.as_raw());
        self.height.with(|v| style.size.y = v.as_raw());
        self.margins.with(|v| style.margins.set_all(Some(v)));
        self.left.with(|v| style.margins.left = Some(v));
        self.right.with(|v| style.margins.right = Some(v));
        self.top.with(|v| style.margins.top = Some(v));
        self.bottom.with(|v| style.margins.bottom = Some(v));

        if self.flip {
            style.size.swap();
        }

        Pass::okay(vec![SetPageStyle(style)])
    }
}

/// `pagebreak`: Ends the current page.
pub fn pagebreak(call: FuncCall, _: &ParseState) -> Pass<SyntaxNode> {
    let mut f = Feedback::new();
    call.args.unexpected(&mut f);
    Pass::node(PageBreakNode, f)
}

#[derive(Debug, Default, Clone, PartialEq)]
struct PageBreakNode;

#[async_trait(?Send)]
impl Layout for PageBreakNode {
    async fn layout<'a>(&'a self, _: LayoutContext<'_>) -> Pass<Commands<'a>> {
        Pass::okay(vec![BreakPage])
    }
}
