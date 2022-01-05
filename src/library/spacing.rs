//! Horizontal and vertical spacing between nodes.

use super::prelude::*;

/// `h`: Horizontal spacing.
pub fn h(_: &mut EvalContext, args: &mut Args) -> TypResult<Value> {
    Ok(Value::Node(Node::Spacing(
        SpecAxis::Horizontal,
        args.expect("spacing")?,
    )))
}

/// `v`: Vertical spacing.
pub fn v(_: &mut EvalContext, args: &mut Args) -> TypResult<Value> {
    Ok(Value::Node(Node::Spacing(
        SpecAxis::Vertical,
        args.expect("spacing")?,
    )))
}

/// Kinds of spacing.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum SpacingKind {
    /// A length stated in absolute values and/or relative to the parent's size.
    Linear(Linear),
    /// A length that is the fraction of the remaining free space in the parent.
    Fractional(Fractional),
}

castable! {
    SpacingKind,
    Expected: "linear or fractional",
    Value::Length(v) => Self::Linear(v.into()),
    Value::Relative(v) => Self::Linear(v.into()),
    Value::Linear(v) => Self::Linear(v),
    Value::Fractional(v) => Self::Fractional(v),
}
