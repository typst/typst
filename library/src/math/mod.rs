//! Mathematical formulas.

mod accent;
mod atom;
mod frac;
mod group;
mod matrix;
mod root;
mod script;
mod style;
mod tex;

pub use self::accent::*;
pub use self::atom::*;
pub use self::frac::*;
pub use self::group::*;
pub use self::matrix::*;
pub use self::root::*;
pub use self::script::*;
pub use self::style::*;

use typst::model::{Guard, SequenceNode};
use unicode_segmentation::UnicodeSegmentation;

use self::tex::layout_tex;
use crate::prelude::*;
use crate::text::{FontFamily, LinebreakNode, SpaceNode, SymbolNode, TextNode};

/// # Math
/// A mathematical formula.
///
/// ## Syntax
/// This function also has dedicated syntax: Write mathematical markup within
/// dollar signs to create a formula. Starting and ending the formula with at
/// least one space lifts it into a separate block that is centered
/// horizontally.
///
/// In math, single letters are always displayed as is. Multiple letters,
/// however, are interpreted as variables, symbols or functions. To display
/// multiple letters verbatim, you can place them into quotes. Math mode also
/// supports extra shorthands to easily type various arrows and other symbols.
/// The page on the [`symbol`](@symbol) function lists all of them.
///
/// When a variable and a symbol share the same name, the variable is preferred.
/// To force the symbol, surround it with colons. To access a variable with a
/// single letter name, you can prefix it with a `#`.
///
/// In math mode, the arguments to a function call are always parsed as
/// mathematical content. To work with other kinds of values, you first need to
/// enter a code block using the `[$#{..}$]` syntax.
///
/// ## Example
/// ```
/// #set text("Latin Modern Roman")
///
/// Let $a$, $b$, and $c$ be the side
/// lengths of right-angled triangle.
/// Then, we know that:
/// $ a^2 + b^2 = c^2 $
///
/// Prove by induction:
/// $ sum_(k=1)^n k = (n(n+1)) / 2 $
///
/// We define the following set:
/// $ cal(A) :=
///     { x in RR | x "is natural" } $
/// ```
///
/// ## Parameters
/// - items: Content (positional, variadic)
///   The individual parts of the formula.
///
/// - block: bool (named)
///   Whether the formula is displayed as a separate block.
///
/// ## Category
/// math
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
    fn show(&self, _: &mut Vt, _: &Content, _: StyleChain) -> SourceResult<Content> {
        let mut realized = self.clone().pack().guarded(Guard::Base(NodeId::of::<Self>()));
        if self.block {
            realized = realized.aligned(Axes::with_x(Some(Align::Center.into())))
        }
        Ok(realized)
    }
}

impl Finalize for MathNode {
    fn finalize(&self, realized: Content) -> Content {
        realized.styled(
            TextNode::FAMILY,
            FallbackList(vec![FontFamily::new("New Computer Modern Math")]),
        )
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

/// # Alignment Point
/// A math alignment point: `&`, `&&`.
///
/// ## Parameters
/// - index: usize (positional, required)
///   The alignment point's index.
///
/// ## Category
/// math
#[func]
#[capable(Texify)]
#[derive(Debug, Hash)]
pub struct AlignPointNode;

#[node]
impl AlignPointNode {
    fn construct(_: &Vm, _: &mut Args) -> SourceResult<Content> {
        Ok(Self.pack())
    }
}

impl Texify for AlignPointNode {
    fn texify(&self, _: &mut Texifier) -> SourceResult<()> {
        Ok(())
    }
}
