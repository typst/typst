use std::fmt::{self, Debug, Formatter};

use ecow::EcoString;

/// A label for an element.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Label(pub EcoString);

impl Debug for Label {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "<{}>", self.0)
    }
}

/// Indicates that an element cannot be labelled.
pub trait Unlabellable {}
