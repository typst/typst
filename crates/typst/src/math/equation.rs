use std::num::NonZeroUsize;

use crate::diag::{bail, SourceResult};
use crate::engine::Engine;
use crate::foundations::{
    elem, Content, Finalize, Guard, NativeElement, Packed, Resolve, Show, Smart,
    StyleChain, Synthesize,
};
use crate::introspection::{Count, Counter, CounterUpdate, Locatable};
use crate::layout::{
    Abs, AlignElem, Alignment, Axes, Dir, Em, FixedAlignment, Fragment, Frame, Layout,
    Point, Regions, Size,
};
use crate::math::{LayoutMath, MathContext};
use crate::model::{Numbering, Outlinable, ParElem, Refable, Supplement};
use crate::syntax::Span;
use crate::text::{
    families, variant, Font, FontFamily, FontList, FontWeight, Lang, LocalName, Region,
    TextElem,
};
use crate::util::{option_eq, NonZeroExt, Numeric};
use crate::World;

/// A mathematical equation.
///
/// Can be displayed inline with text or as a separate block.
///
/// # Example
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
/// # Syntax
/// This function also has dedicated syntax: Write mathematical markup within
/// dollar signs to create an equation. Starting and ending the equation with at
/// least one space lifts it into a separate block that is centered
/// horizontally. For more details about math syntax, see the
/// [main math page]($category/math).
#[elem(
    Locatable, Synthesize, Show, Finalize, Layout, LayoutMath, Count, LocalName, Refable,
    Outlinable
)]
pub struct EquationElem {
    /// Whether the equation is displayed as a separate block.
    #[default(false)]
    pub block: bool,

    /// How to [number]($numbering) block-level equations.
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

    /// A supplement for the equation.
    ///
    /// For references to equations, this is added before the referenced number.
    ///
    /// If a function is specified, it is passed the referenced equation and
    /// should return content.
    ///
    /// ```example
    /// #set math.equation(numbering: "(1)", supplement: [Eq.])
    ///
    /// We define:
    /// $ phi.alt := (1 + sqrt(5)) / 2 $ <ratio>
    ///
    /// With @ratio, we get:
    /// $ F_n = floor(1 / sqrt(5) phi.alt^n) $
    /// ```
    pub supplement: Smart<Option<Supplement>>,

    /// The contents of the equation.
    #[required]
    pub body: Content,
}

impl Synthesize for Packed<EquationElem> {
    fn synthesize(
        &mut self,
        engine: &mut Engine,
        styles: StyleChain,
    ) -> SourceResult<()> {
        let supplement = match self.as_ref().supplement(styles) {
            Smart::Auto => TextElem::packed(Self::local_name_in(styles)),
            Smart::Custom(None) => Content::empty(),
            Smart::Custom(Some(supplement)) => {
                supplement.resolve(engine, [self.clone().pack()])?
            }
        };

        let elem = self.as_mut();
        elem.push_block(elem.block(styles));
        elem.push_numbering(elem.numbering(styles));
        elem.push_supplement(Smart::Custom(Some(Supplement::Content(supplement))));

        Ok(())
    }
}

impl Show for Packed<EquationElem> {
    #[typst_macros::time(name = "math.equation", span = self.span())]
    fn show(&self, _: &mut Engine, styles: StyleChain) -> SourceResult<Content> {
        let mut realized = self.clone().pack().guarded(Guard::Base(EquationElem::elem()));
        if self.block(styles) {
            realized = AlignElem::new(realized).pack().spanned(self.span());
        }
        Ok(realized)
    }
}

impl Finalize for Packed<EquationElem> {
    fn finalize(&self, realized: Content, style: StyleChain) -> Content {
        let mut realized = realized;
        if self.block(style) {
            realized = realized.styled(AlignElem::set_alignment(Alignment::CENTER));
        }
        realized
            .styled(TextElem::set_weight(FontWeight::from_number(450)))
            .styled(TextElem::set_font(FontList(vec![FontFamily::new(
                "New Computer Modern Math",
            )])))
    }
}

/// Layouted items suitable for placing in a paragraph.
#[derive(Debug, Clone)]
pub enum MathParItem {
    Space(Abs),
    Frame(Frame),
}

impl MathParItem {
    /// The text representation of this item.
    pub fn text(&self) -> char {
        match self {
            MathParItem::Space(_) => ' ',        // Space
            MathParItem::Frame(_) => '\u{FFFC}', // Object Replacement Character
        }
    }
}

impl Packed<EquationElem> {
    pub fn layout_inline(
        &self,
        engine: &mut Engine<'_>,
        styles: StyleChain,
        regions: Regions,
    ) -> SourceResult<Vec<MathParItem>> {
        assert!(!self.block(styles));

        // Find a math font.
        let font = find_math_font(engine, styles, self.span())?;

        let mut ctx = MathContext::new(engine, styles, regions, &font, false);
        let rows = ctx.layout_root(self)?;

        let mut items = if rows.row_count() == 1 {
            rows.into_par_items()
        } else {
            vec![MathParItem::Frame(rows.into_fragment(&ctx).into_frame())]
        };

        for item in &mut items {
            let MathParItem::Frame(frame) = item else { continue };

            let font_size = TextElem::size_in(styles);
            let slack = ParElem::leading_in(styles) * 0.7;
            let top_edge = TextElem::top_edge_in(styles).resolve(font_size, &font, None);
            let bottom_edge =
                -TextElem::bottom_edge_in(styles).resolve(font_size, &font, None);

            let ascent = top_edge.max(frame.ascent() - slack);
            let descent = bottom_edge.max(frame.descent() - slack);
            frame.translate(Point::with_y(ascent - frame.baseline()));
            frame.size_mut().y = ascent + descent;
        }

        Ok(items)
    }
}

impl Layout for Packed<EquationElem> {
    #[typst_macros::time(name = "math.equation", span = self.span())]
    fn layout(
        &self,
        engine: &mut Engine,
        styles: StyleChain,
        regions: Regions,
    ) -> SourceResult<Fragment> {
        const NUMBER_GUTTER: Em = Em::new(0.5);

        assert!(self.block(styles));

        // Find a math font.
        let font = find_math_font(engine, styles, self.span())?;

        let mut ctx = MathContext::new(engine, styles, regions, &font, true);
        let mut frame = ctx.layout_frame(self)?;

        if let Some(numbering) = (**self).numbering(styles) {
            let pod = Regions::one(regions.base(), Axes::splat(false));
            let counter = Counter::of(EquationElem::elem())
                .display(self.span(), Some(numbering), false)
                .layout(engine, styles, pod)?
                .into_frame();

            let full_counter_width = counter.width() + NUMBER_GUTTER.resolve(styles);
            let width = if regions.size.x.is_finite() {
                regions.size.x
            } else {
                frame.width() + 2.0 * full_counter_width
            };

            let height = frame.height().max(counter.height());
            let align = AlignElem::alignment_in(styles).resolve(styles).x;
            frame.resize(Size::new(width, height), Axes::splat(align));

            let dir = TextElem::dir_in(styles);
            let offset = match (align, dir) {
                (FixedAlignment::Start, Dir::RTL) => full_counter_width,
                (FixedAlignment::End, Dir::LTR) => -full_counter_width,
                _ => Abs::zero(),
            };
            frame.translate(Point::with_x(offset));

            let x = if dir.is_positive() {
                frame.width() - counter.width()
            } else {
                Abs::zero()
            };
            let y = (frame.height() - counter.height()) / 2.0;

            frame.push_frame(Point::new(x, y), counter)
        }

        Ok(Fragment::frame(frame))
    }
}

impl Count for Packed<EquationElem> {
    fn update(&self) -> Option<CounterUpdate> {
        (self.block(StyleChain::default()) && self.numbering().is_some())
            .then(|| CounterUpdate::Step(NonZeroUsize::ONE))
    }
}

impl LocalName for Packed<EquationElem> {
    fn local_name(lang: Lang, region: Option<Region>) -> &'static str {
        match lang {
            Lang::ALBANIAN => "Ekuacion",
            Lang::ARABIC => "معادلة",
            Lang::BOKMÅL => "Ligning",
            Lang::CATALAN => "Equació",
            Lang::CHINESE if option_eq(region, "TW") => "方程式",
            Lang::CHINESE => "公式",
            Lang::CZECH => "Rovnice",
            Lang::DANISH => "Ligning",
            Lang::DUTCH => "Vergelijking",
            Lang::ESTONIAN => "Valem",
            Lang::FILIPINO => "Ekwasyon",
            Lang::FINNISH => "Yhtälö",
            Lang::FRENCH => "Équation",
            Lang::GERMAN => "Gleichung",
            Lang::GREEK => "Εξίσωση",
            Lang::HUNGARIAN => "Egyenlet",
            Lang::ITALIAN => "Equazione",
            Lang::NYNORSK => "Likning",
            Lang::POLISH => "Równanie",
            Lang::PORTUGUESE => "Equação",
            Lang::ROMANIAN => "Ecuația",
            Lang::RUSSIAN => "Уравнение",
            Lang::SERBIAN => "Једначина",
            Lang::SLOVENIAN => "Enačba",
            Lang::SPANISH => "Ecuación",
            Lang::SWEDISH => "Ekvation",
            Lang::TURKISH => "Denklem",
            Lang::UKRAINIAN => "Рівняння",
            Lang::VIETNAMESE => "Phương trình",
            Lang::JAPANESE => "式",
            Lang::ENGLISH | _ => "Equation",
        }
    }
}

impl Refable for Packed<EquationElem> {
    fn supplement(&self) -> Content {
        // After synthesis, this should always be custom content.
        match (**self).supplement(StyleChain::default()) {
            Smart::Custom(Some(Supplement::Content(content))) => content,
            _ => Content::empty(),
        }
    }

    fn counter(&self) -> Counter {
        Counter::of(EquationElem::elem())
    }

    fn numbering(&self) -> Option<Numbering> {
        (**self).numbering(StyleChain::default())
    }
}

impl Outlinable for Packed<EquationElem> {
    fn outline(&self, engine: &mut Engine) -> SourceResult<Option<Content>> {
        if !self.block(StyleChain::default()) {
            return Ok(None);
        }
        let Some(numbering) = self.numbering() else {
            return Ok(None);
        };

        // After synthesis, this should always be custom content.
        let mut supplement = match (**self).supplement(StyleChain::default()) {
            Smart::Custom(Some(Supplement::Content(content))) => content,
            _ => Content::empty(),
        };

        if !supplement.is_empty() {
            supplement += TextElem::packed("\u{a0}");
        }

        let numbers = self
            .counter()
            .at(engine, self.location().unwrap())?
            .display(engine, &numbering)?;

        Ok(Some(supplement + numbers))
    }
}

impl LayoutMath for Packed<EquationElem> {
    #[typst_macros::time(name = "math.equation", span = self.span())]
    fn layout_math(&self, ctx: &mut MathContext) -> SourceResult<()> {
        self.body().layout_math(ctx)
    }
}

fn find_math_font(
    engine: &mut Engine<'_>,
    styles: StyleChain,
    span: Span,
) -> SourceResult<Font> {
    let variant = variant(styles);
    let world = engine.world;
    let Some(font) = families(styles).find_map(|family| {
        let id = world.book().select(family, variant)?;
        let font = world.font(id)?;
        let _ = font.ttf().tables().math?.constants?;
        Some(font)
    }) else {
        bail!(span, "current font does not support math");
    };
    Ok(font)
}
