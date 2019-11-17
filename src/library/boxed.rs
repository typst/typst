use crate::func::prelude::*;

/// `box`: Layouts content into a box.
#[derive(Debug, PartialEq)]
pub struct Boxed {
    body: SyntaxTree,
}

function! {
    data: Boxed,

    parse(args, body, ctx) {
        args.done()?;
        let body = parse!(required: body, ctx);
        Ok(Boxed { body })
    }

    layout(this, ctx) {
        Ok(commands![AddMultiple(layout_tree(&this.body, ctx)?)])
    }
}
