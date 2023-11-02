use crate::doc::{Lang, Region};
use crate::model::StyleChain;

/// The named with which an element is referenced.
pub trait LocalName {
    /// Get the name in the given language and (optionally) region.
    fn local_name(lang: Lang, region: Option<Region>) -> &'static str
    where
        Self: Sized;

    /// Resolve the local name with a style chain.
    fn local_name_in(styles: StyleChain) -> &'static str
    where
        Self: Sized;
}
