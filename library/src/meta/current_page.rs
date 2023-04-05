use crate::prelude::*;

/// Obtain the [numbering]($func/numbering) scheme of the current page.
///
/// Display: Page numbering
/// Category: meta
/// Returns: content
#[func]
pub fn page_numbering(#[external] location: Location) -> Value {
    let location: Location = args.expect("location")?;
    let introspector = vm.vt.introspector;

    introspector.page_numbering(location).into()
}
