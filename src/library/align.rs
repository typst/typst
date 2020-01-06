use crate::func::prelude::*;
use super::maps::{PosAxisMap, AlignmentKey};


function! {
    /// `align`: Aligns content along the layouting axes.
    #[derive(Debug, PartialEq)]
    pub struct AlignFunc {
        body: Option<SyntaxTree>,
        map: PosAxisMap<AlignmentKey>,
    }

    parse(args, body, ctx) {
        AlignFunc {
            body: parse!(optional: body, ctx),
            map: PosAxisMap::new(&mut args)?,
        }
    }

    layout(self, mut ctx) {
        ctx.base = ctx.spaces[0].dimensions;

        let map = self.map.dedup(ctx.axes, |alignment| alignment.axis(ctx.axes))?;
        for &axis in &[Primary, Secondary] {
            if let Some(alignment) = map.get(axis) {
                *ctx.alignment.get_mut(axis) = alignment.to_generic(ctx.axes, axis)?;
            }
        }

        match &self.body {
            Some(body) => vec![AddMultiple(layout(&body, ctx).await?)],
            None => vec![SetAlignment(ctx.alignment)],
        }
    }
}
