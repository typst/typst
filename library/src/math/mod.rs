//! Mathematical formulas.

mod matrix;
mod style;
mod tex;

pub use self::matrix::*;
pub use self::style::*;

use typst::model::{Guard, SequenceNode};
use unicode_segmentation::UnicodeSegmentation;

use self::tex::layout_tex;
use crate::prelude::*;
use crate::text::{FontFamily, LinebreakNode, SpaceNode, SymbolNode, TextNode};

/// A piece of a mathematical formula.
///
/// # Parameters
/// - items: Content (positional, variadic)
///   The individual parts of the formula.
/// - block: bool (named)
///   Whether the formula is displayed as a separate block.
///
/// # Tags
/// - math
#[func]
#[capable(Show, Layout, Inline, Texify)]
#[derive(Debug, Clone, Hash)]
pub struct MathNode {
    /// Whether the formula is displayed as a separate block.
    pub block: bool,
    /// The pieces of the formula.
    pub children: Vec<Content>,
}

#[node]
impl MathNode {
    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        let block = args.named("block")?.unwrap_or(false);
        let children = args.all()?;
        Ok(Self { block, children }.pack())
    }

    fn field(&self, name: &str) -> Option<Value> {
        match name {
            "block" => Some(Value::Bool(self.block)),
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

        if self.block {
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
        _: Regions,
    ) -> SourceResult<Fragment> {
        let mut t = Texifier::new(styles);
        self.texify(&mut t)?;
        Ok(layout_tex(vt, &t.finish(), self.block, styles)
            .unwrap_or(Fragment::frame(Frame::new(Size::zero()))))
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
///
/// # Parameters
/// - text: EcoString (positional, required)
///   The atom's text.
///
/// # Tags
/// - math
#[func]
#[capable(Texify)]
#[derive(Debug, Hash)]
pub struct AtomNode(pub EcoString);

#[node]
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

/// An accented node.
///
/// # Parameters
/// - base: Content (positional, required)
///   The base to which the accent is applied.
/// - accent: Content (positional, required)
///   The accent to apply to the base.
///
/// # Tags
/// - math
#[func]
#[capable(Texify)]
#[derive(Debug, Hash)]
pub struct AccNode {
    /// The accent base.
    pub base: Content,
    /// The Unicode accent character.
    pub accent: char,
}

#[node]
impl AccNode {
    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        let base = args.expect("base")?;
        let Spanned { v, span } = args.expect::<Spanned<Content>>("accent")?;
        let accent = match extract(&v) {
            Some(Ok(c)) => c,
            Some(Err(msg)) => bail!(span, "{}", msg),
            None => bail!(span, "not an accent"),
        };
        Ok(Self { base, accent }.pack())
    }
}

#[rustfmt::skip]
fn extract(content: &Content) -> Option<Result<char, &'static str>> {
    let MathNode { children, .. } = content.to::<MathNode>()?;
    let [child] = children.as_slice() else { return None };
    let c = if let Some(atom) = child.to::<AtomNode>() {
        let mut chars = atom.0.chars();
        chars.next().filter(|_| chars.next().is_none())?
    } else if let Some(symbol) = child.to::<SymbolNode>() {
        match symmie::get(&symbol.0) {
            Some(c) => c,
            None => return Some(Err("unknown symbol")),
        }
    } else {
        return None;
    };

    Some(Ok(match c {
        '`' | '\u{300}' => '\u{300}',              // Grave
        '´' | '\u{301}' => '\u{301}',              // Acute
        '^' | '\u{302}' => '\u{302}',              // Circumflex
        '~' | '\u{223C}' | '\u{303}' => '\u{303}', // Tilde
        '¯' | '\u{304}' => '\u{304}',              // Macron
        '‾' | '\u{305}' => '\u{305}',              // Overline
        '˘' | '\u{306}' => '\u{306}',              // Breve
        '.' | '\u{22C5}' | '\u{307}' => '\u{307}', // Dot
        '¨' | '\u{308}' => '\u{308}',              // Diaeresis
        'ˇ' | '\u{30C}' => '\u{30C}',              // Caron
        '→' | '\u{20D7}' => '\u{20D7}',            // Arrow
        _ => return None,
    }))
}

impl Texify for AccNode {
    fn texify(&self, t: &mut Texifier) -> SourceResult<()> {
        if let Some(sym) = unicode_math::SYMBOLS.iter().find(|sym| {
            sym.codepoint == self.accent
                && sym.atom_type == unicode_math::AtomType::Accent
        }) {
            t.push_str("\\");
            t.push_str(sym.name);
            t.push_str("{");
            self.base.texify(t)?;
            t.push_str("}");
        } else {
            self.base.texify(t)?;
        }
        Ok(())
    }
}

/// A fraction.
///
/// # Parameters
/// - num: Content (positional, required)
///   The fraction's numerator.
/// - denom: Content (positional, required)
///   The fraction's denominator.
///
/// # Tags
/// - math
#[func]
#[capable(Texify)]
#[derive(Debug, Hash)]
pub struct FracNode {
    /// The numerator.
    pub num: Content,
    /// The denominator.
    pub denom: Content,
}

#[node]
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

/// A binomial.
///
/// # Parameters
/// - upper: Content (positional, required)
///   The binomial's upper index.
/// - lower: Content (positional, required)
///   The binomial's lower index.
///
/// # Tags
/// - math
#[func]
#[capable(Texify)]
#[derive(Debug, Hash)]
pub struct BinomNode {
    /// The upper index.
    pub upper: Content,
    /// The lower index.
    pub lower: Content,
}

#[node]
impl BinomNode {
    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        let upper = args.expect("upper index")?;
        let lower = args.expect("lower index")?;
        Ok(Self { upper, lower }.pack())
    }
}

impl Texify for BinomNode {
    fn texify(&self, t: &mut Texifier) -> SourceResult<()> {
        t.push_str("\\binom{");
        self.upper.texify(t)?;
        t.push_str("}{");
        self.lower.texify(t)?;
        t.push_str("}");
        Ok(())
    }
}

/// A sub- and/or superscript.
///
/// # Parameters
/// - base: Content (positional, required)
///   The base to which the applies the sub- and/or superscript.
/// - sub: Content (named)
///   The subscript.
/// - sup: Content (named)
///   The superscript.
///
/// # Tags
/// - math
#[func]
#[capable(Texify)]
#[derive(Debug, Hash)]
pub struct ScriptNode {
    /// The base.
    pub base: Content,
    /// The subscript.
    pub sub: Option<Content>,
    /// The superscript.
    pub sup: Option<Content>,
}

#[node]
impl ScriptNode {
    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        let base = args.expect("base")?;
        let sub = args.named("sub")?;
        let sup = args.named("sup")?;
        Ok(Self { base, sub, sup }.pack())
    }
}

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

/// A math alignment point: `&`, `&&`.
///
/// # Parameters
/// - index: usize (positional, required)
///   The alignment point's index.
///
/// # Tags
/// - math
#[func]
#[capable(Texify)]
#[derive(Debug, Hash)]
pub struct AlignPointNode(pub NonZeroUsize);

#[node]
impl AlignPointNode {
    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        Ok(Self(args.expect("index")?).pack())
    }
}

impl Texify for AlignPointNode {
    fn texify(&self, _: &mut Texifier) -> SourceResult<()> {
        Ok(())
    }
}

/// A square root.
///
/// # Parameters
/// - body: Content (positional, required)
///   The expression to take the square root of.
///
/// # Tags
/// - math
#[func]
#[capable(Texify)]
#[derive(Debug, Hash)]
pub struct SqrtNode(pub Content);

#[node]
impl SqrtNode {
    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        Ok(Self(args.expect("body")?).pack())
    }
}

impl Texify for SqrtNode {
    fn texify(&self, t: &mut Texifier) -> SourceResult<()> {
        t.push_str("\\sqrt{");
        self.0.texify(t)?;
        t.push_str("}");
        Ok(())
    }
}

/// A floored expression.
///
/// # Parameters
/// - body: Content (positional, required)
///   The expression to floor.
///
/// # Tags
/// - math
#[func]
#[capable(Texify)]
#[derive(Debug, Hash)]
pub struct FloorNode(pub Content);

#[node]
impl FloorNode {
    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        Ok(Self(args.expect("body")?).pack())
    }
}

impl Texify for FloorNode {
    fn texify(&self, t: &mut Texifier) -> SourceResult<()> {
        t.push_str("\\left\\lfloor ");
        self.0.texify(t)?;
        t.push_str("\\right\\rfloor ");
        Ok(())
    }
}

/// A ceiled expression.
///
/// # Parameters
/// - body: Content (positional, required)
///   The expression to ceil.
///
/// # Tags
/// - math
#[func]
#[capable(Texify)]
#[derive(Debug, Hash)]
pub struct CeilNode(pub Content);

#[node]
impl CeilNode {
    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        Ok(Self(args.expect("body")?).pack())
    }
}

impl Texify for CeilNode {
    fn texify(&self, t: &mut Texifier) -> SourceResult<()> {
        t.push_str("\\left\\lceil ");
        self.0.texify(t)?;
        t.push_str("\\right\\rceil ");
        Ok(())
    }
}
