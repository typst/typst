use std::num::NonZeroUsize;

use comemo::Track;
use unicode_math_class::MathClass;

use crate::diag::{bail, SourceResult};
use crate::engine::Engine;
use crate::foundations::{
    elem, Cast, Content, Context, NativeElement, Packed, Resolve, ShowSet, Smart,
    StyleChain, Styles, Synthesize,
};
use crate::introspection::{Count, Counter, CounterState, CounterUpdate, Locatable};
use crate::layout::{
    Abs, AlignElem, Alignment, Axes, Em, FixedAlignment, Frame, LayoutMultiple,
    LayoutSingle, OuterHAlignment, Point, Regions, Size, SpecificAlignment, VAlignment,
};
use crate::math::{
    scaled_font_size, LayoutMath, MathContext, MathRunFrameBuilder, MathSize, MathVariant,
};
use crate::model::{Numbering, Outlinable, ParElem, Refable, Supplement};
use crate::syntax::Span;
use crate::text::{
    families, variant, Font, FontFamily, FontList, FontWeight, LocalName, TextElem,
};
use crate::util::{NonZeroExt, Numeric};
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
    Locatable,
    Synthesize,
    ShowSet,
    LayoutSingle,
    LayoutMath,
    Count,
    LocalName,
    Refable,
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
    #[borrowed]
    pub numbering: Option<Numbering>,

    /// The alignment of the equation numbering.
    ///
    /// For the horizontal component, you can use `{right}`, `{left}`, or `{start}`
    /// and `{end}` of the text direction; for the vertical component, you can use
    /// `{top}`, `{horizon}`, or `{bottom}`.
    ///
    /// ```example
    /// #set math.equation(numbering: "(1)", number-align: bottom)
    ///
    /// We can calculate:
    /// $ E &= sqrt(m_0^2 + p^2) \
    ///     &approx 125 "GeV" $
    /// ```
    #[default(SpecificAlignment::Both(OuterHAlignment::End, VAlignment::Horizon))]
    pub number_align: SpecificAlignment<OuterHAlignment, VAlignment>,

    /// The numbering mode of the equation.
    ///
    /// To number each equation block, use `{"by-block"}`; to number each line in the
    /// equation block, use `{"by-line"}`.
    ///
    /// ```example
    /// #set math.equation(numbering: "(1.1)", number-mode: "by-line")
    ///
    /// $ E &= h omega \
    ///     &approx 1 "eV" $
    /// and
    /// $ v = c $
    /// ```
    #[default(NumberMode::ByBlock)]
    pub number_mode: NumberMode,

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

    /// The size of the glyphs.
    #[internal]
    #[default(MathSize::Text)]
    #[ghost]
    pub size: MathSize,

    /// The style variant to select.
    #[internal]
    #[ghost]
    pub variant: MathVariant,

    /// Affects the height of exponents.
    #[internal]
    #[default(false)]
    #[ghost]
    pub cramped: bool,

    /// Whether to use bold glyphs.
    #[internal]
    #[default(false)]
    #[ghost]
    pub bold: bool,

    /// Whether to use italic glyphs.
    #[internal]
    #[ghost]
    pub italic: Smart<bool>,

    /// A forced class to use for all fragment.
    #[internal]
    #[ghost]
    pub class: Option<MathClass>,
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
                supplement.resolve(engine, styles, [self.clone().pack()])?
            }
        };

        self.push_supplement(Smart::Custom(Some(Supplement::Content(supplement))));
        Ok(())
    }
}

impl ShowSet for Packed<EquationElem> {
    fn show_set(&self, styles: StyleChain) -> Styles {
        let mut out = Styles::new();
        if self.block(styles) {
            out.set(AlignElem::set_alignment(Alignment::CENTER));
            out.set(EquationElem::set_size(MathSize::Display));
        } else {
            out.set(EquationElem::set_size(MathSize::Text));
        }
        out.set(TextElem::set_weight(FontWeight::from_number(450)));
        out.set(TextElem::set_font(FontList(vec![FontFamily::new(
            "New Computer Modern Math",
        )])));
        out
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

        let font = find_math_font(engine, styles, self.span())?;

        let mut ctx = MathContext::new(engine, styles, regions, &font);
        let run = ctx.layout_into_run(self, styles)?;

        let mut items = if run.row_count() == 1 {
            run.into_par_items()
        } else {
            vec![MathParItem::Frame(run.into_fragment(&ctx, styles).into_frame())]
        };

        for item in &mut items {
            let MathParItem::Frame(frame) = item else { continue };

            let font_size = scaled_font_size(&ctx, styles);
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

impl LayoutSingle for Packed<EquationElem> {
    #[typst_macros::time(name = "math.equation", span = self.span())]
    fn layout(
        &self,
        engine: &mut Engine,
        styles: StyleChain,
        regions: Regions,
    ) -> SourceResult<Frame> {
        assert!(self.block(styles));

        let span = self.span();
        let font = find_math_font(engine, styles, span)?;

        let mut ctx = MathContext::new(engine, styles, regions, &font);
        let equation_builder = ctx
            .layout_into_run(self, styles)?
            .multiline_frame_builder(&ctx, styles);

        let Some(numbering) = (**self).numbering(styles) else {
            return Ok(equation_builder.build());
        };

        let number_align = match self.number_align(styles) {
            SpecificAlignment::H(h) => SpecificAlignment::Both(h, VAlignment::Horizon),
            SpecificAlignment::V(v) => SpecificAlignment::Both(OuterHAlignment::End, v),
            SpecificAlignment::Both(h, v) => SpecificAlignment::Both(h, v),
        };

        let pod: Regions<'_> = Regions::one(regions.base(), Axes::splat(false));
        let loc = self.location().unwrap();
        let context = Context::new(Some(loc), Some(styles));
        let mut counter_state = Counter::of(EquationElem::elem()).at_loc(engine, loc)?;

        let mut display_counter_state =
            |counter_state: &CounterState| -> SourceResult<Frame> {
                Ok(counter_state
                    .display(engine, context.track(), numbering)?
                    .display()
                    .spanned(span)
                    .layout(engine, styles, pod)?
                    .into_frame())
            };
        let block_number = display_counter_state(&counter_state)?;

        let mut line_medatadata = vec![];
        let number_align = number_align.resolve(styles);
        let number_mode = self.number_mode(styles);
        let line_counter_level = counter_state.levels().saturating_add(1);
        let line_count = equation_builder.frames.len();
        for (i, line) in equation_builder.frames.iter().enumerate() {
            counter_state.step(line_counter_level, 1);
            let line_number = match number_mode {
                NumberMode::ByLine => {
                    if matches!(line_count, 1) {
                        Some(block_number.clone())
                    } else {
                        Some(display_counter_state(&counter_state)?)
                    }
                }
                NumberMode::ByBlock => match (i, line_count - i, number_align.y) {
                    (0, _, FixedAlignment::Start) | (_, 1, FixedAlignment::End) => {
                        Some(block_number.clone())
                    }
                    _ => None,
                },
            };
            line_medatadata.push(LineMetadata {
                number: line_number,
                size: line.0.size(),
                point: line.1,
            });
        }

        let frame = add_equation_number(
            equation_builder,
            AlignElem::alignment_in(styles).resolve(styles).x,
            block_number,
            line_medatadata,
            number_align,
            number_mode,
            regions.size.x,
            styles,
        )?;

        Ok(frame)
    }
}

impl Count for Packed<EquationElem> {
    fn update(&self) -> Option<CounterUpdate> {
        (self.block(StyleChain::default()) && self.numbering().is_some())
            .then(|| CounterUpdate::Step(NonZeroUsize::ONE))
    }
}

impl LocalName for Packed<EquationElem> {
    const KEY: &'static str = "equation";
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

    fn numbering(&self) -> Option<&Numbering> {
        (**self).numbering(StyleChain::default()).as_ref()
    }
}

impl Outlinable for Packed<EquationElem> {
    fn outline(
        &self,
        engine: &mut Engine,
        styles: StyleChain,
    ) -> SourceResult<Option<Content>> {
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

        let numbers = self.counter().display_at_loc(
            engine,
            self.location().unwrap(),
            styles,
            numbering,
        )?;

        Ok(Some(supplement + numbers))
    }
}

impl LayoutMath for Packed<EquationElem> {
    #[typst_macros::time(name = "math.equation", span = self.span())]
    fn layout_math(&self, ctx: &mut MathContext, styles: StyleChain) -> SourceResult<()> {
        self.body().layout_math(ctx, styles)
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

/// How equations are numbered.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Cast)]
enum NumberMode {
    ByBlock,
    ByLine,
}

struct LineMetadata {
    number: Option<Frame>,
    size: Axes<Abs>,
    point: Point,
}

#[allow(clippy::too_many_arguments)]
fn add_equation_number(
    equation_builder: MathRunFrameBuilder,
    equation_align: FixedAlignment,
    block_number: Frame,
    line_metadata: Vec<LineMetadata>,
    number_align: Axes<FixedAlignment>,
    number_mode: NumberMode,
    region_size_x: Abs,
    styles: StyleChain,
) -> SourceResult<Frame> {
    let mut equation = equation_builder.build();
    if line_metadata.is_empty() {
        return Ok(equation);
    }

    let number_width = match number_mode {
        NumberMode::ByBlock => block_number.width(),
        NumberMode::ByLine => line_metadata.iter().fold(Abs::zero(), |acc, e| {
            acc.max(e.number.as_ref().map_or(Abs::zero(), |num| num.width()))
        }),
    };
    static NUMBER_GUTTER: Em = Em::new(0.5);
    let full_number_width = NUMBER_GUTTER.resolve(styles) + number_width;

    let width = if region_size_x.is_finite() {
        region_size_x
    } else {
        equation.width() + 2.0 * full_number_width
    };
    let height = match number_align.y {
        FixedAlignment::Start => {
            let first = line_metadata.first();
            let excess_above = first.map_or(Abs::zero(), |first| {
                first.number.as_ref().map_or(Abs::zero(), |number| {
                    (number.height() - first.size.y) / 2.0 - first.point.y
                })
            });
            equation.height() + Abs::zero().max(excess_above)
        }
        FixedAlignment::Center => equation.height().max(block_number.height()),
        FixedAlignment::End => {
            let last = line_metadata.last();
            let excess_below = last.map_or(Abs::zero(), |last| {
                last.number.as_ref().map_or(Abs::zero(), |number| {
                    (number.height() + last.size.y) / 2.0 - equation.height()
                        + last.point.y
                })
            });
            equation.height() + Abs::zero().max(excess_below)
        }
    };
    let resizing_offset = equation.resize(
        Size::new(width, height),
        Axes::<FixedAlignment>::new(equation_align, number_align.y.inv()),
    );
    equation.translate(Point::with_x(match (equation_align, number_align.x) {
        (FixedAlignment::Start, FixedAlignment::Start) => full_number_width,
        (FixedAlignment::End, FixedAlignment::End) => -full_number_width,
        _ => Abs::zero(),
    }));

    let x = match number_align.x {
        FixedAlignment::Start => Abs::zero(),
        FixedAlignment::End => equation.width() - number_width,
        _ => unreachable!(),
    };
    let dh = |h1: Abs, h2: Abs| (h1 - h2) / 2.0;

    if matches!(
        (number_mode, number_align.y),
        (NumberMode::ByBlock, FixedAlignment::Center)
    ) {
        let y = dh(equation.height(), block_number.height());
        equation.push_frame(Point::new(x, y), block_number);
    } else {
        for line in line_metadata {
            match line.number {
                Some(number) => {
                    let y = resizing_offset.y
                        + line.point.y
                        + dh(line.size.y, number.height());
                    equation.push_frame(Point::new(x, y), number);
                }
                None => continue,
            };
        }
    }

    Ok(equation)
}
