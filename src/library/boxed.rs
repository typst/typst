use crate::func::prelude::*;
use super::maps::ExtentMap;

function! {
    /// `box`: Layouts content into a box.
    #[derive(Debug, PartialEq)]
    pub struct Boxed {
        body: SyntaxTree,
        map: ExtentMap,
    }

    parse(args, body, ctx) {
        Boxed {
            body: parse!(expected: body, ctx),
            map: ExtentMap::new(&mut args, false)?,
        }
    }

    layout(self, mut ctx) {
        self.map.apply(ctx.axes, &mut ctx.spaces[0].dimensions)?;
        vec![AddMultiple(layout_tree(&self.body, ctx)?)]
    }
}
