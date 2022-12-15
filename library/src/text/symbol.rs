use crate::prelude::*;
use crate::text::TextNode;

/// A symbol identified by symmie notation.
///
/// Tags: text.
#[func]
#[capable(Show)]
#[derive(Debug, Hash)]
pub struct SymbolNode(pub EcoString);

#[node]
impl SymbolNode {
    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        Ok(Self(args.expect("notation")?).pack())
    }

    fn field(&self, name: &str) -> Option<Value> {
        match name {
            "notation" => Some(Value::Str(self.0.clone().into())),
            _ => None,
        }
    }
}

impl Show for SymbolNode {
    fn show(&self, _: &mut Vt, this: &Content, _: StyleChain) -> SourceResult<Content> {
        match symmie::get(&self.0) {
            Some(c) => Ok(TextNode::packed(c)),
            None => {
                if let Some(span) = this.span() {
                    bail!(span, "unknown symbol");
                }

                Ok(Content::empty())
            }
        }
    }
}
