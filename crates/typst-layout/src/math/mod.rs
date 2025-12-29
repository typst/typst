mod accent;
mod cancel;
mod fenced;
mod fraction;
mod fragment;
mod line;
mod radical;
mod run;
mod scripts;
mod shaping;
mod table;
mod text;

use comemo::Tracked;
use typst_library::World;
use typst_library::diag::{At, SourceResult, warning};
use typst_library::engine::Engine;
use typst_library::foundations::{NativeElement, Packed, Resolve, Style, StyleChain};
use typst_library::introspection::{Counter, Locator, SplitLocator};
use typst_library::layout::{
    Abs, AlignElem, Axes, BlockElem, Em, FixedAlignment, Fragment, Frame, InlineItem,
    OuterHAlignment, Point, Region, Regions, Size, SpecificAlignment, VAlignment,
};
use typst_library::math::ir::{
    BoxItem, ExternalItem, MathItem, MathKind, MathProperties, resolve_equation,
};
use typst_library::math::{EquationElem, families};
use typst_library::model::ParElem;
use typst_library::routines::Arenas;
use typst_library::text::{Font, FontFlags, TextEdgeBounds, TextElem, variant};
use typst_syntax::Span;
use typst_utils::{LazyHash, Numeric};

use self::accent::layout_accent;
use self::cancel::layout_cancel;
use self::fenced::layout_fenced;
use self::fraction::{layout_fraction, layout_skewed_fraction};
use self::fragment::{FrameFragment, MathFragment};
use self::line::layout_line;
use self::radical::layout_radical;
use self::run::{MathFragmentsExt, MathRunFrameBuilder};
use self::scripts::{layout_primes, layout_scripts};
use self::table::layout_table;
use self::text::{layout_glyph, layout_text};

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

    let scale_style = style_for_script_scale(&font);
    let styles = styles.chain(&scale_style);

    let mut locator = locator.split();

    let arenas = Arenas::default();
    let item = resolve_equation(elem, engine, &mut locator, &arenas, styles)?;

    let mut ctx = MathContext::new(engine, &mut locator, region, font.clone());
    let mut items = if !item.is_multiline() {
        ctx.layout_into_fragments(&item, styles)?.into_par_items()
    } else {
        vec![InlineItem::Frame(ctx.layout_into_fragment(&item, styles)?.into_frame())]
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

    let scale_style = style_for_script_scale(&font);
    let styles = styles.chain(&scale_style);

    let mut locator = locator.split();

    let arenas = Arenas::default();
    let item = resolve_equation(elem, engine, &mut locator, &arenas, styles)?;

    let mut ctx = MathContext::new(engine, &mut locator, regions.base(), font.clone());
    let full_equation_builder = ctx
        .layout_into_fragments(&item, styles)?
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
        .display_at(engine, elem.location().unwrap(), styles, numbering, span)?
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

    /// Layout the given math item and return the resulting [`MathFragment`]s.
    fn layout_into_fragments(
        &mut self,
        item: &MathItem,
        styles: StyleChain,
    ) -> SourceResult<Vec<MathFragment>> {
        let start = self.fragments.len();
        self.layout_into_self(item, styles)?;
        Ok(self.fragments.drain(start..).collect())
    }

    /// Layout the given math item and return the resulting [`MathFragment`]s.
    fn layout_into_fragment(
        &mut self,
        item: &MathItem,
        styles: StyleChain,
    ) -> SourceResult<MathFragment> {
        let fragments = self.layout_into_fragments(item, styles)?;
        if fragments.len() == 1 {
            return Ok(fragments.into_iter().next().unwrap());
        }

        // Fragments without a math_size are ignored: the notion of size does
        // not apply to them, so their text-likeness is meaningless.
        let text_like = fragments
            .iter()
            .filter(|e| e.math_size().is_some())
            .all(|e| e.is_text_like());

        let styles = item.styles().unwrap_or(styles);
        let props = MathProperties::default(styles);
        let frame = fragments.into_frame(styles);
        Ok(FrameFragment::new(&props, styles, frame)
            .with_text_like(text_like)
            .into())
    }

    fn layout_into_self(
        &mut self,
        item: &MathItem,
        styles: StyleChain,
    ) -> SourceResult<()> {
        let outer_styles = item.styles().unwrap_or(styles);
        let outer_font = outer_styles.get_ref(TextElem::font);

        for item in item.as_slice() {
            let styles = item.styles().unwrap_or(outer_styles);

            // Whilst this check isn't exact, it more or less suffices as a
            // change in font variant probably won't have an effect on metrics.
            if styles != outer_styles && styles.get_ref(TextElem::font) != outer_font {
                self.fonts_stack
                    .push(get_font(self.engine.world, styles, item.span())?);
                let scale_style = style_for_script_scale(self.font());
                layout_realized(item, self, styles.chain(&scale_style))?;
                self.fonts_stack.pop();
            } else {
                layout_realized(item, self, styles)?;
            }
        }

        Ok(())
    }
}

/// Lays out a single math item.
fn layout_realized(
    item: &MathItem,
    ctx: &mut MathContext,
    styles: StyleChain,
) -> SourceResult<()> {
    // Handle non-component items first.
    let MathItem::Component(comp) = item else {
        match item {
            MathItem::Spacing(amount, _) => ctx.push(MathFragment::Space(*amount)),
            MathItem::Space => ctx
                .push(MathFragment::Space(ctx.font().math().space_width.resolve(styles))),
            MathItem::Linebreak => ctx.push(MathFragment::Linebreak),
            MathItem::Align => ctx.push(MathFragment::Align),
            MathItem::Tag(tag) => ctx.push(MathFragment::Tag(tag.clone())),
            _ => unreachable!(),
        }
        return Ok(());
    };

    let props = &comp.props;

    // Insert left spacing.
    if let Some(lspace) = props.lspace {
        let width = lspace.at(styles.resolve(TextElem::size));
        let frag = MathFragment::Space(width);
        if let Some(i) = ctx.fragments.iter().rposition(|f| !f.is_ignorant())
            && matches!(ctx.fragments[i], MathFragment::Align)
        {
            // Skip a single alignment point (if one exists) when placing
            // spacing on the left.
            ctx.fragments.insert(i, frag);
        } else {
            ctx.push(frag);
        }
    }

    // Dispatch based on item kind to the appropriate layout function.
    match &comp.kind {
        MathKind::Box(item) => layout_box(item, ctx, styles, props)?,
        MathKind::External(item) => layout_external(item, ctx, styles, props)?,
        MathKind::Glyph(item) => layout_glyph(item, ctx, styles, props)?,
        MathKind::Cancel(item) => layout_cancel(item, ctx, styles, props)?,
        MathKind::Radical(item) => layout_radical(item, ctx, styles, props)?,
        MathKind::Line(item) => layout_line(item, ctx, styles, props)?,
        MathKind::Accent(item) => layout_accent(item, ctx, styles, props)?,
        MathKind::Scripts(item) => layout_scripts(item, ctx, styles, props)?,
        MathKind::Primes(item) => layout_primes(item, ctx, styles, props)?,
        MathKind::Table(item) => layout_table(item, ctx, styles, props)?,
        MathKind::Fraction(item) => layout_fraction(item, ctx, styles, props)?,
        MathKind::SkewedFraction(item) => {
            layout_skewed_fraction(item, ctx, styles, props)?
        }
        MathKind::Text(item) => layout_text(item, ctx, styles, props)?,
        MathKind::Fenced(item) => layout_fenced(item, ctx, styles, props)?,
        MathKind::Group(_) => {
            let fragment = ctx.layout_into_fragment(item, styles)?;
            let italics = fragment.italics_correction();
            let accent_attach = fragment.accent_attach();
            ctx.push(
                FrameFragment::new(props, styles, fragment.into_frame())
                    .with_italics_correction(italics)
                    .with_accent_attach(accent_attach),
            );
        }
    }

    // Insert right spacing.
    if let Some(rspace) = props.rspace {
        let width = rspace.at(styles.resolve(TextElem::size));
        ctx.push(MathFragment::Space(width));
    }

    Ok(())
}

/// Lays out a [`BoxItem`].
fn layout_box(
    item: &BoxItem,
    ctx: &mut MathContext,
    styles: StyleChain,
    props: &MathProperties,
) -> SourceResult<()> {
    let frame = crate::inline::layout_box(
        item.elem,
        ctx.engine,
        ctx.locator.next(&item.elem.span()),
        styles,
        ctx.region.size,
    )?;
    ctx.push(FrameFragment::new(props, styles, frame));
    Ok(())
}

/// Layout into a frame with normal layout.
fn layout_external(
    item: &ExternalItem,
    ctx: &mut MathContext,
    styles: StyleChain,
    props: &MathProperties,
) -> SourceResult<()> {
    let mut frame = crate::layout_frame(
        ctx.engine,
        item.content,
        ctx.locator.next(&item.content.span()),
        styles,
        ctx.region,
    )?;
    if !frame.has_baseline() {
        let axis = ctx.font().math().axis_height.resolve(styles);
        frame.set_baseline(frame.height() / 2.0 + axis);
    }
    ctx.push(FrameFragment::new(props, styles, frame));
    Ok(())
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
            hint: "rendering may be poor";
        ))
    }
}
