use crate::prelude::*;

/// Todo: TODO!
/// Display: Provide
/// Category: meta
#[element(Behave, Show, Locatable)]
pub struct ProvideElem {
    #[required]
    pub key: EcoString,
    #[required]
    pub value: Value,
}

impl Show for ProvideElem {
    fn show(&self, _vt: &mut Vt, _styles: StyleChain) -> SourceResult<Content> {
        Ok(Content::empty())
    }
}

impl Behave for ProvideElem {
    fn behaviour(&self) -> Behaviour {
        Behaviour::Ignorant
    }
}
