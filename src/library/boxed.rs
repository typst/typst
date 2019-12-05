use crate::func::prelude::*;
use super::keys::*;

function! {
    /// `box`: Layouts content into a box.
    #[derive(Debug, PartialEq)]
    pub struct Boxed {
        body: SyntaxTree,
        map: ConsistentMap<AxisKey, Size>,
    }

    parse(args, body, ctx) {
        let mut map = ConsistentMap::new();

        for arg in args.keys() {
            let key = match arg.v.key.v.0.as_str() {
                "width" | "w" => AxisKey::Horizontal,
                "height" | "h" => AxisKey::Vertical,
                "primary-size" => AxisKey::Primary,
                "secondary-size" => AxisKey::Secondary,
                _ => error!(unexpected_argument),
            };

            let size = Size::from_expr(arg.v.value)?;
            map.add(key, size)?;
        }

        Boxed {
            body: parse!(expected: body, ctx),
            map,
        }
    }

    layout(self, mut ctx) {
        let map = self.map.dedup(|key, val| Ok((key.specific(ctx.axes), val)))?;

        let dimensions = &mut ctx.spaces[0].dimensions;
        map.with(SpecificAxisKind::Horizontal, |&val| dimensions.x = val);
        map.with(SpecificAxisKind::Vertical, |&val| dimensions.y = val);

        vec![AddMultiple(layout_tree(&self.body, ctx)?)]
    }
}
