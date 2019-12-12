use crate::func::prelude::*;
use super::maps::ConsistentMap;
use super::keys::AxisKey;

function! {
    /// `direction`: Sets the directions of the layouting axes.
    #[derive(Debug, PartialEq)]
    pub struct Direction {
        body: Option<SyntaxTree>,
        map: ConsistentMap<AxisKey, Axis>,
    }

    parse(args, body, ctx) {
        let mut map = ConsistentMap::new();

        map.add_opt_span(AxisKey::Primary, args.get_pos_opt::<Axis>()?)?;
        map.add_opt_span(AxisKey::Secondary, args.get_pos_opt::<Axis>()?)?;

        for arg in args.keys() {
            let axis = AxisKey::from_ident(&arg.v.key)?;
            let value = Axis::from_expr(arg.v.value)?;

            map.add(axis, value)?;
        }

        Direction {
            body: parse!(optional: body, ctx),
            map,
        }
    }

    layout(self, mut ctx) {
        let axes = ctx.axes;

        let map = self.map.dedup(|key, &direction| {
            Ok((match key {
                AxisKey::Primary => GenericAxisKind::Primary,
                AxisKey::Secondary => GenericAxisKind::Secondary,
                AxisKey::Horizontal => axes.horizontal(),
                AxisKey::Vertical => axes.vertical(),
            }, direction))
        })?;

        map.with(GenericAxisKind::Primary, |&val| ctx.axes.primary = val);
        map.with(GenericAxisKind::Secondary, |&val| ctx.axes.secondary = val);

        if ctx.axes.primary.is_horizontal() == ctx.axes.secondary.is_horizontal() {
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
