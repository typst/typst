use crate::prelude::*;

/// # Hide
/// Hide content without affecting layout.
///
/// The `hide` function allows you to hide content while the layout still 'sees'
/// it. This is useful to create to create whitespace that is exactly as large
/// as some content. It may also be useful to redact content because its
/// arguments are not included in the output.
///
/// ## Example
/// ```
/// Hello Jane \
/// #hide[Hello] Joe
/// ```
///
/// ## Parameters
/// - body: Content (positional, required)
///   The content to hide.
///
/// ## Category
/// layout
#[func]
#[capable(Layout, Inline)]
#[derive(Debug, Hash)]
pub struct HideNode(pub Content);

#[node]
impl HideNode {
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

impl Layout for HideNode {
    fn layout(
        &self,
        vt: &mut Vt,
        styles: StyleChain,
        regions: Regions,
    ) -> SourceResult<Fragment> {
        let mut fragment = self.0.layout(vt, styles, regions)?;
        for frame in &mut fragment {
            frame.clear();
        }
        Ok(fragment)
    }
}

impl Inline for HideNode {}
