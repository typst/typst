#[macro_use]
mod shared;
mod accent;
mod attach;
mod cancel;
mod frac;
mod fragment;
mod lr;
mod mat;
mod root;
mod run;
mod stretch;
mod text;
mod underover;

use comemo::Tracked;
use typst_library::World;
use typst_library::diag::{At, SourceResult, warning};
use typst_library::engine::Engine;
use typst_library::foundations::{
    Content, NativeElement, Packed, Resolve, Style, StyleChain, SymbolElem,
};
use typst_library::introspection::{Counter, Locator, SplitLocator, TagElem};
use typst_library::layout::{
    Abs, AlignElem, Axes, BlockElem, BoxElem, Em, FixedAlignment, Fragment, Frame, HElem,
    InlineItem, OuterHAlignment, PlaceElem, Point, Region, Regions, Size, Spacing,
    SpecificAlignment, VAlignment,
};
use typst_library::math::*;
use typst_library::model::ParElem;
use typst_library::routines::{Arenas, RealizationKind};
use typst_library::text::{
    Font, FontFlags, LinebreakElem, SpaceElem, TextEdgeBounds, TextElem, variant,
};
use typst_syntax::Span;
use typst_utils::{LazyHash, Numeric};

use unicode_math_class::MathClass;

use self::fragment::{
    FrameFragment, GlyphFragment, Limits, MathFragment, has_dtls_feat, stretch_axes,
};
use self::run::{LeftRightAlternator, MathRun, MathRunFrameBuilder};
use self::shared::*;
use self::stretch::stretch_fragment;

/// Layout an inline equation (in a paragraph).
#[typst_macros::time(span = elem.span())]
pub fn layout_equation_inline(
    elem: &Packed<EquationElem>,
    engine: &mut Engine,
    locator: Locator,
    styles: StyleChain,
    region: Size,
) -> SourceResult<Vec<InlineItem>> {
    assert!(!elem.block.get(styles));

    let span = elem.span();
    let font = get_font(engine.world, styles, span)?;
    warn_non_math_font(&font, engine, span);

    let mut locator = locator.split();
    let mut ctx = MathContext::new(engine, &mut locator, region, font.clone());

    let scale_style = style_for_script_scale(&font);
    let styles = styles.chain(&scale_style);

    let run = ctx.layout_into_run(&elem.body, styles)?;

    let mut items = if run.row_count() == 1 {
        run.into_par_items()
    } else {
        vec![InlineItem::Frame(run.into_fragment(styles).into_frame())]
    };

    // An empty equation should have a height, so we still create a frame
    // (which is then resized in the loop).
    if items.is_empty() {
        items.push(InlineItem::Frame(Frame::soft(Size::zero())));
    }

    for item in &mut items {
        let InlineItem::Frame(frame) = item else { continue };

        let slack = styles.resolve(ParElem::leading) * 0.7;

        let (t, b) = font.edges(
            styles.get(TextElem::top_edge),
            styles.get(TextElem::bottom_edge),
            styles.resolve(TextElem::size),
            TextEdgeBounds::Frame(frame),
        );

        let ascent = t.max(frame.ascent() - slack);
        let descent = b.max(frame.descent() - slack);
        frame.translate(Point::with_y(ascent - frame.baseline()));
        frame.size_mut().y = ascent + descent;
    }

    Ok(items)
}

/// Layout a block-level equation (in a flow).
#[typst_macros::time(span = elem.span())]
pub fn layout_equation_block(
    elem: &Packed<EquationElem>,
    engine: &mut Engine,
    locator: Locator,
    styles: StyleChain,
    regions: Regions,
) -> SourceResult<Fragment> {
    assert!(elem.block.get(styles));

    let span = elem.span();
    let font = get_font(engine.world, styles, span)?;
    warn_non_math_font(&font, engine, span);

    let mut locator = locator.split();
    let mut ctx = MathContext::new(engine, &mut locator, regions.base(), font.clone());

    let scale_style = style_for_script_scale(&font);
    let styles = styles.chain(&scale_style);

    let full_equation_builder = ctx
        .layout_into_run(&elem.body, styles)?
        .multiline_frame_builder(styles);
    let width = full_equation_builder.size.x;

    let equation_builders = if styles.get(BlockElem::breakable) {
        let mut rows = full_equation_builder.frames.into_iter().peekable();
        let mut equation_builders = vec![];
        let mut last_first_pos = Point::zero();
        let mut regions = regions;

        loop {
            // Keep track of the position of the first row in this region,
            // so that the offset can be reverted later.
            let Some(&(_, first_pos)) = rows.peek() else { break };
            last_first_pos = first_pos;

            let mut frames = vec![];
            let mut height = Abs::zero();
            while let Some((sub, pos)) = rows.peek() {
                let mut pos = *pos;
                pos.y -= first_pos.y;

                // Finish this region if the line doesn't fit. Only do it if
                // we placed at least one line _or_ we still have non-last
                // regions. Crucially, we don't want to infinitely create
                // new regions which are too small.
                if !regions.size.y.fits(sub.height() + pos.y)
                    && (regions.may_progress()
                        || (regions.may_break() && !frames.is_empty()))
                {
                    break;
                }

                let (sub, _) = rows.next().unwrap();
                height = height.max(pos.y + sub.height());
                frames.push((sub, pos));
            }

            equation_builders
                .push(MathRunFrameBuilder { frames, size: Size::new(width, height) });
            regions.next();
        }

        // Append remaining rows to the equation builder of the last region.
        if let Some(equation_builder) = equation_builders.last_mut() {
            equation_builder.frames.extend(rows.map(|(frame, mut pos)| {
                pos.y -= last_first_pos.y;
                (frame, pos)
            }));

            let height = equation_builder
                .frames
                .iter()
                .map(|(frame, pos)| frame.height() + pos.y)
                .max()
                .unwrap_or(equation_builder.size.y);

            equation_builder.size.y = height;
        }

        // Ensure that there is at least one frame, even for empty equations.
        if equation_builders.is_empty() {
            equation_builders
                .push(MathRunFrameBuilder { frames: vec![], size: Size::zero() });
        }

        equation_builders
    } else {
        vec![full_equation_builder]
    };

    let Some(numbering) = elem.numbering.get_ref(styles) else {
        let frames = equation_builders
            .into_iter()
            .map(MathRunFrameBuilder::build)
            .collect();
        return Ok(Fragment::frames(frames));
    };

    let pod = Region::new(regions.base(), Axes::splat(false));
    let counter = Counter::of(EquationElem::ELEM)
        .display_at_loc(engine, elem.location().unwrap(), styles, numbering)?
        .spanned(span);
    let number = crate::layout_frame(engine, &counter, locator.next(&()), styles, pod)?;

    static NUMBER_GUTTER: Em = Em::new(0.5);
    let full_number_width = number.width() + NUMBER_GUTTER.resolve(styles);

    let number_align = match elem.number_align.get(styles) {
        SpecificAlignment::H(h) => SpecificAlignment::Both(h, VAlignment::Horizon),
        SpecificAlignment::V(v) => SpecificAlignment::Both(OuterHAlignment::End, v),
        SpecificAlignment::Both(h, v) => SpecificAlignment::Both(h, v),
    };

    // Add equation numbers to each equation region.
    let region_count = equation_builders.len();
    let frames = equation_builders
        .into_iter()
        .map(|builder| {
            if builder.frames.is_empty() && region_count > 1 {
                // Don't number empty regions, but do number empty equations.
                return builder.build();
            }
            add_equation_number(
                builder,
                number.clone(),
                number_align.resolve(styles),
                styles.get(AlignElem::alignment).resolve(styles).x,
                regions.size.x,
                full_number_width,
            )
        })
        .collect();

    Ok(Fragment::frames(frames))
}

fn add_equation_number(
    equation_builder: MathRunFrameBuilder,
    number: Frame,
    number_align: Axes<FixedAlignment>,
    equation_align: FixedAlignment,
    region_size_x: Abs,
    full_number_width: Abs,
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
    let mut equation = equation_builder.build();

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

/// Resize the equation's frame accordingly so that it encompasses the number.
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

/// The context for math layout.
struct MathContext<'a, 'v, 'e> {
    // External.
    engine: &'v mut Engine<'e>,
    locator: &'v mut SplitLocator<'a>,
    region: Region,
    // Mutable.
    fonts_stack: Vec<Font>,
    fragments: Vec<MathFragment>,
}

impl<'a, 'v, 'e> MathContext<'a, 'v, 'e> {
    /// Create a new math context.
    fn new(
        engine: &'v mut Engine<'e>,
        locator: &'v mut SplitLocator<'a>,
        base: Size,
        font: Font,
    ) -> Self {
        Self {
            engine,
            locator,
            region: Region::new(base, Axes::splat(false)),
            fonts_stack: vec![font],
            fragments: vec![],
        }
    }

    /// Get the current base font.
    #[inline]
    fn font(&self) -> &Font {
        // Will always be at least one font in the stack.
        self.fonts_stack.last().unwrap()
    }

    /// Push a fragment.
    fn push(&mut self, fragment: impl Into<MathFragment>) {
        self.fragments.push(fragment.into());
    }

    /// Push multiple fragments.
    fn extend(&mut self, fragments: impl IntoIterator<Item = MathFragment>) {
        self.fragments.extend(fragments);
    }

    /// Layout the given element and return the result as a [`MathRun`].
    fn layout_into_run(
        &mut self,
        elem: &Content,
        styles: StyleChain,
    ) -> SourceResult<MathRun> {
        Ok(MathRun::new(self.layout_into_fragments(elem, styles)?))
    }

    /// Layout the given element and return the resulting [`MathFragment`]s.
    fn layout_into_fragments(
        &mut self,
        elem: &Content,
        styles: StyleChain,
    ) -> SourceResult<Vec<MathFragment>> {
        // The element's layout_math() changes the fragments held in this
        // MathContext object, but for convenience this function shouldn't change
        // them, so we restore the MathContext's fragments after obtaining the
        // layout result.
        let prev = std::mem::take(&mut self.fragments);
        self.layout_into_self(elem, styles)?;
        Ok(std::mem::replace(&mut self.fragments, prev))
    }

    /// Layout the given element and return the result as a
    /// unified [`MathFragment`].
    fn layout_into_fragment(
        &mut self,
        elem: &Content,
        styles: StyleChain,
    ) -> SourceResult<MathFragment> {
        Ok(self.layout_into_run(elem, styles)?.into_fragment(styles))
    }

    /// Layout the given element and return the result as a [`Frame`].
    fn layout_into_frame(
        &mut self,
        elem: &Content,
        styles: StyleChain,
    ) -> SourceResult<Frame> {
        Ok(self.layout_into_fragment(elem, styles)?.into_frame())
    }

    /// Layout arbitrary content.
    fn layout_into_self(
        &mut self,
        content: &Content,
        styles: StyleChain,
    ) -> SourceResult<()> {
        let arenas = Arenas::default();
        let pairs = (self.engine.routines.realize)(
            RealizationKind::Math,
            self.engine,
            self.locator,
            &arenas,
            content,
            styles,
        )?;

        let outer_styles = styles;
        let outer_font = styles.get_ref(TextElem::font);
        for (elem, styles) in pairs {
            // Whilst this check isn't exact, it more or less suffices as a
            // change in font variant probably won't have an effect on metrics.
            if styles != outer_styles && styles.get_ref(TextElem::font) != outer_font {
                self.fonts_stack
                    .push(get_font(self.engine.world, styles, elem.span())?);
                let scale_style = style_for_script_scale(self.font());
                layout_realized(elem, self, styles.chain(&scale_style))?;
                self.fonts_stack.pop();
            } else {
                layout_realized(elem, self, styles)?;
            }
        }

        Ok(())
    }
}

/// Lays out a leaf element resulting from realization.
fn layout_realized(
    elem: &Content,
    ctx: &mut MathContext,
    styles: StyleChain,
) -> SourceResult<()> {
    if let Some(elem) = elem.to_packed::<TagElem>() {
        ctx.push(MathFragment::Tag(elem.tag.clone()));
    } else if elem.is::<SpaceElem>() {
        ctx.push(MathFragment::Space(ctx.font().math().space_width.resolve(styles)));
    } else if elem.is::<LinebreakElem>() {
        ctx.push(MathFragment::Linebreak);
    } else if let Some(elem) = elem.to_packed::<HElem>() {
        layout_h(elem, ctx, styles)?;
    } else if let Some(elem) = elem.to_packed::<TextElem>() {
        self::text::layout_text(elem, ctx, styles)?;
    } else if let Some(elem) = elem.to_packed::<SymbolElem>() {
        self::text::layout_symbol(elem, ctx, styles)?;
    } else if let Some(elem) = elem.to_packed::<BoxElem>() {
        layout_box(elem, ctx, styles)?;
    } else if elem.is::<AlignPointElem>() {
        ctx.push(MathFragment::Align);
    } else if let Some(elem) = elem.to_packed::<ClassElem>() {
        layout_class(elem, ctx, styles)?;
    } else if let Some(elem) = elem.to_packed::<AccentElem>() {
        self::accent::layout_accent(elem, ctx, styles)?;
    } else if let Some(elem) = elem.to_packed::<AttachElem>() {
        self::attach::layout_attach(elem, ctx, styles)?;
    } else if let Some(elem) = elem.to_packed::<PrimesElem>() {
        self::attach::layout_primes(elem, ctx, styles)?;
    } else if let Some(elem) = elem.to_packed::<ScriptsElem>() {
        self::attach::layout_scripts(elem, ctx, styles)?;
    } else if let Some(elem) = elem.to_packed::<LimitsElem>() {
        self::attach::layout_limits(elem, ctx, styles)?;
    } else if let Some(elem) = elem.to_packed::<CancelElem>() {
        self::cancel::layout_cancel(elem, ctx, styles)?
    } else if let Some(elem) = elem.to_packed::<FracElem>() {
        self::frac::layout_frac(elem, ctx, styles)?;
    } else if let Some(elem) = elem.to_packed::<BinomElem>() {
        self::frac::layout_binom(elem, ctx, styles)?;
    } else if let Some(elem) = elem.to_packed::<LrElem>() {
        self::lr::layout_lr(elem, ctx, styles)?
    } else if let Some(elem) = elem.to_packed::<MidElem>() {
        self::lr::layout_mid(elem, ctx, styles)?
    } else if let Some(elem) = elem.to_packed::<VecElem>() {
        self::mat::layout_vec(elem, ctx, styles)?
    } else if let Some(elem) = elem.to_packed::<MatElem>() {
        self::mat::layout_mat(elem, ctx, styles)?
    } else if let Some(elem) = elem.to_packed::<CasesElem>() {
        self::mat::layout_cases(elem, ctx, styles)?
    } else if let Some(elem) = elem.to_packed::<OpElem>() {
        layout_op(elem, ctx, styles)?
    } else if let Some(elem) = elem.to_packed::<RootElem>() {
        self::root::layout_root(elem, ctx, styles)?
    } else if let Some(elem) = elem.to_packed::<StretchElem>() {
        self::stretch::layout_stretch(elem, ctx, styles)?
    } else if let Some(elem) = elem.to_packed::<UnderlineElem>() {
        self::underover::layout_underline(elem, ctx, styles)?
    } else if let Some(elem) = elem.to_packed::<OverlineElem>() {
        self::underover::layout_overline(elem, ctx, styles)?
    } else if let Some(elem) = elem.to_packed::<UnderbraceElem>() {
        self::underover::layout_underbrace(elem, ctx, styles)?
    } else if let Some(elem) = elem.to_packed::<OverbraceElem>() {
        self::underover::layout_overbrace(elem, ctx, styles)?
    } else if let Some(elem) = elem.to_packed::<UnderbracketElem>() {
        self::underover::layout_underbracket(elem, ctx, styles)?
    } else if let Some(elem) = elem.to_packed::<OverbracketElem>() {
        self::underover::layout_overbracket(elem, ctx, styles)?
    } else if let Some(elem) = elem.to_packed::<UnderparenElem>() {
        self::underover::layout_underparen(elem, ctx, styles)?
    } else if let Some(elem) = elem.to_packed::<OverparenElem>() {
        self::underover::layout_overparen(elem, ctx, styles)?
    } else if let Some(elem) = elem.to_packed::<UndershellElem>() {
        self::underover::layout_undershell(elem, ctx, styles)?
    } else if let Some(elem) = elem.to_packed::<OvershellElem>() {
        self::underover::layout_overshell(elem, ctx, styles)?
    } else {
        let mut frame = layout_external(elem, ctx, styles)?;
        if !frame.has_baseline() {
            let axis = ctx.font().math().axis_height.resolve(styles);
            frame.set_baseline(frame.height() / 2.0 + axis);
        }
        ctx.push(
            FrameFragment::new(styles, frame)
                .with_spaced(true)
                .with_ignorant(elem.is::<PlaceElem>()),
        );
    }

    Ok(())
}

/// Lays out an [`BoxElem`].
fn layout_box(
    elem: &Packed<BoxElem>,
    ctx: &mut MathContext,
    styles: StyleChain,
) -> SourceResult<()> {
    let frame = crate::inline::layout_box(
        elem,
        ctx.engine,
        ctx.locator.next(&elem.span()),
        styles,
        ctx.region.size,
    )?;
    ctx.push(FrameFragment::new(styles, frame).with_spaced(true));
    Ok(())
}

/// Lays out an [`HElem`].
fn layout_h(
    elem: &Packed<HElem>,
    ctx: &mut MathContext,
    styles: StyleChain,
) -> SourceResult<()> {
    if let Spacing::Rel(rel) = elem.amount
        && rel.rel.is_zero()
    {
        ctx.push(MathFragment::Spacing(rel.abs.resolve(styles), elem.weak.get(styles)));
    }
    Ok(())
}

/// Lays out a [`ClassElem`].
#[typst_macros::time(name = "math.class", span = elem.span())]
fn layout_class(
    elem: &Packed<ClassElem>,
    ctx: &mut MathContext,
    styles: StyleChain,
) -> SourceResult<()> {
    let style = EquationElem::class.set(Some(elem.class)).wrap();
    let mut fragment = ctx.layout_into_fragment(&elem.body, styles.chain(&style))?;
    fragment.set_class(elem.class);
    fragment.set_limits(Limits::for_class(elem.class));
    ctx.push(fragment);
    Ok(())
}

/// Lays out an [`OpElem`].
#[typst_macros::time(name = "math.op", span = elem.span())]
fn layout_op(
    elem: &Packed<OpElem>,
    ctx: &mut MathContext,
    styles: StyleChain,
) -> SourceResult<()> {
    let fragment = ctx.layout_into_fragment(&elem.text, styles)?;
    let italics = fragment.italics_correction();
    let accent_attach = fragment.accent_attach();
    let text_like = fragment.is_text_like();

    ctx.push(
        FrameFragment::new(styles, fragment.into_frame())
            .with_class(MathClass::Large)
            .with_italics_correction(italics)
            .with_accent_attach(accent_attach)
            .with_text_like(text_like)
            .with_limits(if elem.limits.get(styles) {
                Limits::Display
            } else {
                Limits::Never
            }),
    );
    Ok(())
}

/// Layout into a frame with normal layout.
fn layout_external(
    content: &Content,
    ctx: &mut MathContext,
    styles: StyleChain,
) -> SourceResult<Frame> {
    crate::layout_frame(
        ctx.engine,
        content,
        ctx.locator.next(&content.span()),
        styles,
        ctx.region,
    )
}

/// Styles to add font constants to the style chain.
fn style_for_script_scale(font: &Font) -> LazyHash<Style> {
    EquationElem::script_scale
        .set((
            font.math().script_percent_scale_down,
            font.math().script_script_percent_scale_down,
        ))
        .wrap()
}

/// Get the current base font.
fn get_font(
    world: Tracked<dyn World + '_>,
    styles: StyleChain,
    span: Span,
) -> SourceResult<Font> {
    let variant = variant(styles);
    families(styles)
        .find_map(|family| {
            world
                .book()
                .select(family.as_str(), variant)
                .and_then(|id| world.font(id))
                .filter(|_| family.covers().is_none())
        })
        .ok_or("no font could be found")
        .at(span)
}

/// Check if the top-level base font has a MATH table.
fn warn_non_math_font(font: &Font, engine: &mut Engine, span: Span) {
    if !font.info().flags.contains(FontFlags::MATH) {
        engine.sink.warn(warning!(
            span,
            "current font is not designed for math";
            hint: "rendering may be poor"
        ))
    }
}
