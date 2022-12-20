use crate::prelude::*;
use crate::text::TextNode;

/// # Symbol
/// A symbol identified by symmie notation.
///
/// Symmie is Typst's notation for Unicode symbols. It is based on the idea of
/// _modifiers._ Many symbols in Unicode are very similar. In symmie, such
/// groups of symbols share a common name. To distinguish between the symbols
/// within a group, we use one or multiple modifiers that are separated from the
/// name by colons.
///
/// There is currently no easily viewable list of all names, but in the
/// meantime you can rely on the autocompletion in Typst's web editor.
///
/// ## Syntax
/// This function also has dedicated syntax: In markup, you can enclose symmie
/// notation within colons to produce a symbol. And in math, you can just write
/// the notation directly. There, all letter sequence of length at least two are
/// automatically parsed as symbols (unless a variable of that name is defined).
///
/// ## Example
/// ```
/// // In text, with colons.
/// :arrow:l: \
/// :arrow:r: \
/// :arrow:t: \
/// :turtle: \
/// :face:halo: \
/// :woman:old:
///
/// // In math, directly.
/// $f : NN -> RR$ \
/// $A sub:eq B without C$ \
/// $a times:div b eq:not c$
/// ```
///
/// ## Parameters
/// - notation: EcoString (positional, required)
///   The symbol's symmie notation.
///
///   Consists of a name, followed by a number colon-separated modifiers
///   in no particular order.
///
///   ### Example
///   ```
///   #symbol("NN") \
///   #symbol("face:grin")
///   ```
///
/// ## Category
/// text
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
