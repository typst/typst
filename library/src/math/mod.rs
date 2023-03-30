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
use typst::model::Guard;
use unicode_math_class::MathClass;

use self::ctx::*;
use self::fragment::*;
use self::row::*;
use self::spacing::*;
use crate::layout::{HElem, ParElem, Spacing};
use crate::meta::{Count, Counter, CounterUpdate, LocalName, Numbering};
use crate::prelude::*;
use crate::text::{
    families, variant, FontFamily, FontList, LinebreakElem, SpaceElem, TextElem, TextSize,
};

/// Create a module with all math definitions.
pub fn module() -> Module {
    let mut math = Scope::deduplicating();
    math.define("equation", EquationElem::func());
    math.define("text", TextElem::func());

    // Grouping.
    math.define("lr", LrElem::func());
    math.define("abs", abs);
    math.define("norm", norm);
    math.define("floor", floor);
    math.define("ceil", ceil);
    math.define("round", round);

    // Attachments and accents.
    math.define("attach", AttachElem::func());
    math.define("scripts", ScriptsElem::func());
    math.define("limits", LimitsElem::func());
    math.define("accent", AccentElem::func());
    math.define("underline", UnderlineElem::func());
    math.define("overline", OverlineElem::func());
    math.define("underbrace", UnderbraceElem::func());
    math.define("overbrace", OverbraceElem::func());
    math.define("underbracket", UnderbracketElem::func());
    math.define("overbracket", OverbracketElem::func());

    // Fractions and matrix-likes.
    math.define("frac", FracElem::func());
    math.define("binom", BinomElem::func());
    math.define("vec", VecElem::func());
    math.define("mat", MatElem::func());
    math.define("cases", CasesElem::func());

    // Roots.
    math.define("sqrt", sqrt);
    math.define("root", RootElem::func());

    // Styles.
    math.define("upright", upright);
    math.define("bold", bold);
    math.define("italic", italic);
    math.define("serif", serif);
    math.define("sans", sans);
    math.define("cal", cal);
    math.define("frak", frak);
    math.define("mono", mono);
    math.define("bb", bb);

    // Text operators.
    math.define("op", OpElem::func());
    op::define(&mut math);

    // Spacings.
    spacing::define(&mut math);

    // Symbols.
    for (name, symbol) in crate::symbols::SYM {
        math.define(*name, symbol.clone());
    }

    Module::new("math").with_scope(math)
}

/// A mathematical equation.
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
/// dollar signs to create an equation. Starting and ending the equation with at
/// least one space lifts it into a separate block that is centered
/// horizontally. For more details about math syntax, see the
/// [main math page]($category/math).
///
/// Display: Equation
/// Category: math
#[element(Locatable, Synthesize, Show, Finalize, Layout, LayoutMath, Count, LocalName)]
pub struct EquationElem {
    /// Whether the equation is displayed as a separate block.
    #[default(false)]
    pub block: bool,

    /// How to [number]($func/numbering) block-level equations.
    ///
    /// ```example
    /// #set math.equation(numbering: "(1)")
    ///
    /// We define:
    /// $ phi.alt := (1 + sqrt(5)) / 2 $ <ratio>
    ///
    /// With @ratio, we get:
    /// $ F_n = floor(1 / sqrt(5) phi.alt^n) $
    /// ```
    pub numbering: Option<Numbering>,

    /// The contents of the equation.
    #[required]
    pub body: Content,
}

impl Synthesize for EquationElem {
    fn synthesize(&mut self, styles: StyleChain) {
        self.push_block(self.block(styles));
        self.push_numbering(self.numbering(styles));
    }
}

impl Show for EquationElem {
    fn show(&self, _: &mut Vt, styles: StyleChain) -> SourceResult<Content> {
        let mut realized = self.clone().pack().guarded(Guard::Base(Self::func()));
        if self.block(styles) {
            realized = realized.aligned(Axes::with_x(Some(Align::Center.into())))
        }
        Ok(realized)
    }
}

impl Finalize for EquationElem {
    fn finalize(&self, realized: Content, _: StyleChain) -> Content {
        realized
            .styled(TextElem::set_weight(FontWeight::from_number(450)))
            .styled(TextElem::set_font(FontList(vec![FontFamily::new(
                "New Computer Modern Math",
            )])))
    }
}

impl Layout for EquationElem {
    fn layout(
        &self,
        vt: &mut Vt,
        styles: StyleChain,
        regions: Regions,
    ) -> SourceResult<Fragment> {
        const NUMBER_GUTTER: Em = Em::new(0.5);

        let block = self.block(styles);

        // Find a math font.
        let variant = variant(styles);
        let world = vt.world;
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

        if block {
            if let Some(numbering) = self.numbering(styles) {
                let pod = Regions::one(regions.base(), Axes::splat(false));
                let counter = Counter::of(Self::func())
                    .display(Some(numbering), false)
                    .layout(vt, styles, pod)?
                    .into_frame();

                let width = if regions.size.x.is_finite() {
                    regions.size.x
                } else {
                    frame.width()
                        + 2.0 * (counter.width() + NUMBER_GUTTER.resolve(styles))
                };

                let height = frame.height().max(counter.height());
                frame.resize(Size::new(width, height), Align::CENTER_HORIZON);

                let x = if TextElem::dir_in(styles).is_positive() {
                    frame.width() - counter.width()
                } else {
                    Abs::zero()
                };
                let y = (frame.height() - counter.height()) / 2.0;

                frame.push_frame(Point::new(x, y), counter)
            }
        } else {
            let slack = ParElem::leading_in(styles) * 0.7;
            let top_edge = TextElem::top_edge_in(styles).resolve(styles, font.metrics());
            let bottom_edge =
                -TextElem::bottom_edge_in(styles).resolve(styles, font.metrics());

            let ascent = top_edge.max(frame.ascent() - slack);
            let descent = bottom_edge.max(frame.descent() - slack);
            frame.translate(Point::with_y(ascent - frame.baseline()));
            frame.size_mut().y = ascent + descent;
        }

        Ok(Fragment::frame(frame))
    }
}

impl Count for EquationElem {
    fn update(&self) -> Option<CounterUpdate> {
        (self.block(StyleChain::default())
            && self.numbering(StyleChain::default()).is_some())
        .then(|| CounterUpdate::Step(NonZeroUsize::ONE))
    }
}

impl LocalName for EquationElem {
    fn local_name(&self, lang: Lang) -> &'static str {
        match lang {
            Lang::GERMAN => "Gleichung",
            Lang::ITALIAN => "Equazione",
            Lang::ENGLISH | _ => "Equation",
        }
    }
}

pub trait LayoutMath {
    fn layout_math(&self, ctx: &mut MathContext) -> SourceResult<()>;
}

impl LayoutMath for EquationElem {
    fn layout_math(&self, ctx: &mut MathContext) -> SourceResult<()> {
        self.body().layout_math(ctx)
    }
}

impl LayoutMath for Content {
    fn layout_math(&self, ctx: &mut MathContext) -> SourceResult<()> {
        if let Some(children) = self.to_sequence() {
            for child in children {
                child.layout_math(ctx)?;
            }
            return Ok(());
        }

        if let Some((elem, styles)) = self.to_styled() {
            if TextElem::font_in(ctx.styles().chain(&styles))
                != TextElem::font_in(ctx.styles())
            {
                let frame = ctx.layout_content(self)?;
                ctx.push(FrameFragment::new(ctx, frame).with_spaced(true));
                return Ok(());
            }

            let prev_map = std::mem::replace(&mut ctx.local, styles.clone());
            let prev_size = ctx.size;
            ctx.local.apply(prev_map.clone());
            ctx.size = TextElem::size_in(ctx.styles());
            elem.layout_math(ctx)?;
            ctx.size = prev_size;
            ctx.local = prev_map;
            return Ok(());
        }

        if self.is::<SpaceElem>() {
            ctx.push(MathFragment::Space(ctx.space_width.scaled(ctx)));
            return Ok(());
        }

        if self.is::<LinebreakElem>() {
            ctx.push(MathFragment::Linebreak);
            return Ok(());
        }

        if let Some(elem) = self.to::<HElem>() {
            if let Spacing::Rel(rel) = elem.amount() {
                if rel.rel.is_zero() {
                    ctx.push(MathFragment::Spacing(rel.abs.resolve(ctx.styles())));
                }
            }
            return Ok(());
        }

        if let Some(elem) = self.to::<TextElem>() {
            let fragment = ctx.layout_text(elem)?;
            ctx.push(fragment);
            return Ok(());
        }

        if let Some(elem) = self.with::<dyn LayoutMath>() {
            return elem.layout_math(ctx);
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
