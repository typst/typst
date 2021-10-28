use super::prelude::*;

/// `h`: Horizontal spacing.
pub fn h(_: &mut EvalContext, args: &mut Args) -> TypResult<Value> {
    let mut template = Template::new();
    template.spacing(GenAxis::Inline, args.expect("spacing")?);
    Ok(Value::Template(template))
}

/// `v`: Vertical spacing.
pub fn v(_: &mut EvalContext, args: &mut Args) -> TypResult<Value> {
    let mut template = Template::new();
    template.spacing(GenAxis::Block, args.expect("spacing")?);
    Ok(Value::Template(template))
}

/// Kinds of spacing.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Spacing {
    /// A length stated in absolute values and/or relative to the parent's size.
    Linear(Linear),
    /// A length that is the fraction of the remaining free space in the parent.
    Fractional(Fractional),
}

castable! {
    Spacing: "linear or fractional",
    Value::Length(v) => Self::Linear(v.into()),
    Value::Relative(v) => Self::Linear(v.into()),
    Value::Linear(v) => Self::Linear(v),
    Value::Fractional(v) => Self::Fractional(v),
}
