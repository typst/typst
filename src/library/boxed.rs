use super::prelude::*;

/// Wraps content into a box.
#[derive(Debug, PartialEq)]
pub struct BoxFunc {
    body: SyntaxTree
}

impl Function for BoxFunc {
    fn parse(header: &FuncHeader, body: Option<&str>, ctx: ParseContext) -> ParseResult<Self>
    where Self: Sized {
        if has_arguments(header) {
            return err("pagebreak: expected no arguments");
        }

        if let Some(body) = body {
            Ok(BoxFunc {
                body: parse(body, ctx)?
            })
        } else {
            err("box: expected body")
        }
    }

    fn layout(&self, ctx: LayoutContext) -> LayoutResult<CommandList> {
        let layout = layout_tree(&self.body, ctx)?;
        Ok(commands![Command::AddMany(layout)])
    }
}
