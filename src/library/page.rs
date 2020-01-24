use crate::size::Size;
use crate::style::{Paper, PaperClass};
use crate::syntax::func::maps::{AxisMap, PaddingMap};
use super::*;


function! {
    /// `page.size`: Set the size of pages.
    #[derive(Debug, Clone, PartialEq)]
    pub struct PageSizeFunc {
        paper: Option<Paper>,
        extents: AxisMap<Size>,
        flip: bool,
    }

    parse(header, body, ctx, errors, decos) {
        body!(nope: body, errors);
        PageSizeFunc {
            paper: header.args.pos.get::<Paper>(errors),
            extents: AxisMap::parse::<ExtentKey, Size>(errors, &mut header.args.key),
            flip: header.args.key.get::<bool>(errors, "flip").unwrap_or(false),
        }
    }

    layout(self, ctx, errors) {
        let mut style = ctx.style.page;

        if let Some(paper) = self.paper {
            style.class = paper.class;
            style.dimensions = paper.dimensions;
        } else {
            style.class = PaperClass::Custom;
        }

        let map = self.extents.dedup(errors, ctx.axes);
        map.with(Horizontal, |&width| style.dimensions.x = width);
        map.with(Vertical, |&height| style.dimensions.y = height);

        if self.flip {
            style.dimensions.swap();
        }

        vec![SetPageStyle(style)]
    }
}

function! {
    /// `page.margins`: Sets the page margins.
    #[derive(Debug, Clone, PartialEq)]
    pub struct PageMarginsFunc {
        padding: PaddingMap,
    }

    parse(header, body, ctx, errors, decos) {
        body!(nope: body, errors);
        PageMarginsFunc {
            padding: PaddingMap::parse(errors, &mut header.args),
        }
    }

    layout(self, ctx, errors) {
        let mut style = ctx.style.page;
        self.padding.apply(errors, ctx.axes, &mut style.margins);
        vec![SetPageStyle(style)]
    }
}
