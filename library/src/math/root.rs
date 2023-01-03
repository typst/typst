use super::*;

/// # Square Root
/// A square root.
///
/// _Note:_ Non-square roots are not yet supported.
///
/// ## Example
/// ```
/// $ sqrt(x^2) = x = sqrt(x)^2 $
/// ```
///
/// ## Parameters
/// - body: Content (positional, required)
///   The expression to take the square root of.
///
/// ## Category
/// math
#[func]
#[capable(Texify)]
#[derive(Debug, Hash)]
pub struct SqrtNode(pub Content);

#[node]
impl SqrtNode {
    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        Ok(Self(args.expect("body")?).pack())
    }
}

impl Texify for SqrtNode {
    fn texify(&self, t: &mut Texifier) -> SourceResult<()> {
        t.push_str("\\sqrt{");
        self.0.texify(t)?;
        t.push_str("}");
        Ok(())
    }
}
