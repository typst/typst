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
pub async fn page(mut args: TableValue, ctx: LayoutContext<'_>) -> Pass<Value> {
    let mut f = Feedback::new();
    let paper = args.take::<Paper>();
    let width = args.take_with_key::<_, Length>("width", &mut f);
    let height = args.take_with_key::<_, Length>("height", &mut f);
    let margins = args.take_with_key::<_, ScaleLength>("margins", &mut f);
    let left = args.take_with_key::<_, ScaleLength>("left", &mut f);
    let right = args.take_with_key::<_, ScaleLength>("right", &mut f);
    let top = args.take_with_key::<_, ScaleLength>("top", &mut f);
    let bottom = args.take_with_key::<_, ScaleLength>("bottom", &mut f);
    let flip = args.take_with_key::<_, bool>("flip", &mut f).unwrap_or(false);
    args.unexpected(&mut f);

    let mut style = ctx.style.page;

    if let Some(paper) = paper {
        style.class = paper.class;
        style.size = paper.size();
    } else if width.is_some() || height.is_some() {
        style.class = PaperClass::Custom;
    }

    width.with(|v| style.size.x = v.as_raw());
    height.with(|v| style.size.y = v.as_raw());
    margins.with(|v| style.margins.set_all(Some(v)));
    left.with(|v| style.margins.left = Some(v));
    right.with(|v| style.margins.right = Some(v));
    top.with(|v| style.margins.top = Some(v));
    bottom.with(|v| style.margins.bottom = Some(v));

    if flip {
        style.size.swap();
    }

    Pass::commands(vec![SetPageStyle(style)], f)
}

/// `pagebreak`: Ends the current page.
pub async fn pagebreak(args: TableValue, _: LayoutContext<'_>) -> Pass<Value> {
    let mut f = Feedback::new();
    args.unexpected(&mut f);
    Pass::commands(vec![BreakPage], f)
}
