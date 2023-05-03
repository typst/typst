use typst::eval::Scope;

use super::*;

/// A text operator in an equation.
///
/// ## Example
/// ```example
/// $ tan x = (sin x)/(cos x) $
/// $ op("custom",
///      limits: #true)_(n->oo) n $
/// ```
///
/// ## Predefined Operators
/// Typst predefines the operators `arccos`,  `arcsin`,  `arctan`,  `arg`,
/// `cos`,  `cosh`,  `cot`, `ctg`, `coth`,  `csc`,  `deg`,  `det`,  `dim`,
/// `exp`, `gcd`,  `hom`,  `mod`,  `inf`,  `ker`,  `lg`,  `lim`,  `ln`,  `log`,
/// `max`, `min`,  `Pr`,  `sec`,  `sin`,  `sinc`,  `sinh`,  `sup`,  `tan`, `tg`,
/// `tanh`, `liminf`, and `limsup`.
///
/// Display: Text Operator
/// Category: math
#[element(LayoutMath)]
pub struct OpElem {
    /// The operator's text.
    #[required]
    pub text: EcoString,

    /// Whether the operator should force attachments to display as limits.
    ///
    /// Defaults to `{false}`.
    #[default(false)]
    pub limits: bool,
}

impl LayoutMath for OpElem {
    #[tracing::instrument(skip(ctx))]
    fn layout_math(&self, ctx: &mut MathContext) -> SourceResult<()> {
        let fragment =
            ctx.layout_text(&TextElem::new(self.text()).spanned(self.span()))?;
        ctx.push(
            FrameFragment::new(ctx, fragment.into_frame())
                .with_class(MathClass::Large)
                .with_limits(self.limits(ctx.styles())),
        );
        Ok(())
    }
}

macro_rules! ops {
    ($($name:ident $(: $value:literal)? $(($tts:tt))?),* $(,)?) => {
        pub(super) fn define(math: &mut Scope) {
            $(math.define(
                stringify!($name),
                OpElem::new(ops!(@name $name $(: $value)?).into())
                    .with_limits(ops!(@limit $($tts)*))
                    .pack()
            );)*

            let dif = |d| {
                HElem::new(THIN.into()).pack()
                    + MathStyleElem::new(TextElem::packed(d)).with_italic(Some(false)).pack()
            };
            math.define("dif", dif('d'));
            math.define("Dif", dif('D'));
        }
    };
    (@name $name:ident) => { stringify!($name) };
    (@name $name:ident: $value:literal) => { $value };
    (@limit limits) => { true };
    (@limit) => { false };
}

ops! {
    arccos,
    arcsin,
    arctan,
    arg,
    cos,
    cosh,
    cot,
    coth,
    csc,
    ctg,
    deg,
    det (limits),
    dim,
    exp,
    gcd (limits),
    hom,
    id,
    im,
    inf (limits),
    ker,
    lg,
    lim (limits),
    liminf: "lim inf" (limits),
    limsup: "lim sup" (limits),
    ln,
    log,
    max (limits),
    min (limits),
    mod,
    Pr (limits),
    sec,
    sin,
    sinc,
    sinh,
    sup (limits),
    tan,
    tanh,
    tg,
    tr,
}
