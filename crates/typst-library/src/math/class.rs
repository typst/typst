use super::*;

/// Forced use of a certain math class.
///
/// This is useful to treat certain symbols as if they were of a different
/// class, e.g. to make text behave like a binary operator.
///
/// # Example
/// ```example
/// $x class("relation", "<=") 5$
/// ```
///
/// Display: Class
/// Category: math
#[element(LayoutMath)]
pub struct ClassElem {
    #[required]
    pub body: Content,

    #[required]
    pub class: MathClass,
}

impl LayoutMath for ClassElem {
    fn layout_math(&self, ctx: &mut MathContext) -> SourceResult<()> {
        ctx.style(ctx.style.with_class(self.class()));
        let mut fragment = ctx.layout_fragment(&self.body())?;
        ctx.unstyle();

        fragment.set_class(self.class());
        ctx.push(fragment);
        Ok(())
    }
}