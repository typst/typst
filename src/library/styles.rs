use toddle::query::FontClass;

use super::prelude::*;

macro_rules! style_func {
    (
        $(#[$outer:meta])*
        pub struct $struct:ident { $name:expr },
        $style:ident => $class:ident
    ) => {
        $(#[$outer])*
        #[derive(Debug, PartialEq)]
        pub struct $struct {
            body: Option<SyntaxTree>
        }

        impl Function for $struct {
            fn parse(header: &FuncHeader, body: Option<&str>, ctx: ParseContext)
                -> ParseResult<Self> where Self: Sized {
                // Accept only invocations without arguments and with body.
                if has_arguments(header) {
                    return err(format!("{}: expected no arguments", $name));
                }

                let body = parse_maybe_body(body, ctx)?;

                Ok($struct { body })
            }

            fn layout(&self, ctx: LayoutContext) -> LayoutResult<CommandList> {
                let mut new_style = ctx.style.clone();
                new_style.toggle_class(FontClass::$class);

                if let Some(body) = &self.body {
                    let saved_style = ctx.style.clone();
                    Ok(commands![
                        Command::SetStyle(new_style),
                        Command::Layout(body),
                        Command::SetStyle(saved_style),
                    ])
                } else {
                    Ok(commands![Command::SetStyle(new_style)])
                }
            }
        }
    };
}

style_func! {
    /// Typesets text in bold.
    pub struct BoldFunc { "bold" },
    style => Bold
}

style_func! {
    /// Typesets text in italics.
    pub struct ItalicFunc { "italic" },
    style => Italic
}

style_func! {
    /// Typesets text in monospace.
    pub struct MonospaceFunc { "mono" },
    style => Monospace
}
