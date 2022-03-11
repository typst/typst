use crate::library::prelude::*;

/// An inline-level container that sizes content and places it into a paragraph.
pub struct BoxNode;

#[class]
impl BoxNode {
    fn construct(_: &mut Context, args: &mut Args) -> TypResult<Content> {
        let width = args.named("width")?;
        let height = args.named("height")?;
        let body: LayoutNode = args.find()?.unwrap_or_default();
        Ok(Content::inline(body.sized(Spec::new(width, height))))
    }
}

/// A block-level container that places content into a separate flow.
pub struct BlockNode;

#[class]
impl BlockNode {
    fn construct(_: &mut Context, args: &mut Args) -> TypResult<Content> {
        Ok(Content::Block(args.find()?.unwrap_or_default()))
    }
}
