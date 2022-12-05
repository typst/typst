//! Mathematical formulas.

mod tex;

use std::fmt::Write;

use typst::model::Guard;

use self::tex::{layout_tex, Texify};
use crate::prelude::*;
use crate::text::FontFamily;

/// A piece of a mathematical formula.
#[derive(Debug, Clone, Hash)]
pub struct MathNode {
    /// The pieces of the formula.
    pub children: Vec<Content>,
    /// Whether the formula is display-level.
    pub display: bool,
}

#[node(Show, Layout, Inline, Texify)]
impl MathNode {
    fn field(&self, name: &str) -> Option<Value> {
        match name {
            "display" => Some(Value::Bool(self.display)),
            _ => None,
        }
    }
}

impl Show for MathNode {
    fn show(&self, _: &mut Vt, _: &Content, styles: StyleChain) -> SourceResult<Content> {
        let mut map = StyleMap::new();
        map.set_family(FontFamily::new("NewComputerModernMath"), styles);

        let mut realized = self
            .clone()
            .pack()
            .guarded(Guard::Base(NodeId::of::<Self>()))
            .styled_with_map(map);

        if self.display {
            realized = realized.aligned(Axes::with_x(Some(Align::Center.into())))
        }

        Ok(realized)
    }
}

impl Layout for MathNode {
    fn layout(
        &self,
        vt: &mut Vt,
        styles: StyleChain,
        _: &Regions,
    ) -> SourceResult<Fragment> {
        layout_tex(vt, &self.texify(), self.display, styles)
    }
}

impl Inline for MathNode {}

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
