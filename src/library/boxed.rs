use super::prelude::*;
use crate::layout::Flow;

/// Wraps content into a box.
#[derive(Debug, PartialEq)]
pub struct BoxFunc {
    body: SyntaxTree,
    flow: Flow,
}

impl Function for BoxFunc {
    fn parse(header: &FuncHeader, body: Option<&str>, ctx: ParseContext) -> ParseResult<Self>
    where Self: Sized {
        let flow = if header.args.is_empty() {
            Flow::Vertical
        } else if header.args.len() == 1 {
            if let Expression::Ident(ident) = &header.args[0] {
                match ident.as_str() {
                    "vertical" => Flow::Vertical,
                    "horizontal" => Flow::Horizontal,
                    f => return err(format!("invalid flow specifier: '{}'", f)),
                }
            } else {
                return err(format!(
                    "expected alignment specifier, found: '{}'",
                    header.args[0]
                ));
            }
        } else {
            return err("box: expected flow specifier or no arguments");
        };

        if let Some(body) = body {
            Ok(BoxFunc {
                body: parse(body, ctx)?,
                flow,
            })
        } else {
            err("box: expected body")
        }
    }

    fn layout(&self, ctx: LayoutContext) -> LayoutResult<CommandList> {
        let layout = layout_tree(&self.body, LayoutContext {
            flow: self.flow,
            .. ctx
        })?;

        Ok(commands![Command::AddMany(layout)])
    }
}
