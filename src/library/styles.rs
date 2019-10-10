//! Basic style functions: bold, italic, monospace.

use super::prelude::*;
use toddle::query::FontClass;



macro_rules! style_func {
    ($(#[$outer:meta])* pub struct $struct:ident { $name:expr },
     $style:ident => $style_change:block) => {
        $(#[$outer])*
        #[derive(Debug, PartialEq)]
        pub struct $struct { body: SyntaxTree }

        impl Function for $struct {
            fn parse(header: &FuncHeader, body: Option<&str>, ctx: ParseContext)
                -> ParseResult<Self> where Self: Sized {
                // Accept only invocations without arguments and with body.
                if header.args.is_empty() && header.kwargs.is_empty() {
                    if let Some(body) = body {
                        Ok($struct { body: parse(body, ctx)? })
                    } else {
                        Err(ParseError::new(format!("expected body for function `{}`", $name)))
                    }
                } else {
                    Err(ParseError::new(format!("unexpected arguments to function `{}`", $name)))
                }
            }

            fn layout(&self, ctx: LayoutContext) -> LayoutResult<Option<Layout>> {
                // Change the context.
                let mut $style = ctx.style.clone();
                $style_change

                // Create a box and put it into a flex layout.
                let boxed = layout(&self.body, LayoutContext {
                    style: &$style,
                    .. ctx
                })?;
                let flex = FlexLayout::from_box(boxed);

                Ok(Some(Layout::Flex(flex)))
            }
        }
    };
}

style_func! {
    /// Typesets text in bold.
    pub struct BoldFunc { "bold" },
    style => { style.toggle_class(FontClass::Bold) }
}

style_func! {
    /// Typesets text in italics.
    pub struct ItalicFunc { "italic" },
    style => { style.toggle_class(FontClass::Italic) }
}

style_func! {
    /// Typesets text in monospace.
    pub struct MonospaceFunc { "mono" },
    style => { style.toggle_class(FontClass::Monospace) }
}
