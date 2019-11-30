use crate::func::prelude::*;

/// `page.break`: Ends the current page.
#[derive(Debug, PartialEq)]
pub struct PageBreak;

function! {
    data: PageBreak,
    parse: plain,
    layout(_, _) { Ok(vec![FinishSpace]) }
}

/// `page.size`: Set the size of pages.
#[derive(Debug, PartialEq)]
pub struct PageSize {
    width: Option<Size>,
    height: Option<Size>,
}

function! {
    data: PageSize,

    parse(args, body, _ctx) {
        parse!(forbidden: body);
        Ok(PageSize {
            width: args.get_key_opt::<ArgSize>("width")?.map(|a| a.val),
            height: args.get_key_opt::<ArgSize>("height")?.map(|a| a.val),
        })
    }

    layout(this, ctx) {
        let mut style = ctx.style.page;

        if let Some(width) = this.width { style.dimensions.x = width; }
        if let Some(height) = this.height { style.dimensions.y = height; }

        Ok(vec![SetPageStyle(style)])
    }
}

/// `page.margins`: Set the margins of pages.
#[derive(Debug, PartialEq)]
pub struct PageMargins {
    left: Option<Size>,
    top: Option<Size>,
    right: Option<Size>,
    bottom: Option<Size>,
}

function! {
    data: PageMargins,

    parse(args, body, _ctx) {
        parse!(forbidden: body);
        let default = args.get_pos_opt::<ArgSize>()?;
        let mut get = |which| {
            args.get_key_opt::<ArgSize>(which)
                .map(|size| size.or(default).map(|a| a.val))
        };

        Ok(PageMargins {
            left: get("left")?,
            top: get("top")?,
            right: get("right")?,
            bottom: get("bottom")?,
        })
    }

    layout(this, ctx) {
        let mut style = ctx.style.page;

        if let Some(left) = this.left { style.margins.left = left; }
        if let Some(top) = this.top { style.margins.top = top; }
        if let Some(right) = this.right { style.margins.right = right; }
        if let Some(bottom) = this.bottom { style.margins.bottom = bottom; }

        Ok(vec![SetPageStyle(style)])
    }
}
