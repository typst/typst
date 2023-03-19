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
    location: StableId,
    /// The location before which to query.
    #[named]
    #[external]
    before: StableId,
    /// The location after which to query.
    #[named]
    #[external]
    after: StableId,
) -> Value {
    let selector = target.0;
    let introspector = vm.vt.introspector;
    let elements = if let Some(id) = args.named("before")? {
        introspector.query_before(selector, id)
    } else if let Some(id) = args.named("after")? {
        introspector.query_after(selector, id)
    } else {
        let _: StableId = args.expect("id")?;
        introspector.query(selector)
    };
    elements.into()
}

/// A query target.
struct Target(Selector);

cast_from_value! {
    Target,
    label: Label => Self(Selector::Label(label)),
    func: Func => {
        let Some(id) = func.id() else {
            return Err("this function is not selectable".into());
        };

        if !Content::new(id).can::<dyn Locatable>() {
            Err(eco_format!("cannot query for {}s", id.name))?;
        }

        Self(Selector::Node(id, None))
    }
}
