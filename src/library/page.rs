use std::mem;

use crate::geom::{Length, Linear};
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
pub fn page(mut args: Args, ctx: &mut EvalContext) -> Value {
    if let Some(paper) = args.find::<Paper>() {
        ctx.state.page.class = paper.class;
        ctx.state.page.size = paper.size();
    }

    if let Some(width) = args.get::<_, Length>(ctx, "width") {
        ctx.state.page.class = PaperClass::Custom;
        ctx.state.page.size.width = width;
    }

    if let Some(height) = args.get::<_, Length>(ctx, "height") {
        ctx.state.page.class = PaperClass::Custom;
        ctx.state.page.size.height = height;
    }

    if let Some(margins) = args.get::<_, Linear>(ctx, "margins") {
        ctx.state.page.margins = Sides::uniform(Some(margins));
    }

    if let Some(left) = args.get::<_, Linear>(ctx, "left") {
        ctx.state.page.margins.left = Some(left);
    }

    if let Some(top) = args.get::<_, Linear>(ctx, "top") {
        ctx.state.page.margins.top = Some(top);
    }

    if let Some(right) = args.get::<_, Linear>(ctx, "right") {
        ctx.state.page.margins.right = Some(right);
    }

    if let Some(bottom) = args.get::<_, Linear>(ctx, "bottom") {
        ctx.state.page.margins.bottom = Some(bottom);
    }

    if args.get::<_, bool>(ctx, "flip").unwrap_or(false) {
        let size = &mut ctx.state.page.size;
        mem::swap(&mut size.width, &mut size.height);
    }

    args.done(ctx);

    ctx.end_page_group();
    ctx.start_page_group(false);

    Value::None
}

/// `pagebreak`: Starts a new page.
pub fn pagebreak(args: Args, ctx: &mut EvalContext) -> Value {
    args.done(ctx);
    ctx.end_page_group();
    ctx.start_page_group(true);
    Value::None
}
