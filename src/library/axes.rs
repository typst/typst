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
        let mut axes = ctx.axes;
        axes.primary.alignment = this.alignment;

        if ctx.axes.primary.alignment == Alignment::End
           && this.alignment == Alignment::Origin {
            axes.primary.expand = true;
        }

        Ok(match &this.body {
            Some(body) => commands![AddMultiple(
                layout_tree(body, LayoutContext {
                    axes,
                    .. ctx.clone()
                })?
            )],
            None => commands![Command::SetAxes(axes)]
        })
    }
}
