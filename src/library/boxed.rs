use crate::func::prelude::*;
use super::maps::ExtentMap;

function! {
    /// `box`: Layouts content into a box.
    #[derive(Debug, PartialEq)]
    pub struct Boxed {
        body: SyntaxTree,
        map: ExtentMap<PSize>,
        debug: bool,
    }

    parse(args, body, ctx) {
        Boxed {
            body: parse!(optional: body, ctx).unwrap_or(SyntaxTree::new()),
            map: ExtentMap::new(&mut args, false)?,
            debug: args.get_key_opt::<bool>("debug")?
                .map(Spanned::value)
                .unwrap_or(true),
        }
    }

    layout(self, mut ctx) {
        use SpecificAxisKind::*;

        ctx.debug = self.debug;
        let space = &mut ctx.spaces[0];

        self.map.apply_with(ctx.axes, |axis, p| {
            let entity = match axis {
                Horizontal => { space.expand.horizontal = true; &mut space.dimensions.x },
                Vertical => { space.expand.vertical = true; &mut space.dimensions.y },
            };

            *entity = p.concretize(*entity)
        })?;

        vec![AddMultiple(layout_tree(&self.body, ctx)?)]
    }
}
