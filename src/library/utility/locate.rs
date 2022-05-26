use crate::library::prelude::*;
use crate::model::LocateNode;

/// Format content with access to its location on the page.
pub fn locate(_: &mut Machine, args: &mut Args) -> TypResult<Value> {
    let node = LocateNode::new(args.expect("recipe")?);
    Ok(Value::Content(Content::Locate(node)))
}
