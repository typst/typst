use super::*;
use crate::length::{Length, ScaleLength};
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
pub async fn page(_: Span, mut args: DictValue, ctx: LayoutContext<'_>) -> Pass<Value> {
    let mut f = Feedback::new();
    let mut style = ctx.style.page;

    if let Some(paper) = args.take::<Paper>() {
        style.class = paper.class;
        style.size = paper.size();
    }

    if let Some(width) = args.take_key::<Length>("width", &mut f) {
        style.class = PaperClass::Custom;
        style.size.x = width.as_raw();
    }

    if let Some(height) = args.take_key::<Length>("height", &mut f) {
        style.class = PaperClass::Custom;
        style.size.y = height.as_raw();
    }

    if let Some(margins) = args.take_key::<ScaleLength>("margins", &mut f) {
        style.margins.set_all(Some(margins));
    }

    if let Some(left) = args.take_key::<ScaleLength>("left", &mut f) {
        style.margins.left = Some(left);
    }

    if let Some(right) = args.take_key::<ScaleLength>("right", &mut f) {
        style.margins.right = Some(right);
    }

    if let Some(top) = args.take_key::<ScaleLength>("top", &mut f) {
        style.margins.top = Some(top);
    }

    if let Some(bottom) = args.take_key::<ScaleLength>("bottom", &mut f) {
        style.margins.bottom = Some(bottom);
    }

    if args.take_key::<bool>("flip", &mut f).unwrap_or(false) {
        style.size.swap();
    }

    args.unexpected(&mut f);
    Pass::commands(vec![SetPageStyle(style)], f)
}

/// `pagebreak`: Ends the current page.
pub async fn pagebreak(_: Span, args: DictValue, _: LayoutContext<'_>) -> Pass<Value> {
    let mut f = Feedback::new();
    args.unexpected(&mut f);
    Pass::commands(vec![BreakPage], f)
}
