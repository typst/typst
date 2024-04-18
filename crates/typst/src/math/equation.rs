use std::num::NonZeroUsize;

use unicode_math_class::MathClass;

use crate::diag::{bail, SourceResult};
use crate::engine::Engine;
use crate::foundations::{
    elem, Content, NativeElement, Packed, Resolve, ShowSet, Smart, StyleChain, Styles,
    Synthesize,
};
use crate::introspection::{Count, Counter, CounterUpdate, Locatable};
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
    decorate_frame
    families, variant, Font, FontFamily, FontList, FontWeight, LocalName, TextElem,
};
use crate::utils::{NonZeroExt, Numeric};
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
    /// By default, the alignment is `{end + horizon}`. For the horizontal
    /// component, you can use `{right}`, `{left}`, or `{start}` and `{end}`
    /// of the text direction; for the vertical component, you can use
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

/// Computes the origin position and the size of the bounding box that covers
/// a list of boxes from left to right
fn compute_bounding_box(pos_and_sizes: &[(Point, Size)]) -> (Point, Size) {
    let mut start_pos_x = Abs::inf();
    let mut start_pos_y = Abs::inf();
    let mut size_x = Abs::zero();
    let mut size_y = Abs::zero();

    for (p, s) in pos_and_sizes {
        let s = s.to_point();

        start_pos_x.set_min(p.x);
        start_pos_y.set_min(p.y);

        size_x.set_max(p.x + s.x);
        size_y.set_max(s.y)
    }

    (Point::new(start_pos_x, start_pos_y), Size::new(size_x, size_y))
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
        let mut pos_and_sizes: Vec<(Point, Size)> = Vec::new();
        let mut x = Abs::zero();

        // An empty equation should have a height, so we still create a frame
        // (which is then resized in the loop).
        if items.is_empty() {
            items.push(MathParItem::Frame(Frame::soft(Size::zero())));
        }

        // helper function to determine the vertical offset for a frame from MathParItem
        let get_vertical_shift_fn = |frame: &Frame| {
            let font_size = scaled_font_size(&ctx, styles);
            let slack = ParElem::leading_in(styles) * 0.7;
            let top_edge = TextElem::top_edge_in(styles).resolve(font_size, &font, None);
            let ascent = top_edge.max(frame.ascent() - slack);
            ascent - frame.baseline()
        };

        for (idx, item) in items.iter().enumerate() {
            match item {
                MathParItem::Frame(frame) => {
                    // determine the coordinates of the frame in the MathParItem Array
                    let y = get_vertical_shift_fn(frame);
                    let pos = Point::new(x, y);
                    let size = Size::new(frame.width().abs(), frame.height().abs());
                    pos_and_sizes.push((pos, size));
                    x += frame.width();
                }
                MathParItem::Space(space_width) => {
                    // A MarhParItem can also be space (consumes a width), in the latter
                    // case, we need to update the running x-coordinate
                    // also note that we compute starting from the first non-space MathParItem
                    if idx != 0 {
                        x += *space_width;
                    }
                }
            }
        }
        // computing the origin position and the size of the bounding box of the entire MathParItem
        // Array.
        let (pos, size) = compute_bounding_box(&pos_and_sizes);
        let decos = TextElem::deco_in(styles);
        let pos_and_frames = decorate_frame(&decos, pos, size);
        let (background_pos_and_frames, foreground_pos_and_frames): (Vec<_>, Vec<_>) =
            pos_and_frames.into_iter().partition(|&(b, _, _)| b);

        let mut first_frame_index: Option<usize> = None;
        let mut last_frame_index: Option<usize> = None;
        for (index, item) in items.iter().enumerate() {
            let MathParItem::Frame(_) = item else { continue };
            if first_frame_index.is_none() {
                first_frame_index = Some(index);
            }
            last_frame_index = Some(index);
        }
        if let Some(index) = first_frame_index {
            if let Some(MathParItem::Frame(ref mut frame)) = items.get_mut(index) {
                let y_off_set = get_vertical_shift_fn(frame);
                for (_, pos, frame_item) in background_pos_and_frames {
                    let new_pos = Point::new(pos.x, pos.y - y_off_set);
                    frame.prepend(new_pos, frame_item);
                }
            }
        }
        if let Some(index) = last_frame_index {
            if let Some(MathParItem::Frame(ref mut frame)) = items.get_mut(index) {
                let y_off_set = get_vertical_shift_fn(frame);
                for (_, pos, frame_item) in foreground_pos_and_frames {
                    let new_pos = Point::new(pos.x, pos.y - y_off_set);
                    frame.push(new_pos, frame_item);
                }
            }
        }
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
            return Ok(equation_builder.build(styles));
        };

        let pod = Regions::one(regions.base(), Axes::splat(false));
        let number = Counter::of(EquationElem::elem())
            .display_at_loc(engine, self.location().unwrap(), styles, numbering)?
            .spanned(span)
            .layout(engine, styles, pod)?
            .into_frame();

        static NUMBER_GUTTER: Em = Em::new(0.5);
        let full_number_width = number.width() + NUMBER_GUTTER.resolve(styles);

        let number_align = match self.number_align(styles) {
            SpecificAlignment::H(h) => SpecificAlignment::Both(h, VAlignment::Horizon),
            SpecificAlignment::V(v) => SpecificAlignment::Both(OuterHAlignment::End, v),
            SpecificAlignment::Both(h, v) => SpecificAlignment::Both(h, v),
        };

        let frame = add_equation_number(
            equation_builder,
            number,
            number_align.resolve(styles),
            AlignElem::alignment_in(styles).resolve(styles).x,
            regions.size.x,
            full_number_width,
            styles,
        );

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

fn add_equation_number(
    equation_builder: MathRunFrameBuilder,
    number: Frame,
    number_align: Axes<FixedAlignment>,
    equation_align: FixedAlignment,
    region_size_x: Abs,
    full_number_width: Abs,
    styles: StyleChain,
) -> Frame {
    let first =
        equation_builder.frames.first().map_or(
            (equation_builder.size, Point::zero(), Abs::zero()),
            |(frame, pos)| (frame.size(), *pos, frame.baseline()),
        );
    let last =
        equation_builder.frames.last().map_or(
            (equation_builder.size, Point::zero(), Abs::zero()),
            |(frame, pos)| (frame.size(), *pos, frame.baseline()),
        );
    let line_count = equation_builder.frames.len();
    let mut equation = equation_builder.build(styles);

    let width = if region_size_x.is_finite() {
        region_size_x
    } else {
        equation.width() + 2.0 * full_number_width
    };

    let is_multiline = line_count >= 2;
    let resizing_offset = resize_equation(
        &mut equation,
        &number,
        number_align,
        equation_align,
        width,
        is_multiline,
        [first, last],
    );
    equation.translate(Point::with_x(match (equation_align, number_align.x) {
        (FixedAlignment::Start, FixedAlignment::Start) => full_number_width,
        (FixedAlignment::End, FixedAlignment::End) => -full_number_width,
        _ => Abs::zero(),
    }));

    let x = match number_align.x {
        FixedAlignment::Start => Abs::zero(),
        FixedAlignment::End => equation.width() - number.width(),
        _ => unreachable!(),
    };
    let y = {
        let align_baselines = |(_, pos, baseline): (_, Point, Abs), number: &Frame| {
            resizing_offset.y + pos.y + baseline - number.baseline()
        };
        match number_align.y {
            FixedAlignment::Start => align_baselines(first, &number),
            FixedAlignment::Center if !is_multiline => align_baselines(first, &number),
            // In this case, the center lines (not baselines) of the number frame
            // and the equation frame shall be aligned.
            FixedAlignment::Center => (equation.height() - number.height()) / 2.0,
            FixedAlignment::End => align_baselines(last, &number),
        }
    };

    equation.push_frame(Point::new(x, y), number);
    equation
}

/// Resize the equation's frame accordingly so that it emcompasses the number.
fn resize_equation(
    equation: &mut Frame,
    number: &Frame,
    number_align: Axes<FixedAlignment>,
    equation_align: FixedAlignment,
    width: Abs,
    is_multiline: bool,
    [first, last]: [(Axes<Abs>, Point, Abs); 2],
) -> Point {
    if matches!(number_align.y, FixedAlignment::Center if is_multiline) {
        // In this case, the center lines (not baselines) of the number frame
        // and the equation frame shall be aligned.
        return equation.resize(
            Size::new(width, equation.height().max(number.height())),
            Axes::<FixedAlignment>::new(equation_align, FixedAlignment::Center),
        );
    }

    let excess_above = Abs::zero().max({
        if !is_multiline || matches!(number_align.y, FixedAlignment::Start) {
            let (.., baseline) = first;
            number.baseline() - baseline
        } else {
            Abs::zero()
        }
    });
    let excess_below = Abs::zero().max({
        if !is_multiline || matches!(number_align.y, FixedAlignment::End) {
            let (size, .., baseline) = last;
            (number.height() - number.baseline()) - (size.y - baseline)
        } else {
            Abs::zero()
        }
    });

    // The vertical expansion is asymmetric on the top and bottom edges, so we
    // first align at the top then translate the content downward later.
    let resizing_offset = equation.resize(
        Size::new(width, equation.height() + excess_above + excess_below),
        Axes::<FixedAlignment>::new(equation_align, FixedAlignment::Start),
    );
    equation.translate(Point::with_y(excess_above));
    resizing_offset + Point::with_y(excess_above)
}
