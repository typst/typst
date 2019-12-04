use crate::func::prelude::*;

function! {
    /// `box`: Layouts content into a box.
    #[derive(Debug, PartialEq)]
    pub struct Boxed {
        body: SyntaxTree,
        map: ArgMap<AxisKey, Size>,
    }

    parse(args, body, ctx) {
        let mut map = ArgMap::new();

        for arg in args.keys() {
            let key = match arg.val.0.val {
                "width" | "w" => AxisKey::Horizontal,
                "height" | "h" => AxisKey::Vertical,
                "primary-size" => AxisKey::Primary,
                "secondary-size" => AxisKey::Secondary,
                _ => pr!("unexpected argument"),
            };

            let size = ArgParser::convert::<ArgSize>(arg.val.1.val)?;
            map.add(key, size);
        }

        Boxed {
            body: parse!(expected: body, ctx),
            map,
        }
    }

    layout(self, mut ctx) {
        let map = self.map.dedup(|key, val| {
            Ok((key.specific(ctx.axes), val))
        });

        let mut dimensions = &mut ctx.spaces[0].dimensions;
        map.with(AxisKey::Horizontal, |val| dimensions.x = val);
        map.with(AxisKey::Vertical, |val| dimensions.y = val);

        vec![AddMultiple(layout_tree(&self.body, ctx)?)]
    }
}
