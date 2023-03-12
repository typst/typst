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
use typst::eval::{Module, Scope};
use typst::font::{Font, FontWeight};
use typst::model::{Guard, SequenceNode, StyledNode};
use unicode_math_class::MathClass;

use self::ctx::*;
use self::fragment::*;
use self::row::*;
use self::spacing::*;
use crate::layout::{HNode, ParNode, Spacing};
use crate::prelude::*;
use crate::text::{
    families, variant, FontFamily, FontList, LinebreakNode, SpaceNode, TextNode, TextSize,
};

/// Create a module with all math definitions.
pub fn module() -> Module {
    let mut math = Scope::deduplicating();
    math.define("formula", FormulaNode::id());
    math.define("text", TextNode::id());

    // Grouping.
    math.define("lr", LrNode::id());
    math.define("abs", abs);
    math.define("norm", norm);
    math.define("floor", floor);
    math.define("ceil", ceil);

    // Attachments and accents.
    math.define("attach", AttachNode::id());
    math.define("scripts", ScriptsNode::id());
    math.define("limits", LimitsNode::id());
    math.define("accent", AccentNode::id());
    math.define("underline", UnderlineNode::id());
    math.define("overline", OverlineNode::id());
    math.define("underbrace", UnderbraceNode::id());
    math.define("overbrace", OverbraceNode::id());
    math.define("underbracket", UnderbracketNode::id());
    math.define("overbracket", OverbracketNode::id());

    // Fractions and matrix-likes.
    math.define("frac", FracNode::id());
    math.define("binom", BinomNode::id());
    math.define("vec", VecNode::id());
    math.define("mat", MatNode::id());
    math.define("cases", CasesNode::id());

    // Roots.
    math.define("sqrt", SqrtNode::id());
    math.define("root", RootNode::id());

    // Styles.
    math.define("upright", UprightNode::id());
    math.define("bold", BoldNode::id());
    math.define("italic", ItalicNode::id());
    math.define("serif", SerifNode::id());
    math.define("sans", SansNode::id());
    math.define("cal", CalNode::id());
    math.define("frak", FrakNode::id());
    math.define("mono", MonoNode::id());
    math.define("bb", BbNode::id());

    // Text operators.
    math.define("op", OpNode::id());
    op::define(&mut math);

    // Spacings.
    spacing::define(&mut math);

    // Symbols.
    for (name, symbol) in crate::symbols::SYM {
        math.define(*name, symbol.clone());
    }

    Module::new("math").with_scope(math)
}

/// A mathematical formula.
///
/// Can be displayed inline with text or as a separate block.
///
/// ## Example
/// ```example
/// #set text(font: "New Computer Modern")
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
/// Display: Formula
/// Category: math
#[node(Show, Finalize, Layout, LayoutMath)]
pub struct FormulaNode {
    /// Whether the formula is displayed as a separate block.
    #[default(false)]
    pub block: bool,

    /// The content of the formula.
    #[required]
    pub body: Content,
}

impl Show for FormulaNode {
    fn show(&self, _: &mut Vt, styles: StyleChain) -> SourceResult<Content> {
        let mut realized = self.clone().pack().guarded(Guard::Base(NodeId::of::<Self>()));
        if self.block(styles) {
            realized = realized.aligned(Axes::with_x(Some(Align::Center.into())))
        }
        Ok(realized)
    }
}

impl Finalize for FormulaNode {
    fn finalize(&self, realized: Content, _: StyleChain) -> Content {
        realized
            .styled(TextNode::set_weight(FontWeight::from_number(450)))
            .styled(TextNode::set_font(FontList(vec![FontFamily::new(
                "New Computer Modern Math",
            )])))
    }
}

impl Layout for FormulaNode {
    fn layout(
        &self,
        vt: &mut Vt,
        styles: StyleChain,
        regions: Regions,
    ) -> SourceResult<Fragment> {
        let block = self.block(styles);

        // Find a math font.
        let variant = variant(styles);
        let world = vt.world();
        let Some(font) = families(styles)
            .find_map(|family| {
                let id = world.book().select(family.as_str(), variant)?;
                let font = world.font(id)?;
                let _ = font.ttf().tables().math?.constants?;
                Some(font)
            })
        else {
            bail!(self.span(), "current font does not support math");
        };

        let mut ctx = MathContext::new(vt, styles, regions, &font, block);
        let mut frame = ctx.layout_frame(self)?;

        if !block {
            let slack = ParNode::leading_in(styles) * 0.7;
            let top_edge = TextNode::top_edge_in(styles).resolve(styles, font.metrics());
            let bottom_edge =
                -TextNode::bottom_edge_in(styles).resolve(styles, font.metrics());

            let ascent = top_edge.max(frame.ascent() - slack);
            let descent = bottom_edge.max(frame.descent() - slack);
            frame.translate(Point::with_y(ascent - frame.baseline()));
            frame.size_mut().y = ascent + descent;
        }

        Ok(Fragment::frame(frame))
    }
}

pub trait LayoutMath {
    fn layout_math(&self, ctx: &mut MathContext) -> SourceResult<()>;
}

impl LayoutMath for FormulaNode {
    fn layout_math(&self, ctx: &mut MathContext) -> SourceResult<()> {
        self.body().layout_math(ctx)
    }
}

impl LayoutMath for Content {
    fn layout_math(&self, ctx: &mut MathContext) -> SourceResult<()> {
        if let Some(node) = self.to::<SequenceNode>() {
            for child in node.children() {
                child.layout_math(ctx)?;
            }
            return Ok(());
        }

        if let Some(styled) = self.to::<StyledNode>() {
            let map = styled.styles();
            if TextNode::font_in(ctx.styles().chain(&map))
                != TextNode::font_in(ctx.styles())
            {
                let frame = ctx.layout_content(self)?;
                ctx.push(FrameFragment::new(ctx, frame).with_spaced(true));
                return Ok(());
            }

            let prev_map = std::mem::replace(&mut ctx.map, map);
            let prev_size = ctx.size;
            ctx.map.apply(prev_map.clone());
            ctx.size = TextNode::size_in(ctx.styles());
            styled.body().layout_math(ctx)?;
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
            if let Spacing::Rel(rel) = node.amount() {
                if rel.rel.is_zero() {
                    ctx.push(MathFragment::Spacing(rel.abs.resolve(ctx.styles())));
                }
            }
            return Ok(());
        }

        if let Some(node) = self.to::<TextNode>() {
            ctx.layout_text(node)?;
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
