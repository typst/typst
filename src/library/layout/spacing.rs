use crate::library::prelude::*;

/// Horizontal spacing.
pub struct HNode;

#[node]
impl HNode {
    fn construct(_: &mut Context, args: &mut Args) -> TypResult<Content> {
        Ok(Content::Horizontal(args.expect("spacing")?))
    }
}

/// Vertical spacing.
pub struct VNode;

#[node]
impl VNode {
    fn construct(_: &mut Context, args: &mut Args) -> TypResult<Content> {
        Ok(Content::Vertical(args.expect("spacing")?))
    }
}

/// Kinds of spacing.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Spacing {
    /// Spacing specified in absolute terms and relative to the parent's size.
    Relative(Relative<Length>),
    /// Spacing specified as a fraction of the remaining free space in the parent.
    Fractional(Fraction),
}

impl Spacing {
    /// Whether this is fractional spacing.
    pub fn is_fractional(self) -> bool {
        matches!(self, Self::Fractional(_))
    }
}

impl From<Length> for Spacing {
    fn from(length: Length) -> Self {
        Self::Relative(length.into())
    }
}

castable! {
    Spacing,
    Expected: "relative length or fraction",
    Value::Length(v) => Self::Relative(v.into()),
    Value::Ratio(v) => Self::Relative(v.into()),
    Value::Relative(v) => Self::Relative(v),
    Value::Fraction(v) => Self::Fractional(v),
}
