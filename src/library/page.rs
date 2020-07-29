use crate::length::Length;
use crate::paper::{Paper, PaperClass};
use super::*;

function! {
    /// `page.size`: Set the size of pages.
    #[derive(Debug, Clone, PartialEq)]
    pub struct PageSizeFunc {
        paper: Option<Paper>,
        extents: AxisMap<Length>,
        flip: bool,
    }

    parse(header, body, state, f) {
        body!(nope: body, f);
        PageSizeFunc {
            paper: header.args.pos.get::<Paper>(&mut f.diagnostics),
            extents: AxisMap::parse::<ExtentKey>(&mut f.diagnostics, &mut header.args.key),
            flip: header.args.key.get::<bool>(&mut f.diagnostics, "flip").unwrap_or(false),
        }
    }

    layout(self, ctx, f) {
        let mut style = ctx.style.page;

        if let Some(paper) = self.paper {
            style.class = paper.class;
            style.dimensions = paper.size();
        } else {
            style.class = PaperClass::Custom;
        }

        let map = self.extents.dedup(&mut f.diagnostics, ctx.axes);
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

    parse(header, body, state, f) {
        body!(nope: body, f);
        PageMarginsFunc {
            padding: PaddingMap::parse(&mut f.diagnostics, &mut header.args),
        }
    }

    layout(self, ctx, f) {
        let mut style = ctx.style.page;
        self.padding.apply(&mut f.diagnostics, ctx.axes, &mut style.margins);
        vec![SetPageStyle(style)]
    }
}
