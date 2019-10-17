use super::prelude::*;
use crate::layout::Alignment;

/// Allows to align content in different ways.
#[derive(Debug, PartialEq)]
pub struct AlignFunc {
    alignment: Alignment,
    body: Option<SyntaxTree>,
}

impl Function for AlignFunc {
    fn parse(header: &FuncHeader, body: Option<&str>, ctx: ParseContext) -> ParseResult<Self>
    where Self: Sized {
        if header.args.len() != 1 || !header.kwargs.is_empty() {
            return err("align: expected exactly one positional argument");
        }

        let alignment = if let Expression::Ident(ident) = &header.args[0] {
            match ident.as_str() {
                "left" => Alignment::Left,
                "right" => Alignment::Right,
                "center" => Alignment::Center,
                s => return err(format!("invalid alignment specifier: '{}'", s)),
            }
        } else {
            return err(format!(
                "expected alignment specifier, found: '{}'",
                header.args[0]
            ));
        };

        let body = parse_maybe_body(body, ctx)?;

        Ok(AlignFunc { alignment, body })
    }

    fn layout(&self, ctx: LayoutContext) -> LayoutResult<CommandList> {
        if let Some(body) = &self.body {
            let layouts = layout_tree(body, LayoutContext {
                alignment: self.alignment,
                .. ctx
            })?;

            Ok(commands![Command::AddMany(layouts)])
        } else {
            Ok(commands![Command::SetAlignment(self.alignment)])
        }
    }
}
