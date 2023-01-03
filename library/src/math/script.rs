use super::*;

/// # Script
/// A mathematical sub- and/or superscript.
///
/// _Note:_ In the future, this might be unified with the [sub](@sub) and
/// [super](@super) functions that handle sub- and superscripts in text.
///
/// ## Syntax
/// This function also has dedicated syntax: Use the underscore (`_`) to
/// indicate a subscript and the circumflex (`^`) to indicate a superscript.
///
/// ## Example
/// ```
/// $ a_i = 2^(1+i) $
/// ```
///
/// ## Parameters
/// - base: Content (positional, required)
///   The base to which the applies the sub- and/or superscript.
///
/// - sub: Content (named)
///   The subscript.
///
/// - sup: Content (named)
///   The superscript.
///
/// ## Category
/// math
#[func]
#[capable(Texify)]
#[derive(Debug, Hash)]
pub struct ScriptNode {
    /// The base.
    pub base: Content,
    /// The subscript.
    pub sub: Option<Content>,
    /// The superscript.
    pub sup: Option<Content>,
}

#[node]
impl ScriptNode {
    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        let base = args.expect("base")?;
        let sub = args.named("sub")?;
        let sup = args.named("sup")?;
        Ok(Self { base, sub, sup }.pack())
    }
}

impl Texify for ScriptNode {
    fn texify(&self, t: &mut Texifier) -> SourceResult<()> {
        self.base.texify(t)?;

        if let Some(sub) = &self.sub {
            t.push_str("_{");
            sub.texify_unparen(t)?;
            t.push_str("}");
        }

        if let Some(sup) = &self.sup {
            t.push_str("^{");
            sup.texify_unparen(t)?;
            t.push_str("}");
        }

        Ok(())
    }
}
