use crate::func::prelude::*;
use toddle::query::FontClass;

macro_rules! stylefunc {
    ($ident:ident, $doc:expr) => (
        #[doc = $doc]
        #[derive(Debug, PartialEq)]
        pub struct $ident {
            body: Option<SyntaxTree>
        }

        function! {
            data: $ident,

            parse(args, body, ctx) {
                args.done()?;
                Ok($ident { body: parse!(optional: body, ctx) })
            }

            layout(this, ctx) {
                let mut style = ctx.style.clone();
                style.toggle_class(FontClass::$ident);

                Ok(match &this.body {
                    Some(body) => commands![
                        SetStyle(style),
                        LayoutTree(body),
                        SetStyle(ctx.style.clone()),
                    ],
                    None => commands![SetStyle(style)]
                })
            }
        }
    );
}

stylefunc!(Italic, "`italic`: Sets text in _italics_.");
stylefunc!(Bold, "`bold`: Sets text in **bold**.");
stylefunc!(Monospace, "`mono`: Sets text in `monospace`.");
