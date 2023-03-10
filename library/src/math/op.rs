use typst::eval::Scope;

use super::*;

/// A text operator in a math formula.
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
/// `max`, `min`,  `Pr`,  `sec`,  `sin`,  `sinh`,  `sup`,  `tan`, `tg`, `tanh`,
/// `liminf`, and `limsup`.
///
/// Display: Text Operator
/// Category: math
#[node(LayoutMath)]
pub struct OpNode {
    /// The operator's text.
    #[required]
    pub text: EcoString,

    /// Whether the operator should force attachments to display as limits.
    ///
    /// Defaults to `{false}`.
    #[default(false)]
    pub limits: bool,
}

impl LayoutMath for OpNode {
    fn layout_math(&self, ctx: &mut MathContext) -> SourceResult<()> {
        let frame = ctx.layout_content(&TextNode::packed(self.text()))?;
        ctx.push(
            FrameFragment::new(ctx, frame)
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
                OpNode::new(ops!(@name $name $(: $value)?).into())
                    .with_limits(ops!(@limit $($tts)*))
                    .pack()
            );)*

            let dif = |d| {
                HNode::new(THIN.into()).pack()
                    + UprightNode::new(TextNode::packed(d)).pack()
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
    ctg,
    coth,
    csc,
    deg,
    det (limits),
    dim,
    exp,
    gcd (limits),
    hom,
    mod,
    inf (limits),
    ker,
    lg,
    lim (limits),
    ln,
    log,
    max (limits),
    min (limits),
    Pr (limits),
    sec,
    sin,
    sinh,
    sup (limits),
    tan,
    tg,
    tanh,
    liminf: "lim inf" (limits),
    limsup: "lim sup" (limits),
}
