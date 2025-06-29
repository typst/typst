use ecow::{eco_format, EcoString};
use typst_utils::{PicoStr, ResolvedPicoStr};

use crate::diag::StrResult;
use crate::foundations::{bail, func, scope, ty, Repr, Str};

/// A label for an element.
///
/// Inserting a label into content attaches it to the closest preceding element
/// that is not a space. The preceding element must be in the same scope as the
/// label, which means that `[Hello #[<label>]]`, for instance, wouldn't work.
///
/// A labelled element can be [referenced]($ref), [queried]($query) for, and
/// [styled]($styling) through its label.
///
/// Once constructed, you can get the name of a label using
/// [`str`]($str/#constructor).
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
/// its name in angle brackets. This works both in markup and code. A label's
/// name can contain letters, numbers, `_`, `-`, `:`, and `.`. A label cannot
/// be empty.
///
/// Note that there is a syntactical difference when using the dedicated syntax
/// for this function. In the code below, the `[<a>]` terminates the heading and
/// thus attaches to the heading itself, whereas the `[#label("b")]` is part of
/// the heading and thus attaches to the heading's text.
///
/// ```typ
/// // Equivalent to `#heading[Introduction] <a>`.
/// = Introduction <a>
///
/// // Equivalent to `#heading[Conclusion #label("b")]`.
/// = Conclusion #label("b")
/// ```
///
/// Currently, labels can only be attached to elements in markup mode, not in
/// code mode. This might change in the future.
#[ty(scope, cast)]
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Label(PicoStr);

impl Label {
    /// Creates a label from an interned string.
    ///
    /// Returns `None` if the given string is empty.
    pub fn new(name: PicoStr) -> Option<Self> {
        const EMPTY: PicoStr = PicoStr::constant("");
        (name != EMPTY).then_some(Self(name))
    }

    /// Resolves the label to a string.
    pub fn resolve(self) -> ResolvedPicoStr {
        self.0.resolve()
    }

    /// Turns this label into its inner interned string.
    pub fn into_inner(self) -> PicoStr {
        self.0
    }
}

#[scope]
impl Label {
    /// Creates a label from a string.
    #[func(constructor)]
    pub fn construct(
        /// The name of the label. Must not be empty.
        name: Str,
    ) -> StrResult<Label> {
        if name.is_empty() {
            bail!("label name must not be empty");
        }

        Ok(Self(PicoStr::intern(name.as_str())))
    }
}

impl Repr for Label {
    fn repr(&self) -> EcoString {
        let resolved = self.resolve();
        if typst_syntax::is_valid_label_literal_id(&resolved) {
            eco_format!("<{resolved}>")
        } else {
            eco_format!("label({})", resolved.repr())
        }
    }
}

impl From<Label> for PicoStr {
    fn from(value: Label) -> Self {
        value.into_inner()
    }
}

/// Indicates that an element cannot be labelled.
pub trait Unlabellable {}
