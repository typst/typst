use ecow::EcoString;

use crate::foundations::{elem, Content, NativeElement, Scope};
use crate::layout::HElem;
use crate::math::{upright, Mathy, THIN};
use crate::text::TextElem;

/// A text operator in an equation.
///
/// # Example
/// ```example
/// $ tan x = (sin x)/(cos x) $
/// $ op("custom",
///      limits: #true)_(n->oo) n $
/// ```
///
/// # Predefined Operators { #predefined }
/// Typst predefines the operators `arccos`, `arcsin`, `arctan`, `arg`, `cos`,
/// `cosh`, `cot`, `coth`, `csc`, `csch`, `ctg`, `deg`, `det`, `dim`, `exp`,
/// `gcd`, `lcm`, `hom`, `id`, `im`, `inf`, `ker`, `lg`, `lim`, `liminf`,
/// `limsup`, `ln`, `log`, `max`, `min`, `mod`, `Pr`, `sec`, `sech`, `sin`,
/// `sinc`, `sinh`, `sup`, `tan`, `tanh`, `tg` and `tr`.
#[elem(title = "Text Operator", Mathy)]
pub struct OpElem {
    /// The operator's text.
    #[required]
    pub text: Content,

    /// Whether the operator should show attachments as limits in display mode.
    #[default(false)]
    pub limits: bool,
}

macro_rules! ops {
    ($($name:ident $(: $value:literal)? $(($tts:tt))?),* $(,)?) => {
        pub(super) fn define(math: &mut Scope) {
            $({
                let operator = EcoString::from(ops!(@name $name $(: $value)?));
                math.define(
                    stringify!($name),
                    OpElem::new(TextElem::new(operator).into())
                        .with_limits(ops!(@limit $($tts)*))
                        .pack()
                );
            })*

            let dif = |d| {
                HElem::new(THIN.into()).with_weak(true).pack()
                    + upright(TextElem::packed(d))
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
    csch,
    ctg,
    deg,
    det (limits),
    dim,
    exp,
    gcd (limits),
    lcm (limits),
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
    sech,
    sin,
    sinc,
    sinh,
    sup (limits),
    tan,
    tanh,
    tg,
    tr,
}
