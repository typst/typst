use ecow::EcoString;

use crate::foundations::{Repr, ty};

/// A type with two states.
///
/// The boolean type has two values: `{true}` and `{false}`. It denotes whether
/// something is active or enabled.
///
/// # Example
/// ```example
/// #false \
/// #true \
/// #(1 < 2)
/// ```
#[ty(cast, title = "Boolean")]
type bool;

impl Repr for bool {
    fn repr(&self) -> EcoString {
        match self {
            true => "true".into(),
            false => "false".into(),
        }
    }
}
