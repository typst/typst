use crate::func::prelude::*;
use super::maps::PosAxisMap;


function! {
    /// `direction`: Sets the directions of the layouting axes.
    #[derive(Debug, PartialEq)]
    pub struct DirectionFunc {
        body: Option<SyntaxTree>,
        map: PosAxisMap<Direction>,
    }

    parse(header, body, ctx) {
        DirectionFunc {
            body: parse!(optional: body, ctx),
            map: PosAxisMap::new(&mut header.args)?,
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
            Some(body) => vec![AddMultiple(layout(&body, ctx).await?)],
            None => vec![Command::SetAxes(ctx.axes)],
        }
    }
}
