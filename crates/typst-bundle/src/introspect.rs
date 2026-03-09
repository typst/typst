use std::fmt::{self, Debug, Formatter};
use std::num::NonZeroUsize;

use ecow::{EcoString, EcoVec};
use typst_library::diag::StrResult;
use typst_library::foundations::{Content, Label, Selector};
use typst_library::introspection::{DocumentPosition, Introspector, Location};
use typst_library::model::Numbering;

/// An introspector implementation for bundles.
#[derive(Clone)]
pub struct BundleIntrospector {}

#[expect(unused)]
impl Introspector for BundleIntrospector {
    fn query(&self, selector: &Selector) -> EcoVec<Content> {
        todo!()
    }

    fn query_first(&self, selector: &Selector) -> Option<Content> {
        todo!()
    }

    fn query_unique(&self, selector: &Selector) -> StrResult<Content> {
        todo!()
    }

    fn query_label(&self, label: Label) -> StrResult<&Content> {
        todo!()
    }

    fn query_labelled(&self) -> EcoVec<Content> {
        todo!()
    }

    fn query_count_before(&self, selector: &Selector, end: Location) -> usize {
        todo!()
    }

    fn label_count(&self, label: Label) -> usize {
        todo!()
    }

    fn locator(&self, key: u128, base: Location) -> Option<Location> {
        todo!()
    }

    fn pages(&self) -> Option<NonZeroUsize> {
        None
    }

    fn page(&self, location: Location) -> Option<NonZeroUsize> {
        todo!()
    }

    fn position(&self, location: Location) -> Option<DocumentPosition> {
        todo!()
    }

    fn page_numbering(&self, location: Location) -> Option<&Numbering> {
        todo!()
    }

    fn page_supplement(&self, location: Location) -> Option<&Content> {
        todo!()
    }

    fn anchor(&self, location: Location) -> Option<&EcoString> {
        todo!()
    }
}

impl Debug for BundleIntrospector {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.pad("BundleIntrospector(..)")
    }
}
