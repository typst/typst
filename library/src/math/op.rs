use typst::model::Scope;

use super::*;

/// # Text Operator
/// A text operator in a math formula.
///
/// ## Parameters
/// - text: EcoString (positional, required)
///   The operator's text.
/// - limits: bool (named)
///   Whether the operator should force attachments to display as limits.
///
///   Defaults to `{false}`.
///
/// ## Category
/// math
#[func]
#[capable(LayoutMath)]
#[derive(Debug, Hash)]
pub struct OpNode {
    /// The operator's text.
    pub text: EcoString,
    /// Whether the operator should force attachments to display as limits.
    pub limits: bool,
}

#[node]
impl OpNode {
    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        Ok(Self {
            text: args.expect("text")?,
            limits: args.named("limits")?.unwrap_or(false),
        }
        .pack())
    }
}

impl LayoutMath for OpNode {
    fn layout_math(&self, ctx: &mut MathContext) -> SourceResult<()> {
        let frame = ctx.layout_content(&TextNode(self.text.clone()).pack())?;
        ctx.push(
            FrameFragment::new(ctx, frame)
                .with_class(MathClass::Large)
                .with_limits(self.limits),
        );
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

            let dif = |d| HNode::strong(THIN).pack() + UprightNode(TextNode::packed(d)).pack();
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
