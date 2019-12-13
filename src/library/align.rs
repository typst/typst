use crate::func::prelude::*;
use super::maps::ConsistentMap;
use super::keys::{AxisKey, AlignmentKey};

function! {
    /// `align`: Aligns content along the layouting axes.
    #[derive(Debug, PartialEq)]
    pub struct Align {
        body: Option<SyntaxTree>,
        map: ConsistentMap<Key, AlignmentKey>,
    }

    parse(args, body, ctx) {
        let mut map = ConsistentMap::new();

        map.add_opt(Key::First, args.get_pos_opt::<AlignmentKey>()?)?;
        map.add_opt(Key::Second, args.get_pos_opt::<AlignmentKey>()?)?;

        for arg in args.keys() {
            let axis = AxisKey::from_ident(&arg.v.key)?;
            let value = AlignmentKey::from_expr(arg.v.value)?;

            map.add(Key::Axis(axis), value)?;
        }

        Align {
            body: parse!(optional: body, ctx),
            map,
        }
    }

    layout(self, mut ctx) {
        let axes = ctx.axes;

        let map = self.map.dedup(|key, alignment| {
            let axis = match key {
                Key::First => alignment.axis(axes, Primary),
                Key::Second => alignment.axis(axes, Secondary),
                Key::Axis(AxisKey::Primary) => Primary,
                Key::Axis(AxisKey::Secondary) => Secondary,
                Key::Axis(AxisKey::Horizontal) => Horizontal.to_generic(axes),
                Key::Axis(AxisKey::Vertical) => Vertical.to_generic(axes),
            };

            let alignment = alignment.to_generic(axes, axis)?;
            Ok((axis, alignment))
        })?;

        map.with(Primary, |&val| ctx.alignment.primary = val);
        map.with(Secondary, |&val| ctx.alignment.secondary = val);

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
