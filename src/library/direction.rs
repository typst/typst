use crate::func::prelude::*;
use super::maps::ConsistentMap;
use super::keys::AxisKey;

function! {
    /// `direction`: Sets the directions of the layouting axes.
    #[derive(Debug, PartialEq)]
    pub struct DirectionChange {
        body: Option<SyntaxTree>,
        map: ConsistentMap<AxisKey, Direction>,
    }

    parse(args, body, ctx) {
        let mut map = ConsistentMap::new();

        map.add_opt_span(AxisKey::Primary, args.get_pos_opt::<Direction>()?)?;
        map.add_opt_span(AxisKey::Secondary, args.get_pos_opt::<Direction>()?)?;

        for arg in args.keys() {
            let axis = AxisKey::from_ident(&arg.v.key)?;
            let value = Direction::from_expr(arg.v.value)?;

            map.add(axis, value)?;
        }

        DirectionChange {
            body: parse!(optional: body, ctx),
            map,
        }
    }

    layout(self, mut ctx) {
        let axes = ctx.axes;

        let map = self.map.dedup(|key, &direction| {
            Ok((match key {
                AxisKey::Primary => Primary,
                AxisKey::Secondary => Secondary,
                AxisKey::Horizontal => Horizontal.to_generic(axes),
                AxisKey::Vertical => Vertical.to_generic(axes),
            }, direction))
        })?;

        map.with(Primary, |&val| ctx.axes.primary = val);
        map.with(Secondary, |&val| ctx.axes.secondary = val);

        if ctx.axes.primary.axis() == ctx.axes.secondary.axis() {
            error!(
                "aligned primary and secondary axes: `{}`, `{}`",
                format!("{:?}", ctx.axes.primary).to_lowercase(),
                format!("{:?}", ctx.axes.secondary).to_lowercase(),
            );
        }

        match &self.body {
            Some(body) => vec![AddMultiple(layout_tree(&body, ctx)?)],
            None => vec![Command::SetAxes(ctx.axes)],
        }
    }
}
