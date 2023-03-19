use crate::prelude::*;

/// Provide access to the location of content.
///
/// Display: Locate
/// Category: meta
/// Returns: content
#[func]
pub fn locate(
    /// The function to call with the location.
    func: Func,
) -> Value {
    LocateNode::new(func).pack().into()
}

/// Executes a `locate` call.
///
/// Display: Styled
/// Category: special
#[node(Locatable, Show)]
struct LocateNode {
    /// The function to call with the location.
    #[required]
    func: Func,
}

impl Show for LocateNode {
    fn show(&self, vt: &mut Vt, _: StyleChain) -> SourceResult<Content> {
        if !vt.introspector.init() {
            return Ok(Content::empty());
        }

        let id = self.0.stable_id().unwrap();
        Ok(self.func().call_vt(vt, [id.into()])?.display())
    }
}

/// Provide access to active styles.
///
/// Display: Styled
/// Category: layout
/// Returns: content
#[func]
pub fn style(
    /// The function to call with the styles.
    func: Func,
) -> Value {
    StyleNode::new(func).pack().into()
}

/// Executes a style access.
///
/// Display: Style
/// Category: special
#[node(Show)]
struct StyleNode {
    /// The function to call with the styles.
    #[required]
    func: Func,
}

impl Show for StyleNode {
    fn show(&self, vt: &mut Vt, styles: StyleChain) -> SourceResult<Content> {
        Ok(self.func().call_vt(vt, [styles.to_map().into()])?.display())
    }
}
