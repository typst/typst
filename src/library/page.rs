use crate::length::Length;
use crate::paper::{Paper, PaperClass};
use super::*;

function! {
    /// `page`: Configure pages.
    #[derive(Debug, Clone, PartialEq)]
    pub struct PageFunc {
        paper: Option<Paper>,
        extents: AxisMap<Length>,
        padding: PaddingMap,
        flip: bool,
    }

    parse(header, body, state, f) {
        body!(nope: body, f);
        PageFunc {
            paper: header.args.pos.get::<Paper>(&mut f.diagnostics),
            extents: AxisMap::parse::<ExtentKey>(&mut f.diagnostics, &mut header.args.key),
            padding: PaddingMap::parse(&mut f.diagnostics, &mut header.args),
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
        map.with(Horizontal, |&width| style.dimensions.x = width.as_raw());
        map.with(Vertical, |&height| style.dimensions.y = height.as_raw());

        if self.flip {
            style.dimensions.swap();
        }

        self.padding.apply(&mut f.diagnostics, ctx.axes, &mut style.margins);

        vec![SetPageStyle(style)]
    }
}
