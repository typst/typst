use std::mem;

use super::*;
use crate::eval::Absolute;
use crate::geom::{Linear, Sides};
use crate::paper::{Paper, PaperClass};

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
pub async fn page(mut args: ValueDict, ctx: &mut LayoutContext) -> Value {
    let mut page = ctx.state.page.clone();

    if let Some(paper) = args.take::<Paper>() {
        page.class = paper.class;
        page.size = paper.size();
    }

    if let Some(Absolute(width)) = args.take_key::<Absolute>("width", &mut ctx.f) {
        page.class = PaperClass::Custom;
        page.size.width = width;
    }

    if let Some(Absolute(height)) = args.take_key::<Absolute>("height", &mut ctx.f) {
        page.class = PaperClass::Custom;
        page.size.height = height;
    }

    if let Some(margins) = args.take_key::<Linear>("margins", &mut ctx.f) {
        page.margins = Sides::uniform(Some(margins));
    }

    if let Some(left) = args.take_key::<Linear>("left", &mut ctx.f) {
        page.margins.left = Some(left);
    }

    if let Some(top) = args.take_key::<Linear>("top", &mut ctx.f) {
        page.margins.top = Some(top);
    }

    if let Some(right) = args.take_key::<Linear>("right", &mut ctx.f) {
        page.margins.right = Some(right);
    }

    if let Some(bottom) = args.take_key::<Linear>("bottom", &mut ctx.f) {
        page.margins.bottom = Some(bottom);
    }

    if args.take_key::<bool>("flip", &mut ctx.f).unwrap_or(false) {
        mem::swap(&mut page.size.width, &mut page.size.height);
    }

    args.unexpected(&mut ctx.f);
    Value::Commands(vec![SetPageState(page)])
}

/// `pagebreak`: Ends the current page.
pub async fn pagebreak(args: ValueDict, ctx: &mut LayoutContext) -> Value {
    args.unexpected(&mut ctx.f);
    Value::Commands(vec![BreakPage])
}
