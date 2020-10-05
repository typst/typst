use std::mem;

use crate::eval::Absolute;
use crate::geom::Linear;
use crate::paper::{Paper, PaperClass};
use crate::prelude::*;

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
pub async fn page(mut args: Args, ctx: &mut LayoutContext) -> Value {
    let mut page = ctx.state.page.clone();

    if let Some(paper) = args.find::<Paper>() {
        page.class = paper.class;
        page.size = paper.size();
    }

    if let Some(Absolute(width)) = args.get::<_, Absolute>(ctx, "width") {
        page.class = PaperClass::Custom;
        page.size.width = width;
    }

    if let Some(Absolute(height)) = args.get::<_, Absolute>(ctx, "height") {
        page.class = PaperClass::Custom;
        page.size.height = height;
    }

    if let Some(margins) = args.get::<_, Linear>(ctx, "margins") {
        page.margins = Sides::uniform(Some(margins));
    }

    if let Some(left) = args.get::<_, Linear>(ctx, "left") {
        page.margins.left = Some(left);
    }

    if let Some(top) = args.get::<_, Linear>(ctx, "top") {
        page.margins.top = Some(top);
    }

    if let Some(right) = args.get::<_, Linear>(ctx, "right") {
        page.margins.right = Some(right);
    }

    if let Some(bottom) = args.get::<_, Linear>(ctx, "bottom") {
        page.margins.bottom = Some(bottom);
    }

    if args.get::<_, bool>(ctx, "flip").unwrap_or(false) {
        mem::swap(&mut page.size.width, &mut page.size.height);
    }

    args.done(ctx);
    Value::Commands(vec![SetPageState(page)])
}

/// `pagebreak`: Ends the current page.
pub async fn pagebreak(args: Args, ctx: &mut LayoutContext) -> Value {
    args.done(ctx);
    Value::Commands(vec![BreakPage])
}
