use crate::prelude::*;

/// # Repeat
/// Repeats content to fill a line.
///
/// ## Parameters
/// - body: Content (positional, required)
///   The content to repeat.
///
/// ## Category
/// layout
#[func]
#[capable(Layout, Inline)]
#[derive(Debug, Hash)]
pub struct RepeatNode(pub Content);

#[node]
impl RepeatNode {
    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        Ok(Self(args.expect("body")?).pack())
    }

    fn field(&self, name: &str) -> Option<Value> {
        match name {
            "body" => Some(Value::Content(self.0.clone())),
            _ => None,
        }
    }
}

impl Layout for RepeatNode {
    fn layout(
        &self,
        vt: &mut Vt,
        styles: StyleChain,
        regions: Regions,
    ) -> SourceResult<Fragment> {
        self.0.layout(vt, styles, regions)
    }
}

impl Inline for RepeatNode {}
