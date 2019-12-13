use crate::func::prelude::*;
use super::maps::PosAxisMap;

function! {
    /// `direction`: Sets the directions of the layouting axes.
    #[derive(Debug, PartialEq)]
    pub struct DirectionChange {
        body: Option<SyntaxTree>,
        map: PosAxisMap<Direction>,
    }

    parse(args, body, ctx) {
        DirectionChange {
            body: parse!(optional: body, ctx),
            map: PosAxisMap::new(&mut args)?,
        }
    }

    layout(self, mut ctx) {
        ctx.base = ctx.spaces[0].dimensions;

        let map = self.map.dedup(ctx.axes, |direction| {
            Some(direction.axis().to_generic(ctx.axes))
        })?;

        map.with(Primary, |&dir| ctx.axes.primary = dir);
        map.with(Secondary, |&dir| ctx.axes.secondary = dir);

        if ctx.axes.primary.axis() == ctx.axes.secondary.axis() {
            error!(
                "invalid aligned primary and secondary axes: `{}`, `{}`",
                format!("{:?}", ctx.axes.primary).to_lowercase(),
                format!("{:?}", ctx.axes.secondary).to_lowercase(),
            );
        }

        match &self.body {
            Some(body) => vec![AddMultiple(layout(&body, ctx)?)],
            None => vec![Command::SetAxes(ctx.axes)],
        }
    }
}
