use crate::library::prelude::*;

/// An inline-level container that sizes content and places it into a paragraph.
pub struct BoxNode;

#[node]
impl BoxNode {
    fn construct(_: &mut Vm, args: &mut Args) -> SourceResult<Content> {
        let width = args.named("width")?;
        let height = args.named("height")?;
        let body: LayoutNode = args.eat()?.unwrap_or_default();
        Ok(Content::inline(body.sized(Axes::new(width, height))))
    }
}

/// A block-level container that places content into a separate flow.
pub struct BlockNode;

#[node]
impl BlockNode {
    fn construct(_: &mut Vm, args: &mut Args) -> SourceResult<Content> {
        Ok(Content::Block(args.eat()?.unwrap_or_default()))
    }
}
