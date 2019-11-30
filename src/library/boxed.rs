use crate::func::prelude::*;

/// `box`: Layouts content into a box.
#[derive(Debug, PartialEq)]
pub struct Boxed {
    body: SyntaxTree,
    width: Option<Size>,
    height: Option<Size>,
}

function! {
    data: Boxed,

    parse(args, body, ctx) {
        let width = args.get_key_opt::<ArgSize>("width")?.map(|a| a.val);
        let height = args.get_key_opt::<ArgSize>("height")?.map(|a| a.val);
        args.done()?;

        let body = parse!(required: body, ctx);
        Ok(Boxed {
            body,
            width,
            height,
        })
    }

    layout(this, mut ctx) {
        if let Some(width) = this.width {
            ctx.spaces[0].dimensions.x = width;
        }
        if let Some(height) = this.height {
            ctx.spaces[0].dimensions.y = height;
        }

        Ok(vec![AddMultiple(layout_tree(&this.body, ctx)?)])
    }
}
