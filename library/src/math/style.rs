use super::*;

/// # Serif
/// Serif (roman) font style in math.
///
/// This is already the default.
///
/// _Note:_ In the future this might be unified with text styling.
///
/// ## Parameters
/// - body: Content (positional, required)
///   The piece of formula to style.
///
/// ## Category
/// math
#[func]
#[capable(Texify)]
#[derive(Debug, Hash)]
pub struct SerifNode(pub Content);

#[node]
impl SerifNode {
    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        Ok(Self(args.expect("body")?).pack())
    }
}

impl Texify for SerifNode {
    fn texify(&self, t: &mut Texifier) -> SourceResult<()> {
        t.push_str("\\mathrm{");
        self.0.texify_unparen(t)?;
        t.push_str("}");
        Ok(())
    }
}

/// # Sans-serif
/// Sans-serif font style in math.
///
/// _Note:_ In the future this might be unified with text styling.
///
/// ## Example
/// ```
/// $ sans(A B C) $
/// ```
///
/// ## Parameters
/// - body: Content (positional, required)
///   The piece of formula to style.
///
/// ## Category
/// math
#[func]
#[capable(Texify)]
#[derive(Debug, Hash)]
pub struct SansNode(pub Content);

#[node]
impl SansNode {
    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        Ok(Self(args.expect("body")?).pack())
    }
}

impl Texify for SansNode {
    fn texify(&self, t: &mut Texifier) -> SourceResult<()> {
        t.push_str("\\mathsf{");
        self.0.texify_unparen(t)?;
        t.push_str("}");
        Ok(())
    }
}

/// # Bold
/// Bold font style in math.
///
/// _Note:_ In the future this might be unified with text styling.
///
/// ## Example
/// ```
/// $ bold(A) := B^+ $
/// ```
///
/// ## Parameters
/// - body: Content (positional, required)
///   The piece of formula to style.
///
/// ## Category
/// math
#[func]
#[capable(Texify)]
#[derive(Debug, Hash)]
pub struct BoldNode(pub Content);

#[node]
impl BoldNode {
    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        Ok(Self(args.expect("body")?).pack())
    }
}

impl Texify for BoldNode {
    fn texify(&self, t: &mut Texifier) -> SourceResult<()> {
        t.push_str("\\mathbf{");
        self.0.texify_unparen(t)?;
        t.push_str("}");
        Ok(())
    }
}

/// # Italic
/// Italic font style in math.
///
/// This is already the default.
///
/// _Note:_ In the future this might be unified with text styling.
///
/// ## Parameters
/// - body: Content (positional, required)
///   The piece of formula to style.
///
/// ## Category
/// math
#[func]
#[capable(Texify)]
#[derive(Debug, Hash)]
pub struct ItalNode(pub Content);

#[node]
impl ItalNode {
    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        Ok(Self(args.expect("body")?).pack())
    }
}

impl Texify for ItalNode {
    fn texify(&self, t: &mut Texifier) -> SourceResult<()> {
        t.push_str("\\mathit{");
        self.0.texify_unparen(t)?;
        t.push_str("}");
        Ok(())
    }
}

/// # Calligraphic
/// Calligraphic font style in math.
///
/// _Note:_ In the future this might be unified with text styling.
///
/// ## Example
/// ```
/// Let $cal(P)$ be the set of ...
/// ```
///
/// ## Parameters
/// - body: Content (positional, required)
///   The piece of formula to style.
///
/// ## Category
/// math
#[func]
#[capable(Texify)]
#[derive(Debug, Hash)]
pub struct CalNode(pub Content);

#[node]
impl CalNode {
    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        Ok(Self(args.expect("body")?).pack())
    }
}

impl Texify for CalNode {
    fn texify(&self, t: &mut Texifier) -> SourceResult<()> {
        t.push_str("\\mathcal{");
        self.0.texify_unparen(t)?;
        t.push_str("}");
        Ok(())
    }
}

/// # Fraktur
/// Fraktur font style in math.
///
/// _Note:_ In the future this might be unified with text styling.
///
/// ## Example
/// ```
/// $ frak(P) $
/// ```
///
/// ## Parameters
/// - body: Content (positional, required)
///   The piece of formula to style.
///
/// ## Category
/// math
#[func]
#[capable(Texify)]
#[derive(Debug, Hash)]
pub struct FrakNode(pub Content);

#[node]
impl FrakNode {
    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        Ok(Self(args.expect("body")?).pack())
    }
}

impl Texify for FrakNode {
    fn texify(&self, t: &mut Texifier) -> SourceResult<()> {
        t.push_str("\\mathfrak{");
        self.0.texify_unparen(t)?;
        t.push_str("}");
        Ok(())
    }
}

/// # Monospace
/// Monospace font style in math.
///
/// _Note:_ In the future this might be unified with text styling.
///
/// ## Example
/// ```
/// $ mono(x + y = z) $
/// ```
///
/// ## Parameters
/// - body: Content (positional, required)
///   The piece of formula to style.
///
/// ## Category
/// math
#[func]
#[capable(Texify)]
#[derive(Debug, Hash)]
pub struct MonoNode(pub Content);

#[node]
impl MonoNode {
    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        Ok(Self(args.expect("body")?).pack())
    }
}

impl Texify for MonoNode {
    fn texify(&self, t: &mut Texifier) -> SourceResult<()> {
        t.push_str("\\mathtt{");
        self.0.texify_unparen(t)?;
        t.push_str("}");
        Ok(())
    }
}

/// # Blackboard Bold
/// Blackboard bold (double-struck) font style in math.
///
/// For uppercase latin letters, blackboard bold is additionally available
/// through [symmie symbols](@symbol) of the form `NN` and `RR`.
///
/// _Note:_ In the future this might be unified with text styling.
///
/// ## Example
/// ```
/// $ bb(b) $
/// $ bb(N) = NN $
/// $ f: NN -> RR $
/// ```
///
/// ## Parameters
/// - body: Content (positional, required) The piece of formula to style.
///
/// ## Category
/// math
#[func]
#[capable(Texify)]
#[derive(Debug, Hash)]
pub struct BbNode(pub Content);

#[node]
impl BbNode {
    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        Ok(Self(args.expect("body")?).pack())
    }
}

impl Texify for BbNode {
    fn texify(&self, t: &mut Texifier) -> SourceResult<()> {
        t.push_str("\\mathbb{");
        self.0.texify_unparen(t)?;
        t.push_str("}");
        Ok(())
    }
}
