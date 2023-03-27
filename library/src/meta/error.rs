use typst::diag::SourceError;

use crate::prelude::*;

/// A compile-time error
///
/// Display: Error
/// Category: meta
#[element(Show, Construct)]
pub struct ErrorElem {
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
