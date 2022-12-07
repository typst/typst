//! Mathematical formulas.

mod tex;

use typst::model::{Guard, SequenceNode};
use unicode_segmentation::UnicodeSegmentation;

use self::tex::layout_tex;
use crate::prelude::*;
use crate::text::{FontFamily, LinebreakNode, SpaceNode, SymbolNode, TextNode};

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
        let mut t = Texifier::new(styles);
        self.texify(&mut t)?;
        layout_tex(vt, &t.finish(), self.display, styles)
    }
}

impl Inline for MathNode {}

/// Turn a math node into TeX math code.
#[capability]
trait Texify {
    /// Perform the conversion.
    fn texify(&self, t: &mut Texifier) -> SourceResult<()>;

    /// Texify the node, but trim parentheses..
    fn texify_unparen(&self, t: &mut Texifier) -> SourceResult<()> {
        let s = {
            let mut sub = Texifier::new(t.styles);
            self.texify(&mut sub)?;
            sub.finish()
        };

        let unparened = if s.starts_with("\\left(") && s.ends_with("\\right)") {
            s[6..s.len() - 7].into()
        } else {
            s
        };

        t.push_str(&unparened);
        Ok(())
    }
}

/// Builds the TeX representation of the formula.
struct Texifier<'a> {
    tex: EcoString,
    support: bool,
    space: bool,
    styles: StyleChain<'a>,
}

impl<'a> Texifier<'a> {
    /// Create a new texifier.
    fn new(styles: StyleChain<'a>) -> Self {
        Self {
            tex: EcoString::new(),
            support: false,
            space: false,
            styles,
        }
    }

    /// Finish texifier and return the TeX string.
    fn finish(self) -> EcoString {
        self.tex
    }

    /// Push a weak space.
    fn push_space(&mut self) {
        self.space = !self.tex.is_empty();
    }

    /// Mark this position as supportive. This allows a space before or after
    /// to exist.
    fn support(&mut self) {
        self.support = true;
    }

    /// Flush a space.
    fn flush(&mut self) {
        if self.space && self.support {
            self.tex.push_str("\\ ");
        }

        self.space = false;
        self.support = false;
    }

    /// Push a string.
    fn push_str(&mut self, s: &str) {
        self.flush();
        self.tex.push_str(s);
    }

    /// Escape and push a char for TeX usage.
    #[rustfmt::skip]
    fn push_escaped(&mut self, c: char) {
        self.flush();
        match c {
            ' ' => self.tex.push_str("\\ "),
            '%' | '&' | '$' | '#' => {
                self.tex.push('\\');
                self.tex.push(c);
                self.tex.push(' ');
            }
            '{' => self.tex.push_str("\\left\\{"),
            '}' => self.tex.push_str("\\right\\}"),
            '[' | '(' => {
                self.tex.push_str("\\left");
                self.tex.push(c);
            }
            ']' | ')' => {
                self.tex.push_str("\\right");
                self.tex.push(c);
            }
            'a' ..= 'z' | 'A' ..= 'Z' | '0' ..= '9' | 'Α' ..= 'Ω' | 'α' ..= 'ω' |
            '*' | '+' | '-' | '?' | '!' | '=' | '<' | '>' |
            ':' | ',' | ';' | '|' | '/' | '@' | '.' | '"' => self.tex.push(c),
            c => {
                if let Some(sym) = unicode_math::SYMBOLS
                .iter()
                .find(|sym| sym.codepoint == c) {
                    self.tex.push('\\');
                    self.tex.push_str(sym.name);
                    self.tex.push(' ');
                }
            }
        }
    }
}

impl Texify for MathNode {
    fn texify(&self, t: &mut Texifier) -> SourceResult<()> {
        for child in &self.children {
            child.texify(t)?;
        }
        Ok(())
    }
}

impl Texify for Content {
    fn texify(&self, t: &mut Texifier) -> SourceResult<()> {
        if self.is::<SpaceNode>() {
            t.push_space();
            return Ok(());
        }

        if self.is::<LinebreakNode>() {
            t.push_str("\\");
            return Ok(());
        }

        if let Some(node) = self.to::<SymbolNode>() {
            if let Some(c) = symmie::get(&node.0) {
                t.push_escaped(c);
                return Ok(());
            } else if let Some(span) = self.span() {
                bail!(span, "unknown symbol");
            }
        }

        if let Some(node) = self.to::<TextNode>() {
            t.support();
            t.push_str("\\mathrm{");
            for c in node.0.chars() {
                t.push_escaped(c);
            }
            t.push_str("}");
            t.support();
            return Ok(());
        }

        if let Some(node) = self.to::<SequenceNode>() {
            for child in &node.0 {
                child.texify(t)?;
            }
            return Ok(());
        }

        if let Some(node) = self.with::<dyn Texify>() {
            return node.texify(t);
        }

        if let Some(span) = self.span() {
            bail!(span, "not allowed here");
        }

        Ok(())
    }
}

/// An atom in a math formula: `x`, `+`, `12`.
#[derive(Debug, Hash)]
pub struct AtomNode(pub EcoString);

#[node(Texify)]
impl AtomNode {
    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        Ok(Self(args.expect("text")?).pack())
    }
}

impl Texify for AtomNode {
    fn texify(&self, t: &mut Texifier) -> SourceResult<()> {
        let multi = self.0.graphemes(true).count() > 1;
        if multi {
            t.push_str("\\mathrm{");
        }

        for c in self.0.chars() {
            let supportive = c == '|';
            if supportive {
                t.support();
            }
            t.push_escaped(c);
            if supportive {
                t.support();
            }
        }

        if multi {
            t.push_str("}");
        }

        Ok(())
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
impl FracNode {
    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        let num = args.expect("numerator")?;
        let denom = args.expect("denominator")?;
        Ok(Self { num, denom }.pack())
    }
}

impl Texify for FracNode {
    fn texify(&self, t: &mut Texifier) -> SourceResult<()> {
        t.push_str("\\frac{");
        self.num.texify_unparen(t)?;
        t.push_str("}{");
        self.denom.texify_unparen(t)?;
        t.push_str("}");
        Ok(())
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
    fn texify(&self, t: &mut Texifier) -> SourceResult<()> {
        self.base.texify(t)?;

        if let Some(sub) = &self.sub {
            t.push_str("_{");
            sub.texify_unparen(t)?;
            t.push_str("}");
        }

        if let Some(sup) = &self.sup {
            t.push_str("^{");
            sup.texify_unparen(t)?;
            t.push_str("}");
        }

        Ok(())
    }
}

/// A math alignment indicator: `&`, `&&`.
#[derive(Debug, Hash)]
pub struct AlignNode(pub usize);

#[node(Texify)]
impl AlignNode {}

impl Texify for AlignNode {
    fn texify(&self, _: &mut Texifier) -> SourceResult<()> {
        Ok(())
    }
}

/// A square root.
#[derive(Debug, Hash)]
pub struct SqrtNode(Content);

#[node(Texify)]
impl SqrtNode {
    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        Ok(Self(args.expect("body")?).pack())
    }
}

impl Texify for SqrtNode {
    fn texify(&self, t: &mut Texifier) -> SourceResult<()> {
        t.push_str("\\sqrt{");
        self.0.texify_unparen(t)?;
        t.push_str("}");
        Ok(())
    }
}

/// A column vector.
#[derive(Debug, Hash)]
pub struct VecNode(Vec<Content>);

#[node(Texify)]
impl VecNode {
    /// The kind of delimiter.
    pub const DELIM: Delimiter = Delimiter::Paren;

    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        Ok(Self(args.all()?).pack())
    }
}

impl Texify for VecNode {
    fn texify(&self, t: &mut Texifier) -> SourceResult<()> {
        let kind = match t.styles.get(Self::DELIM) {
            Delimiter::Paren => "pmatrix",
            Delimiter::Bracket => "bmatrix",
            Delimiter::Brace => "Bmatrix",
            Delimiter::Bar => "vmatrix",
        };

        t.push_str("\\begin{");
        t.push_str(kind);
        t.push_str("}");

        for component in &self.0 {
            component.texify_unparen(t)?;
            t.push_str("\\\\");
        }
        t.push_str("\\end{");
        t.push_str(kind);
        t.push_str("}");

        Ok(())
    }
}

/// A vector / matrix delimiter.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Delimiter {
    Paren,
    Bracket,
    Brace,
    Bar,
}

castable! {
    Delimiter,
    Expected: "type of bracket or bar",
    Value::Str(s) => match s.as_str() {
        "(" => Self::Paren,
        "[" => Self::Bracket,
        "{" => Self::Brace,
        "|" => Self::Bar,
        _ => Err("expected \"(\", \"[\", \"{\", or \"|\"")?,
    },
}

/// A case distinction.
#[derive(Debug, Hash)]
pub struct CasesNode(Vec<Content>);

#[node(Texify)]
impl CasesNode {
    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        Ok(Self(args.all()?).pack())
    }
}

impl Texify for CasesNode {
    fn texify(&self, t: &mut Texifier) -> SourceResult<()> {
        t.push_str("\\begin{cases}");
        for component in &self.0 {
            component.texify_unparen(t)?;
            t.push_str("\\\\");
        }
        t.push_str("\\end{cases}");
        Ok(())
    }
}
