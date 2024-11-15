//! Capabilities for Typst IDE support.

mod analyze;
mod complete;
mod definition;
mod jump;
mod matchers;
mod tooltip;
mod utils;

pub use self::analyze::{analyze_expr, analyze_import, analyze_labels};
pub use self::complete::{autocomplete, Completion, CompletionKind};
pub use self::definition::{definition, Definition};
pub use self::jump::{jump_from_click, jump_from_cursor, Jump};
pub use self::matchers::{deref_target, named_items, DerefTarget, NamedItem};
pub use self::tooltip::{tooltip, Tooltip};

use ecow::EcoString;
use typst::syntax::package::PackageSpec;
use typst::syntax::FileId;
use typst::World;

/// Extends the `World` for IDE functionality.
pub trait IdeWorld: World {
    /// Turn this into a normal [`World`].
    ///
    /// This is necessary because trait upcasting is experimental in Rust.
    /// See <https://github.com/rust-lang/rust/issues/65991>.
    ///
    /// Implementors can simply return `self`.
    fn upcast(&self) -> &dyn World;

    /// A list of all available packages and optionally descriptions for them.
    ///
    /// This function is **optional** to implement. It enhances the user
    /// experience by enabling autocompletion for packages. Details about
    /// packages from the `@preview` namespace are available from
    /// `https://packages.typst.org/preview/index.json`.
    fn packages(&self) -> &[(PackageSpec, Option<EcoString>)] {
        &[]
    }

    /// Returns a list of all known files.
    ///
    /// This function is **optional** to implement. It enhances the user
    /// experience by enabling autocompletion for file paths.
    fn files(&self) -> Vec<FileId> {
        vec![]
    }
}

#[cfg(test)]
mod tests;
