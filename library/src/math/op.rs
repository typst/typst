use typst::model::Scope;

use super::*;

/// # Text Operator
/// A text operator in a math formula.
///
/// ## Parameters
/// - text: EcoString (positional, required)
///   The operator's text.
/// - limits: bool (named)
///   Whether the operator should display sub- and superscripts as limits.
///
///   Defaults to `true`.
///
/// ## Category
/// math
#[func]
#[capable(LayoutMath)]
#[derive(Debug, Hash)]
pub struct OpNode {
    /// The operator's text.
    pub text: EcoString,
    /// Whether the operator should display sub- and superscripts as limits.
    pub limits: bool,
}

#[node]
impl OpNode {
    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        Ok(Self {
            text: args.expect("text")?,
            limits: args.named("limits")?.unwrap_or(true),
        }
        .pack())
    }
}

impl LayoutMath for OpNode {
    fn layout_math(&self, ctx: &mut MathContext) -> SourceResult<()> {
        let frame = ctx.layout_non_math(&TextNode(self.text.clone()).pack())?;
        ctx.push(FrameFragment {
            frame,
            class: MathClass::Large,
            limits: self.limits,
        });
        Ok(())
    }
}

macro_rules! ops {
    ($($name:ident $(: $value:literal)? $(($tts:tt))?),* $(,)?) => {
        pub(super) fn define(math: &mut Scope) {
            $(math.define(
                stringify!($name),
                OpNode {
                    text: ops!(@name $name $(: $value)?).into(),
                    limits: ops!(@limit $($tts)*),
                }.pack()
            );)*
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
    tanh,
    liminf: "lim inf" (limits),
    limsup: "lim sup" (limits),
}
