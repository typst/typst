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
    /// A function to format the results with.
    format: Func,
) -> Value {
    QueryNode::new(target.0, format).pack().into()
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

/// Executes a query.
///
/// Display: Query
/// Category: special
#[node(Locatable, Show)]
pub struct QueryNode {
    /// The thing to search for.
    #[required]
    pub target: Selector,

    /// The function to format the results with.
    #[required]
    pub format: Func,
}

impl Show for QueryNode {
    fn show(&self, vt: &mut Vt, _: StyleChain) -> SourceResult<Content> {
        if !vt.introspector.init() {
            return Ok(Content::empty());
        }

        let id = self.0.stable_id().unwrap();
        let target = self.target();
        let (before, after) = vt.introspector.query_split(target, id);
        let func = self.format();
        Ok(func.call_vt(vt, [before.into(), after.into()])?.display())
    }
}
