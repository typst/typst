use crate::func::prelude::*;
use super::keys::*;

function! {
    /// `align`: Aligns content along the layouting axes.
    #[derive(Debug, PartialEq)]
    pub struct Align {
        body: Option<SyntaxTree>,
        map: ConsistentMap<Key, AlignmentKey>,
    }

    parse(args, body, ctx) {
        let mut map = ConsistentMap::new();

        map.add_opt_span(Key::First, args.get_pos_opt::<AlignmentKey>()?)?;
        map.add_opt_span(Key::Second, args.get_pos_opt::<AlignmentKey>()?)?;

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
                Key::First => alignment.axis(axes, GenericAxisKind::Primary),
                Key::Second => alignment.axis(axes, GenericAxisKind::Secondary),
                Key::Axis(AxisKey::Primary) => GenericAxisKind::Primary,
                Key::Axis(AxisKey::Secondary) => GenericAxisKind::Secondary,
                Key::Axis(AxisKey::Horizontal) => axes.horizontal(),
                Key::Axis(AxisKey::Vertical) => axes.vertical(),
            };

            let alignment = alignment.generic(axes, axis)?;
            Ok((axis, alignment))
        })?;

        map.with(GenericAxisKind::Primary, |&val| ctx.alignment.primary = val);
        map.with(GenericAxisKind::Secondary, |&val| ctx.alignment.secondary = val);

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
