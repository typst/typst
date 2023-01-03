use super::*;

/// # Floor
/// A floored expression.
///
/// ## Example
/// ```
/// $ floor(x/2) $
/// ```
///
/// ## Parameters
/// - body: Content (positional, required)
///   The expression to floor.
///
/// ## Category
/// math
#[func]
#[capable(Texify)]
#[derive(Debug, Hash)]
pub struct FloorNode(pub Content);

#[node]
impl FloorNode {
    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        Ok(Self(args.expect("body")?).pack())
    }
}

impl Texify for FloorNode {
    fn texify(&self, t: &mut Texifier) -> SourceResult<()> {
        t.push_str("\\left\\lfloor ");
        self.0.texify(t)?;
        t.push_str("\\right\\rfloor ");
        Ok(())
    }
}

/// # Ceil
/// A ceiled expression.
///
/// ## Example
/// ```
/// $ ceil(x/2) $
/// ```
///
/// ## Parameters
/// - body: Content (positional, required)
///   The expression to ceil.
///
/// ## Category
/// math
#[func]
#[capable(Texify)]
#[derive(Debug, Hash)]
pub struct CeilNode(pub Content);

#[node]
impl CeilNode {
    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        Ok(Self(args.expect("body")?).pack())
    }
}

impl Texify for CeilNode {
    fn texify(&self, t: &mut Texifier) -> SourceResult<()> {
        t.push_str("\\left\\lceil ");
        self.0.texify(t)?;
        t.push_str("\\right\\rceil ");
        Ok(())
    }
}
