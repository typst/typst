use std::fmt::{self, Debug, Formatter};

use ecow::EcoString;
use serde::{Serialize, Serializer};

/// A label for an element.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Label(pub EcoString);

impl Serialize for Label {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&format!("<{}>", self.0))
    }
}

impl Debug for Label {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "<{}>", self.0)
    }
}

/// Indicates that an element cannot be labelled.
pub trait Unlabellable {}
