use crate::func::prelude::*;

/// 📐 `align`: Aligns content in different ways.
#[derive(Debug, PartialEq)]
pub struct Align {
    body: Option<SyntaxTree>,
    alignment: Alignment,
}

function! {
    data: Align,

    parse(args, body, ctx) {
        let body = parse!(optional: body, ctx);
        let arg = args.get_pos::<ArgIdent>()?;
        let alignment = match arg.val {
            "left" | "origin" => Alignment::Origin,
            "center" => Alignment::Center,
            "right" | "end" => Alignment::End,
            s => err!("invalid alignment specifier: {}", s),
        };
        args.done()?;

        Ok(Align {
            body,
            alignment,
        })
    }

    layout(this, ctx) {
        let mut new_axes = ctx.axes;
        new_axes.primary.alignment = this.alignment;

        Ok(match &this.body {
            Some(body) => commands![
                SetAxes(new_axes),
                LayoutTree(body),
                SetAxes(ctx.axes),
            ],
            None => commands![Command::SetAxes(new_axes)]
        })
    }
}
