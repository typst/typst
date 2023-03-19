use crate::prelude::*;

/// Find elements in the document.
///
/// Display: Query
/// Category: meta
/// Returns: content
#[func]
pub fn query(
    /// The thing to search for.
    target: Target,
    /// The location.
    #[external]
    location: Location,
    /// The location before which to query.
    #[named]
    #[external]
    before: Location,
    /// The location after which to query.
    #[named]
    #[external]
    after: Location,
) -> Value {
    let selector = target.0;
    let introspector = vm.vt.introspector;
    let elements = if let Some(location) = args.named("before")? {
        introspector.query_before(selector, location)
    } else if let Some(location) = args.named("after")? {
        introspector.query_after(selector, location)
    } else {
        let _: Location = args.expect("location")?;
        introspector.query(selector)
    };
    elements.into()
}

/// A query target.
struct Target(Selector);

cast_from_value! {
    Target,
    label: Label => Self(Selector::Label(label)),
    element: ElemFunc => {
        if !Content::new(element).can::<dyn Locatable>() {
            Err(eco_format!("cannot query for {}s", element.name()))?;
        }

        Self(Selector::Elem(element, None))
    }
}
