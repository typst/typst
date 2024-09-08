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

use ttf_parser::gsub::SubstitutionSubtable;
use typst_library::diag::{bail, SourceResult};
use typst_library::engine::Engine;
use typst_library::foundations::{Content, NativeElement, Packed, Resolve, StyleChain};
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
    families, features, variant, Font, LinebreakElem, SpaceElem, TextEdgeBounds,
    TextElem, TextSize,
};
use typst_library::World;
use typst_syntax::Span;
use typst_utils::Numeric;
use unicode_math_class::MathClass;

use self::fragment::{
    FrameFragment, GlyphFragment, GlyphwiseSubsts, Limits, MathFragment, VariantFragment,
};
use self::run::{LeftRightAlternator, MathRun, MathRunFrameBuilder};
use self::shared::*;
use self::stretch::{stretch_fragment, stretch_glyph};

/// Layout an inline equation (in a paragraph).
#[typst_macros::time(span = elem.span())]
pub fn layout_equation_inline(
    elem: &Packed<EquationElem>,
    engine: &mut Engine,
    locator: Locator,
    styles: StyleChain,
    region: Size,
) -> SourceResult<Vec<InlineItem>> {
    assert!(!elem.block(styles));

    let font = find_math_font(engine, styles, elem.span())?;

    let mut locator = locator.split();
    let mut ctx = MathContext::new(engine, &mut locator, styles, region, &font);
    let run = ctx.layout_into_run(&elem.body, styles)?;

    let mut items = if run.row_count() == 1 {
        run.into_par_items()
    } else {
        vec![InlineItem::Frame(run.into_fragment(&ctx, styles).into_frame())]
    };

    // An empty equation should have a height, so we still create a frame
    // (which is then resized in the loop).
    if items.is_empty() {
        items.push(InlineItem::Frame(Frame::soft(Size::zero())));
    }

    for item in &mut items {
        let InlineItem::Frame(frame) = item else { continue };

        let font_size = scaled_font_size(&ctx, styles);
        let slack = ParElem::leading_in(styles) * 0.7;

        let (t, b) = font.edges(
            TextElem::top_edge_in(styles),
            TextElem::bottom_edge_in(styles),
            font_size,
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
    assert!(elem.block(styles));

    let span = elem.span();
    let font = find_math_font(engine, styles, span)?;

    let mut locator = locator.split();
    let mut ctx = MathContext::new(engine, &mut locator, styles, regions.base(), &font);
    let full_equation_builder = ctx
        .layout_into_run(&elem.body, styles)?
        .multiline_frame_builder(&ctx, styles);
    let width = full_equation_builder.size.x;

    let equation_builders = if BlockElem::breakable_in(styles) {
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

    let Some(numbering) = (**elem).numbering(styles) else {
        let frames = equation_builders
            .into_iter()
            .map(MathRunFrameBuilder::build)
            .collect();
        return Ok(Fragment::frames(frames));
    };

    let pod = Region::new(regions.base(), Axes::splat(false));
    let counter = Counter::of(EquationElem::elem())
        .display_at_loc(engine, elem.location().unwrap(), styles, numbering)?
        .spanned(span);
    let number =
        (engine.routines.layout_frame)(engine, &counter, locator.next(&()), styles, pod)?;

    static NUMBER_GUTTER: Em = Em::new(0.5);
    let full_number_width = number.width() + NUMBER_GUTTER.resolve(styles);

    let number_align = match elem.number_align(styles) {
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
                AlignElem::alignment_in(styles).resolve(styles).x,
                regions.size.x,
                full_number_width,
            )
        })
        .collect();

    Ok(Fragment::frames(frames))
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
    // Font-related.
    font: &'a Font,
    ttf: &'a ttf_parser::Face<'a>,
    table: ttf_parser::math::Table<'a>,
    constants: ttf_parser::math::Constants<'a>,
    ssty_table: Option<ttf_parser::gsub::AlternateSubstitution<'a>>,
    glyphwise_tables: Option<Vec<GlyphwiseSubsts<'a>>>,
    space_width: Em,
    // Mutable.
    fragments: Vec<MathFragment>,
}

impl<'a, 'v, 'e> MathContext<'a, 'v, 'e> {
    /// Create a new math context.
    fn new(
        engine: &'v mut Engine<'e>,
        locator: &'v mut SplitLocator<'a>,
        styles: StyleChain<'a>,
        base: Size,
        font: &'a Font,
    ) -> Self {
        let math_table = font.ttf().tables().math.unwrap();
        let gsub_table = font.ttf().tables().gsub;
        let constants = math_table.constants.unwrap();

        let ssty_table = gsub_table
            .and_then(|gsub| {
                gsub.features
                    .find(ttf_parser::Tag::from_bytes(b"ssty"))
                    .and_then(|feature| feature.lookup_indices.get(0))
                    .and_then(|index| gsub.lookups.get(index))
            })
            .and_then(|ssty| ssty.subtables.get::<SubstitutionSubtable>(0))
            .and_then(|ssty| match ssty {
                SubstitutionSubtable::Alternate(alt_glyphs) => Some(alt_glyphs),
                _ => None,
            });

        let features = features(styles);
        let glyphwise_tables = gsub_table.map(|gsub| {
            features
                .into_iter()
                .filter_map(|feature| GlyphwiseSubsts::new(gsub, feature))
                .collect()
        });

        let ttf = font.ttf();
        let space_width = ttf
            .glyph_index(' ')
            .and_then(|id| ttf.glyph_hor_advance(id))
            .map(|advance| font.to_em(advance))
            .unwrap_or(THICK);

        Self {
            engine,
            locator,
            region: Region::new(base, Axes::splat(false)),
            font,
            ttf: font.ttf(),
            table: math_table,
            constants,
            ssty_table,
            glyphwise_tables,
            space_width,
            fragments: vec![],
        }
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
        Ok(self.layout_into_run(elem, styles)?.into_fragment(self, styles))
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

        let outer = styles;
        for (elem, styles) in pairs {
            // Hack because the font is fixed in math.
            if styles != outer && TextElem::font_in(styles) != TextElem::font_in(outer) {
                let frame = layout_external(elem, self, styles)?;
                self.push(FrameFragment::new(self, styles, frame).with_spaced(true));
                continue;
            }

            layout_realized(elem, self, styles)?;
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
        let font_size = scaled_font_size(ctx, styles);
        ctx.push(MathFragment::Space(ctx.space_width.at(font_size)));
    } else if elem.is::<LinebreakElem>() {
        ctx.push(MathFragment::Linebreak);
    } else if let Some(elem) = elem.to_packed::<HElem>() {
        layout_h(elem, ctx, styles)?;
    } else if let Some(elem) = elem.to_packed::<TextElem>() {
        self::text::layout_text(elem, ctx, styles)?;
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
    } else if let Some(elem) = elem.to_packed::<LongdivisionElem>() {
        self::root::layout_longdivision(elem, ctx, styles)?
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
            let axis = scaled!(ctx, styles, axis_height);
            frame.set_baseline(frame.height() / 2.0 + axis);
        }
        ctx.push(
            FrameFragment::new(ctx, styles, frame)
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
    let local = TextElem::set_size(TextSize(scaled_font_size(ctx, styles).into())).wrap();
    let frame = (ctx.engine.routines.layout_box)(
        elem,
        ctx.engine,
        ctx.locator.next(&elem.span()),
        styles.chain(&local),
        ctx.region.size,
    )?;
    ctx.push(FrameFragment::new(ctx, styles, frame).with_spaced(true));
    Ok(())
}

/// Lays out an [`HElem`].
fn layout_h(
    elem: &Packed<HElem>,
    ctx: &mut MathContext,
    styles: StyleChain,
) -> SourceResult<()> {
    if let Spacing::Rel(rel) = elem.amount() {
        if rel.rel.is_zero() {
            ctx.push(MathFragment::Spacing(
                rel.abs.at(scaled_font_size(ctx, styles)),
                elem.weak(styles),
            ));
        }
    }
    Ok(())
}

/// Lays out a [`ClassElem`].
#[typst_macros::time(name = "math.op", span = elem.span())]
fn layout_class(
    elem: &Packed<ClassElem>,
    ctx: &mut MathContext,
    styles: StyleChain,
) -> SourceResult<()> {
    let class = *elem.class();
    let style = EquationElem::set_class(Some(class)).wrap();
    let mut fragment = ctx.layout_into_fragment(elem.body(), styles.chain(&style))?;
    fragment.set_class(class);
    fragment.set_limits(Limits::for_class(class));
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
    let fragment = ctx.layout_into_fragment(elem.text(), styles)?;
    let italics = fragment.italics_correction();
    let accent_attach = fragment.accent_attach();
    let text_like = fragment.is_text_like();

    ctx.push(
        FrameFragment::new(ctx, styles, fragment.into_frame())
            .with_class(MathClass::Large)
            .with_italics_correction(italics)
            .with_accent_attach(accent_attach)
            .with_text_like(text_like)
            .with_limits(if elem.limits(styles) {
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
    let local = TextElem::set_size(TextSize(scaled_font_size(ctx, styles).into())).wrap();
    (ctx.engine.routines.layout_frame)(
        ctx.engine,
        content,
        ctx.locator.next(&content.span()),
        styles.chain(&local),
        ctx.region,
    )
}
