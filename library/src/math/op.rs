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

/// Hook up all operators.
pub(super) fn define_operators(math: &mut Scope) {
    math.define("arccos", op("arccos", false));
    math.define("arcsin", op("arcsin", false));
    math.define("arctan", op("arctan", false));
    math.define("arg", op("arg", false));
    math.define("cos", op("cos", false));
    math.define("cosh", op("cosh", false));
    math.define("cot", op("cot", false));
    math.define("coth", op("coth", false));
    math.define("csc", op("csc", false));
    math.define("deg", op("deg", false));
    math.define("det", op("det", true));
    math.define("dim", op("dim", false));
    math.define("exp", op("exp", false));
    math.define("gcd", op("gcd", true));
    math.define("hom", op("hom", false));
    math.define("inf", op("inf", true));
    math.define("ker", op("ker", false));
    math.define("lg", op("lg", false));
    math.define("lim", op("lim", true));
    math.define("ln", op("ln", false));
    math.define("log", op("log", false));
    math.define("max", op("max", true));
    math.define("min", op("min", true));
    math.define("Pr", op("Pr", true));
    math.define("sec", op("sec", false));
    math.define("sin", op("sin", false));
    math.define("sinh", op("sinh", false));
    math.define("sup", op("sup", true));
    math.define("tan", op("tan", false));
    math.define("tanh", op("tanh", false));
    math.define("liminf", op("lim inf", true));
    math.define("limsup", op("lim sup", true));
}

fn op(name: &str, limits: bool) -> Content {
    OpNode { text: name.into(), limits }.pack()
}
