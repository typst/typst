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
    LocateElem::new(func).pack().into()
}

/// Executes a `locate` call.
///
/// Display: Styled
/// Category: special
#[element(Locatable, Show)]
struct LocateElem {
    /// The function to call with the location.
    #[required]
    func: Func,
}

impl Show for LocateElem {
    fn show(&self, vt: &mut Vt, _: StyleChain) -> SourceResult<Content> {
        if !vt.introspector.init() {
            return Ok(Content::empty());
        }

        let location = self.0.location().unwrap();
        Ok(self.func().call_vt(vt, [location.into()])?.display())
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
    StyleElem::new(func).pack().into()
}

/// Executes a style access.
///
/// Display: Style
/// Category: special
#[element(Show)]
struct StyleElem {
    /// The function to call with the styles.
    #[required]
    func: Func,
}

impl Show for StyleElem {
    fn show(&self, vt: &mut Vt, styles: StyleChain) -> SourceResult<Content> {
        Ok(self.func().call_vt(vt, [styles.to_map().into()])?.display())
    }
}
