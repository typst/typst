//! Mathematical formulas.

mod tex;

use std::fmt::Write;

use self::tex::{layout_tex, Texify};
use crate::layout::BlockSpacing;
use crate::prelude::*;
use crate::text::{FallbackList, FontFamily, TextNode};

/// A piece of a mathematical formula.
#[derive(Debug, Clone, Hash)]
pub struct MathNode {
    /// The pieces of the formula.
    pub children: Vec<Content>,
    /// Whether the formula is display-level.
    pub display: bool,
}

#[node(Show, Finalize, LayoutInline, Texify)]
impl MathNode {
    /// The spacing above display math.
    #[property(resolve, shorthand(around))]
    pub const ABOVE: Option<BlockSpacing> = Some(Ratio::one().into());
    /// The spacing below display math.
    #[property(resolve, shorthand(around))]
    pub const BELOW: Option<BlockSpacing> = Some(Ratio::one().into());

    fn field(&self, name: &str) -> Option<Value> {
        match name {
            "display" => Some(Value::Bool(self.display)),
            _ => None,
        }
    }
}

impl Show for MathNode {
    fn unguard_parts(&self, _: RecipeId) -> Content {
        self.clone().pack()
    }

    fn show(&self, _: Tracked<dyn World>, _: StyleChain) -> SourceResult<Content> {
        Ok(self.clone().pack())
    }
}

impl Finalize for MathNode {
    fn finalize(
        &self,
        _: Tracked<dyn World>,
        styles: StyleChain,
        mut realized: Content,
    ) -> SourceResult<Content> {
        realized = realized.styled(
            TextNode::FAMILY,
            FallbackList(vec![FontFamily::new("NewComputerModernMath")]),
        );

        if self.display {
            realized = realized
                .aligned(Axes::with_x(Some(Align::Center.into())))
                .spaced(styles.get(Self::ABOVE), styles.get(Self::BELOW))
        }

        Ok(realized)
    }
}

impl LayoutInline for MathNode {
    fn layout_inline(
        &self,
        world: Tracked<dyn World>,
        _: &Regions,
        styles: StyleChain,
    ) -> SourceResult<Vec<Frame>> {
        Ok(vec![layout_tex(&self.texify(), self.display, world, styles)?])
    }
}

impl Texify for MathNode {
    fn texify(&self) -> EcoString {
        self.children.iter().map(Texify::texify).collect()
    }
}

/// An atom in a math formula: `x`, `+`, `12`.
#[derive(Debug, Hash)]
pub struct AtomNode(pub EcoString);

#[node(Texify)]
impl AtomNode {}

impl Texify for AtomNode {
    fn texify(&self) -> EcoString {
        self.0.chars().map(escape_char).collect()
    }
}

/// A fraction in a mathematical formula.
#[derive(Debug, Hash)]
pub struct FracNode {
    /// The numerator.
    pub num: Content,
    /// The denominator.
    pub denom: Content,
}

#[node(Texify)]
impl FracNode {}

impl Texify for FracNode {
    fn texify(&self) -> EcoString {
        format_eco!(
            "\\frac{{{}}}{{{}}}",
            unparen(self.num.texify()),
            unparen(self.denom.texify())
        )
    }
}

/// A sub- and/or superscript in a mathematical formula.
#[derive(Debug, Hash)]
pub struct ScriptNode {
    /// The base.
    pub base: Content,
    /// The subscript.
    pub sub: Option<Content>,
    /// The superscript.
    pub sup: Option<Content>,
}

#[node(Texify)]
impl ScriptNode {}

impl Texify for ScriptNode {
    fn texify(&self) -> EcoString {
        let mut tex = self.base.texify();

        if let Some(sub) = &self.sub {
            write!(tex, "_{{{}}}", unparen(sub.texify())).unwrap();
        }

        if let Some(sup) = &self.sup {
            write!(tex, "^{{{}}}", unparen(sup.texify())).unwrap();
        }

        tex
    }
}

/// A math alignment indicator: `&`, `&&`.
#[derive(Debug, Hash)]
pub struct AlignNode(pub usize);

#[node(Texify)]
impl AlignNode {}

impl Texify for AlignNode {
    fn texify(&self) -> EcoString {
        EcoString::new()
    }
}

/// Escape a char for TeX usage.
#[rustfmt::skip]
fn escape_char(c: char) -> EcoString {
    match c {
        '{' | '}' | '%' | '&' | '$' | '#' => format_eco!(" \\{c} "),
        'a' ..= 'z' | 'A' ..= 'Z' | '0' ..= '9' | 'Α' ..= 'Ω' | 'α' ..= 'ω' |
        '*' | '+' | '-' | '[' | '(' | ']' | ')' | '?' | '!' | '=' | '<' | '>' |
        ':' | ',' | ';' | '|' | '/' | '@' | '.' | '"' => c.into(),
        c => unicode_math::SYMBOLS
            .iter()
            .find(|sym| sym.codepoint == c)
            .map(|sym| format_eco!("\\{} ", sym.name))
            .unwrap_or_default(),
    }
}

/// Trim grouping parenthesis≤.
fn unparen(s: EcoString) -> EcoString {
    if s.starts_with('(') && s.ends_with(')') {
        s[1..s.len() - 1].into()
    } else {
        s
    }
}
