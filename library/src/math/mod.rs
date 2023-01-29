//! Mathematical formulas.

#[macro_use]
mod ctx;
mod accent;
mod align;
mod attach;
mod frac;
mod fragment;
mod lr;
mod matrix;
mod op;
mod root;
mod row;
mod spacing;
mod stack;
mod stretch;
mod style;
mod symbols;

pub use self::accent::*;
pub use self::align::*;
pub use self::attach::*;
pub use self::frac::*;
pub use self::lr::*;
pub use self::matrix::*;
pub use self::op::*;
pub use self::root::*;
pub use self::stack::*;
pub use self::style::*;

use ttf_parser::GlyphId;
use ttf_parser::Rect;
use typst::font::Font;
use typst::model::{Guard, Module, Scope, SequenceNode};
use unicode_math_class::MathClass;

use self::ctx::*;
use self::fragment::*;
use self::row::*;
use self::spacing::*;
use crate::layout::HNode;
use crate::layout::ParNode;
use crate::layout::Spacing;
use crate::prelude::*;
use crate::text::LinebreakNode;
use crate::text::TextNode;
use crate::text::TextSize;
use crate::text::{families, variant, FallbackList, FontFamily, SpaceNode};

/// Create a module with all math definitions.
pub fn module(sym: &Module) -> Module {
    let mut math = Scope::deduplicating();
    math.def_func::<FormulaNode>("formula");

    // Grouping.
    math.def_func::<LrNode>("lr");
    math.def_func::<AbsFunc>("abs");
    math.def_func::<NormFunc>("norm");
    math.def_func::<FloorFunc>("floor");
    math.def_func::<CeilFunc>("ceil");

    // Attachments and accents.
    math.def_func::<AttachNode>("attach");
    math.def_func::<ScriptsNode>("scripts");
    math.def_func::<LimitsNode>("limits");
    math.def_func::<AccentNode>("accent");
    math.def_func::<UnderlineNode>("underline");
    math.def_func::<OverlineNode>("overline");
    math.def_func::<UnderbraceNode>("underbrace");
    math.def_func::<OverbraceNode>("overbrace");
    math.def_func::<UnderbracketNode>("underbracket");
    math.def_func::<OverbracketNode>("overbracket");

    // Fractions and matrix-likes.
    math.def_func::<FracNode>("frac");
    math.def_func::<BinomNode>("binom");
    math.def_func::<VecNode>("vec");
    math.def_func::<CasesNode>("cases");

    // Roots.
    math.def_func::<SqrtNode>("sqrt");
    math.def_func::<RootNode>("root");

    // Styles.
    math.def_func::<BoldNode>("bold");
    math.def_func::<ItalicNode>("italic");
    math.def_func::<SerifNode>("serif");
    math.def_func::<SansNode>("sans");
    math.def_func::<CalNode>("cal");
    math.def_func::<FrakNode>("frak");
    math.def_func::<MonoNode>("mono");
    math.def_func::<BbNode>("bb");

    // Text operators.
    math.def_func::<OpNode>("op");
    op::define(&mut math);

    // Symbols and spacing.
    symbols::define(&mut math);
    spacing::define(&mut math);
    math.copy_from(sym.scope());

    Module::new("math").with_scope(math)
}

/// # Formula
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
/// The [text](/docs/reference/text/) and [math](/docs/reference/math/) sections
/// list all of them.
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
/// - body: Content (positional, required) The contents of the formula.
///
/// - block: bool (named) Whether the formula is displayed as a separate block.
///
/// ## Category
/// math
#[func]
#[capable(Show, Finalize, Layout, Inline, LayoutMath)]
#[derive(Debug, Clone, Hash)]
pub struct FormulaNode {
    /// Whether the formula is displayed as a separate block.
    pub block: bool,
    /// The content of the formula.
    pub body: Content,
}

#[node]
impl FormulaNode {
    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        let body = args.expect("body")?;
        let block = args.named("block")?.unwrap_or(false);
        Ok(Self { block, body }.pack())
    }

    fn field(&self, name: &str) -> Option<Value> {
        match name {
            "body" => Some(Value::Content(self.body.clone())),
            "block" => Some(Value::Bool(self.block)),
            _ => None,
        }
    }
}

impl Show for FormulaNode {
    fn show(&self, _: &mut Vt, _: &Content, _: StyleChain) -> SourceResult<Content> {
        let mut realized = self.clone().pack().guarded(Guard::Base(NodeId::of::<Self>()));
        if self.block {
            realized = realized.aligned(Axes::with_x(Some(Align::Center.into())))
        }
        Ok(realized)
    }
}

impl Finalize for FormulaNode {
    fn finalize(&self, realized: Content) -> Content {
        realized.styled(
            TextNode::FAMILY,
            FallbackList(vec![FontFamily::new("New Computer Modern Math")]),
        )
    }
}

impl Layout for FormulaNode {
    fn layout(
        &self,
        vt: &mut Vt,
        styles: StyleChain,
        regions: Regions,
    ) -> SourceResult<Fragment> {
        // Find a math font.
        let variant = variant(styles);
        let world = vt.world();
        let Some(font) = families(styles)
            .find_map(|family| {
                let id = world.book().select(family, variant)?;
                let font = world.font(id)?;
                let _ = font.ttf().tables().math?.constants?;
                Some(font)
            })
        else {
            if let Some(span) = self.body.span() {
                bail!(span, "current font does not support math");
            }
            return Ok(Fragment::frame(Frame::new(Size::zero())))
        };

        let mut ctx = MathContext::new(vt, styles, regions, &font, self.block);
        let frame = ctx.layout_frame(self)?;
        Ok(Fragment::frame(frame))
    }
}

impl Inline for FormulaNode {}

#[capability]
trait LayoutMath {
    fn layout_math(&self, ctx: &mut MathContext) -> SourceResult<()>;
}

impl LayoutMath for FormulaNode {
    fn layout_math(&self, ctx: &mut MathContext) -> SourceResult<()> {
        self.body.layout_math(ctx)
    }
}

impl LayoutMath for Content {
    fn layout_math(&self, ctx: &mut MathContext) -> SourceResult<()> {
        if self.is::<SpaceNode>() {
            ctx.push(MathFragment::Space);
            return Ok(());
        }

        if self.is::<LinebreakNode>() {
            ctx.push(MathFragment::Linebreak);
            return Ok(());
        }

        if let Some(node) = self.to::<HNode>() {
            if let Spacing::Relative(rel) = node.amount {
                if rel.rel.is_zero() {
                    ctx.push(MathFragment::Spacing(
                        rel.abs.resolve(ctx.outer.chain(&ctx.map)),
                    ));
                }
            }
            return Ok(());
        }

        if let Some(node) = self.to::<TextNode>() {
            ctx.layout_text(&node.0)?;
            return Ok(());
        }

        if let Some(node) = self.to::<SequenceNode>() {
            for child in &node.0 {
                child.layout_math(ctx)?;
            }
            return Ok(());
        }

        if let Some(node) = self.with::<dyn LayoutMath>() {
            return node.layout_math(ctx);
        }

        let frame = ctx.layout_non_math(self)?;
        ctx.push(FrameFragment::new(frame).with_spaced(true));

        Ok(())
    }
}
