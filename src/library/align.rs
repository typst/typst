use crate::func::prelude::*;

function! {
    /// `align`: Aligns content along the layouting axes.
    #[derive(Debug, PartialEq)]
    pub struct Align {
        body: Option<SyntaxTree>,
        map: ArgMap<Key, AlignmentKey>,
    }

    parse(args, body, ctx) {
        let mut map = ArgMap::new();
        map.put(Key::First, args.get_pos_opt::<ArgIdent>()?)?;
        map.put(Key::Second, args.get_pos_opt::<ArgIdent>()?)?;

        for arg in args.keys() {
            let key = match arg.val.0.val {
                "horizontal" => Key::Axis(AxisKey::Horizontal),
                "vertical" => Key::Axis(AxisKey::Vertical),
                "primary" => Key::Axis(AxisKey::Primary),
                "secondary" => Key::Axis(AxisKey::Secondary),
                _ => error!(unexpected_argument),
            };

            let value = AlignmentKey::parse(arg.val.1.val)?;
            map.add(key, value);
        }

        Align {
            body: parse!(optional: body, ctx),
            map,
        }
    }

    layout(self, mut ctx) {
        let axes = ctx.axes;
        let basic = axes.primary.is_horizontal();

        let map = self.map.dedup(|key, val| {
            let axis = match key {
                Key::First => val.axis(axes, GenericAxisKind::Primary),
                Key::Second => val.axis(axes, GenericAxisKind::Secondary),
                Key::Axis(AxisKey::Primary) => GenericAxisKind::Primary,
                Key::Axis(AxisKey::Secondary) => GenericAxisKind::Secondary,
                Key::Axis(AxisKey::Horizontal) => axes.horizontal(),
                Key::Axis(AxisKey::Vertical) => axes.vertical(),
            };

            let alignment = val.generic(axes, axis)?;
            Ok((key, alignment))
        })?;

        map.with(GenericAxisKind::Primary, |val| ctx.alignment.primary = val);
        map.with(GenericAxisKind::Secondary, |val| ctx.alignment.secondary = val);

        match &self.body {
            Some(body) => vec![AddMultiple(layout_tree(&body, ctx)?)],
            None => vec![Command::SetAlignment(ctx.alignment)],
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
enum Key {
    First,
    Second,
    Axis(AxisKey),
}
