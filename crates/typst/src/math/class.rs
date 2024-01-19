use unicode_math_class::MathClass;

use crate::diag::SourceResult;
use crate::foundations::{elem, Content, Packed};
use crate::math::{LayoutMath, Limits, MathContext};

/// Forced use of a certain math class.
///
/// This is useful to treat certain symbols as if they were of a different
/// class, e.g. to make a symbol behave like a relation. The class of a symbol
/// defines the way it is laid out, including spacing around it, and how its
/// scripts are attached by default. Note that the latter can always be
/// overridden using [`{limits}`](math.limits) and [`{scripts}`](math.scripts).
///
/// # Example
/// ```example
/// #let loves = math.class(
///   "relation",
///   sym.suit.heart,
/// )
///
/// $x loves y and y loves 5$
/// ```
#[elem(LayoutMath)]
pub struct ClassElem {
    /// The class to apply to the content.
    #[required]
    pub class: MathClass,

    /// The content to which the class is applied.
    #[required]
    pub body: Content,
}

impl LayoutMath for Packed<ClassElem> {
    #[typst_macros::time(name = "math.class", span = self.span())]
    fn layout_math(&self, ctx: &mut MathContext) -> SourceResult<()> {
        ctx.style(ctx.style.with_class(*self.class()));
        let mut fragment = ctx.layout_fragment(self.body())?;
        ctx.unstyle();

        fragment.set_class(*self.class());
        fragment.set_limits(Limits::for_class(*self.class()));
        ctx.push(fragment);
        Ok(())
    }
}
