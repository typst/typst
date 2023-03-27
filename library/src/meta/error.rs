use typst::diag::SourceError;

use crate::prelude::*;

/// A compile-time error.
///
/// Note that the error is only emitted when the error element is realized. If the error never
/// appears on the document, nothing will happen.
///
/// ## Example
/// ```example
/// // It is fine to create an error element, as long as it is never realized.
/// #let x = error("My error")
///
/// // Uncommenting this line would cause compilation to fail.
/// // #x
/// ```
///
/// Display: Error
/// Category: meta
#[element(Show, Construct)]
pub struct ErrorElem {
    /// The error string that will appear on compilation failure.
    #[required]
    pub error: SourceError,
}

impl Construct for ErrorElem {
    fn construct(_vm: &mut Vm, args: &mut Args) -> SourceResult<Content> {
        let err = args.expect::<Spanned<EcoString>>("error")?;
        Ok(Self::new(error!(err.span, err.v)).pack())
    }
}

impl Show for ErrorElem {
    fn show(&self, _vt: &mut Vt, _styles: StyleChain) -> SourceResult<Content> {
        bail!(self.error())
    }
}

impl From<SourceError> for ErrorElem {
    fn from(value: SourceError) -> Self {
        ErrorElem::new(value)
    }
}
