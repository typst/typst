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
            return err("expected exactly one positional argument specifying the alignment");
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

        let body = if let Some(body) = body {
            Some(parse(body, ctx)?)
        } else {
            None
        };

        Ok(AlignFunc { alignment, body })
    }

    fn layout(&self, mut ctx: LayoutContext) -> LayoutResult<FuncCommands> {
        if let Some(body) = &self.body {
            ctx.alignment = self.alignment;

            let layouts = layout_tree(body, ctx)?;

            let mut commands = FuncCommands::new();
            commands.add(Command::AddMany(layouts));

            Ok(commands)
        } else {
            unimplemented!("context-modifying align func")
        }
    }
}
