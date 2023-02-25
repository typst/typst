//! Mathematical formulas.

#[macro_use]
mod ctx;
mod accent;
mod align;
mod attach;
mod delimited;
mod frac;
mod fragment;
mod matrix;
mod op;
mod root;
mod row;
mod spacing;
mod stretch;
mod style;
mod underover;

pub use self::accent::*;
pub use self::align::*;
pub use self::attach::*;
pub use self::delimited::*;
pub use self::frac::*;
pub use self::matrix::*;
pub use self::op::*;
pub use self::root::*;
pub use self::style::*;
pub use self::underover::*;

use ttf_parser::{GlyphId, Rect};
use typst::font::Font;
use typst::font::FontWeight;
use typst::model::{Guard, Module, Scope, SequenceNode, StyledNode};
use unicode_math_class::MathClass;

use self::ctx::*;
use self::fragment::*;
use self::row::*;
use self::spacing::*;
use crate::layout::{HNode, ParNode, Spacing};
use crate::prelude::*;
use crate::text::{
    families, variant, FallbackList, FontFamily, LinebreakNode, SpaceNode, TextNode,
    TextSize,
};

/// Create a module with all math definitions.
pub fn module() -> Module {
    let mut math = Scope::deduplicating();
    math.def_func::<FormulaNode>("formula");
    math.def_func::<TextNode>("text");

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
    math.def_func::<MatNode>("mat");
    math.def_func::<CasesNode>("cases");

    // Roots.
    math.def_func::<SqrtNode>("sqrt");
    math.def_func::<RootNode>("root");

    // Styles.
    math.def_func::<UprightNode>("upright");
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

    // Spacings.
    spacing::define(&mut math);

    // Symbols.
    for (name, symbol) in crate::symbols::SYM {
        math.define(*name, symbol.clone());
    }

    Module::new("math").with_scope(math)
}

/// # Formula
/// A mathematical formula.
///
/// Can be displayed inline with text or as a separate block.
///
/// ## Example
/// ```example
/// #set text("New Computer Modern")
///
/// Let $a$, $b$, and $c$ be the side
/// lengths of right-angled triangle.
/// Then, we know that:
/// $ a^2 + b^2 = c^2 $
///
/// Prove by induction:
/// $ sum_(k=1)^n k = (n(n+1)) / 2 $
/// ```
///
/// ## Syntax
/// This function also has dedicated syntax: Write mathematical markup within
/// dollar signs to create a formula. Starting and ending the formula with at
/// least one space lifts it into a separate block that is centered
/// horizontally. For more details about math syntax, see the
/// [main math page]($category/math).
///
/// ## Parameters
/// - body: `Content` (positional, required)
///   The contents of the formula.
///
/// - block: `bool` (named)
///   Whether the formula is displayed as a separate block.
///
/// ## Category
/// math
#[func]
#[capable(Show, Finalize, Layout, LayoutMath)]
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
        realized
            .styled(TextNode::WEIGHT, FontWeight::from_number(450))
            .styled(
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
        let mut frame = ctx.layout_frame(self)?;

        if !self.block {
            let slack = styles.get(ParNode::LEADING) * 0.7;
            let top_edge = styles.get(TextNode::TOP_EDGE).resolve(styles, font.metrics());
            let bottom_edge =
                -styles.get(TextNode::BOTTOM_EDGE).resolve(styles, font.metrics());

            let ascent = top_edge.max(frame.ascent() - slack);
            let descent = bottom_edge.max(frame.descent() - slack);
            frame.translate(Point::with_y(ascent - frame.baseline()));
            frame.size_mut().y = ascent + descent;
        }

        Ok(Fragment::frame(frame))
    }
}

#[capability]
pub trait LayoutMath {
    fn layout_math(&self, ctx: &mut MathContext) -> SourceResult<()>;
}

impl LayoutMath for FormulaNode {
    fn layout_math(&self, ctx: &mut MathContext) -> SourceResult<()> {
        self.body.layout_math(ctx)
    }
}

impl LayoutMath for Content {
    fn layout_math(&self, ctx: &mut MathContext) -> SourceResult<()> {
        if let Some(node) = self.to::<SequenceNode>() {
            for child in &node.0 {
                child.layout_math(ctx)?;
            }
            return Ok(());
        }

        if let Some(styled) = self.to::<StyledNode>() {
            if styled.map.contains(TextNode::FAMILY) {
                let frame = ctx.layout_content(self)?;
                ctx.push(FrameFragment::new(ctx, frame).with_spaced(true));
                return Ok(());
            }

            let prev_map = std::mem::replace(&mut ctx.map, styled.map.clone());
            let prev_size = ctx.size;
            ctx.map.apply(prev_map.clone());
            ctx.size = ctx.styles().get(TextNode::SIZE);
            styled.sub.layout_math(ctx)?;
            ctx.size = prev_size;
            ctx.map = prev_map;
            return Ok(());
        }

        if self.is::<SpaceNode>() {
            ctx.push(MathFragment::Space(ctx.space_width.scaled(ctx)));
            return Ok(());
        }

        if self.is::<LinebreakNode>() {
            ctx.push(MathFragment::Linebreak);
            return Ok(());
        }

        if let Some(node) = self.to::<HNode>() {
            if let Spacing::Rel(rel) = node.amount {
                if rel.rel.is_zero() {
                    ctx.push(MathFragment::Spacing(rel.abs.resolve(ctx.styles())));
                }
            }
            return Ok(());
        }

        if let Some(node) = self.to::<TextNode>() {
            ctx.layout_text(&node.0)?;
            return Ok(());
        }

        if let Some(node) = self.with::<dyn LayoutMath>() {
            return node.layout_math(ctx);
        }

        let mut frame = ctx.layout_content(self)?;
        if !frame.has_baseline() {
            let axis = scaled!(ctx, axis_height);
            frame.set_baseline(frame.height() / 2.0 + axis);
        }
        ctx.push(FrameFragment::new(ctx, frame).with_spaced(true));

        Ok(())
    }
}
