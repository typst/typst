use crate::library::prelude::*;

/// A reference to a label.
#[derive(Debug, Hash)]
pub struct RefNode(pub EcoString);

#[node(showable)]
impl RefNode {
    fn construct(_: &mut Vm, args: &mut Args) -> SourceResult<Content> {
        Ok(Content::show(Self(args.expect("label")?)))
    }
}

impl Show for RefNode {
    fn unguard(&self, _: Selector) -> ShowNode {
        Self(self.0.clone()).pack()
    }

    fn field(&self, name: &str) -> Option<Value> {
        match name {
            "label" => Some(Value::Str(self.0.clone().into())),
            _ => None,
        }
    }

    fn realize(&self, _: Tracked<dyn World>, _: StyleChain) -> SourceResult<Content> {
        Ok(Content::Text(format_eco!("@{}", self.0)))
    }
}
