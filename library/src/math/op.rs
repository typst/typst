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

impl OpNode {
    fn new(text: impl Into<EcoString>, limits: bool) -> Self {
        Self { text: text.into(), limits }
    }
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
pub fn define_operators(scope: &mut Scope) {
    let mut define = |name: &str, limits| {
        scope.define(name, OpNode { text: name.into(), limits }.pack());
    };

    // These have the same name in code and display.
    define("arccos", false);
    define("arcsin", false);
    define("arctan", false);
    define("arg", false);
    define("cos", false);
    define("cosh", false);
    define("cot", false);
    define("coth", false);
    define("csc", false);
    define("deg", false);
    define("det", true);
    define("dim", false);
    define("exp", false);
    define("gcd", true);
    define("hom", false);
    define("inf", true);
    define("ker", false);
    define("lg", false);
    define("lim", true);
    define("ln", false);
    define("log", false);
    define("max", true);
    define("min", true);
    define("Pr", true);
    define("sec", false);
    define("sin", false);
    define("sinh", false);
    define("sup", true);
    define("tan", false);
    define("tanh", false);

    // These have an extra thin space.
    scope.define("liminf", OpNode::new("lim inf", true).pack());
    scope.define("limsup", OpNode::new("lim sup", true).pack());
}

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
#[capable]
#[derive(Debug, Hash)]
pub struct FloorNode(pub Content);

#[node]
impl FloorNode {
    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        Ok(Self(args.expect("body")?).pack())
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
#[capable]
#[derive(Debug, Hash)]
pub struct CeilNode(pub Content);

#[node]
impl CeilNode {
    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        Ok(Self(args.expect("body")?).pack())
    }
}
