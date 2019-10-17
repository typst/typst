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

            fn layout(&self, ctx: LayoutContext) -> LayoutResult<CommandList> {
                let mut commands = CommandList::new();

                let saved_style = ctx.style.clone();
                let mut new_style = ctx.style.clone();
                new_style.toggle_class(FontClass::$class);

                commands.add(Command::SetStyle(new_style));
                commands.add(Command::Layout(&self.body));
                commands.add(Command::SetStyle(saved_style));

                Ok(commands)
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
