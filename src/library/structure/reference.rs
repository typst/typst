use crate::library::prelude::*;

/// A reference to a label.
#[derive(Debug, Hash)]
pub struct RefNode(pub EcoString);

#[node(Show)]
impl RefNode {
    fn construct(_: &mut Vm, args: &mut Args) -> SourceResult<Content> {
        Ok(Self(args.expect("label")?).pack())
    }
}

impl Show for RefNode {
    fn unguard_parts(&self, _: Selector) -> Content {
        Self(self.0.clone()).pack()
    }

    fn field(&self, name: &str) -> Option<Value> {
        match name {
            "label" => Some(Value::Str(self.0.clone().into())),
            _ => None,
        }
    }

    fn realize(&self, _: Tracked<dyn World>, _: StyleChain) -> SourceResult<Content> {
        Ok(TextNode(format_eco!("@{}", self.0)).pack())
    }
}
