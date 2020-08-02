use crate::length::{Length, ScaleLength};
use crate::paper::{Paper, PaperClass};
use super::*;

function! {
    /// `page`: Configure pages.
    #[derive(Debug, Clone, PartialEq)]
    pub struct PageFunc {
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

    parse(header, body, state, f) {
        body!(nope: body, f);
        PageFunc {
            paper: header.args.pos.get::<Paper>(),
            width: header.args.key.get::<Length>("width", f),
            height: header.args.key.get::<Length>("height", f),
            margins: header.args.key.get::<ScaleLength>("margins", f),
            left: header.args.key.get::<ScaleLength>("left", f),
            right: header.args.key.get::<ScaleLength>("right", f),
            top: header.args.key.get::<ScaleLength>("top", f),
            bottom: header.args.key.get::<ScaleLength>("bottom", f),
            flip: header.args.key.get::<bool>("flip", f).unwrap_or(false),
        }
    }

    layout(self, ctx, f) {
        let mut style = ctx.style.page;

        if let Some(paper) = self.paper {
            style.class = paper.class;
            style.dimensions = paper.size();
        } else if self.width.is_some() || self.height.is_some() {
            style.class = PaperClass::Custom;
        }

        self.width.with(|v| style.dimensions.x = v.as_raw());
        self.height.with(|v| style.dimensions.y = v.as_raw());
        self.margins.with(|v| style.margins.set_all(Some(v)));
        self.left.with(|v| style.margins.left = Some(v));
        self.right.with(|v| style.margins.right = Some(v));
        self.top.with(|v| style.margins.top = Some(v));
        self.bottom.with(|v| style.margins.bottom = Some(v));

        if self.flip {
            style.dimensions.swap();
        }

        vec![SetPageStyle(style)]
    }
}
