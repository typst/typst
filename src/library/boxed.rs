use smallvec::smallvec;

use crate::func::prelude::*;
use super::maps::ExtentMap;


function! {
    /// `box`: Layouts content into a box.
    #[derive(Debug, PartialEq)]
    pub struct BoxFunc {
        body: SyntaxTree,
        map: ExtentMap<PSize>,
        debug: Option<bool>,
    }

    parse(args, body, ctx) {
        BoxFunc {
            body: parse!(optional: body, ctx).unwrap_or(SyntaxTree::new()),
            map: ExtentMap::new(&mut args, false)?,
            debug: args.get_key_opt::<bool>("debug")?,
        }
    }

    layout(self, mut ctx) {
        ctx.repeat = false;

        if let Some(debug) = self.debug {
            ctx.debug = debug;
        }

        let map = self.map.dedup(ctx.axes)?;

        // Try to layout this box in all spaces until it fits into some space.
        let mut error = None;
        for &(mut space) in &ctx.spaces {
            let mut ctx = ctx.clone();

            for &axis in &[Horizontal, Vertical] {
                if let Some(psize) = map.get(axis) {
                    let size = psize.scaled(ctx.base.get(axis));
                    *ctx.base.get_mut(axis) = size;
                    *space.dimensions.get_mut(axis) = size;
                    *space.expansion.get_mut(axis) = true;
                }
            }

            ctx.spaces = smallvec![space];

            match layout(&self.body, ctx).await {
                Ok(layouts) => return Ok(vec![AddMultiple(layouts)]),
                Err(err) => error = Some(err),
            }
        }

        return Err(error.expect("expected at least one space"));
    }
}
