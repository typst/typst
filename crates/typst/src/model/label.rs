use std::fmt::Debug;

use ecow::{eco_format, EcoString};

use crate::eval::{func, scope, ty, Repr};

/// A label for an element.
///
/// Inserting a label into content attaches it to the closest previous element
/// that is not a space. Then, the element can be [referenced]($ref) and styled
/// through the label.
///
/// # Example
/// ```example
/// #show <a>: set text(blue)
/// #show label("b"): set text(red)
///
/// = Heading <a>
/// *Strong* #label("b")
/// ```
///
/// # Syntax
/// This function also has dedicated syntax: You can create a label by enclosing
/// its name in angle brackets. This works both in markup and code.
#[ty(scope)]
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Label(pub EcoString);

#[scope]
impl Label {
    /// Creates a label from a string.
    #[func(constructor)]
    pub fn construct(
        /// The name of the label.
        name: EcoString,
    ) -> Label {
        Self(name)
    }
}

impl Repr for Label {
    fn repr(&self) -> EcoString {
        eco_format!("<{}>", self.0)
    }
}

/// Indicates that an element cannot be labelled.
pub trait Unlabellable {}
