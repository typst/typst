use comemo::Track;
use ecow::{EcoVec, eco_format};
use smallvec::smallvec;
use typst_library::diag::{At, SourceResult, bail};
use typst_library::foundations::{
    Content, Context, NativeElement, NativeRuleMap, Packed, Resolve, ShowFn, Smart,
    StyleChain, Target, dict,
};
use typst_library::introspection::{Counter, Locator, LocatorLink};
use typst_library::layout::{
    Abs, AlignElem, Alignment, Axes, BlockBody, BlockElem, ColumnsElem, Em,
    FixedAlignment, GridCell, GridChild, GridElem, GridItem, HAlignment, HElem, HideElem,
    InlineElem, LayoutElem, Length, MoveElem, OuterVAlignment, PadElem, PageElem,
    PlaceElem, PlacementScope, Region, Rel, RepeatElem, RotateElem, ScaleElem, Sides,
    Size, Sizing, SkewElem, Spacing, StackChild, StackElem, TrackSizings, VElem,
};
use typst_library::math::EquationElem;
use typst_library::model::{
    Attribution, BibliographyElem, CiteElem, CiteGroup, CslIndentElem, CslLightElem,
    Destination, DirectLinkElem, EmphElem, EnumElem, FigureCaption, FigureElem,
    FootnoteElem, FootnoteEntry, HeadingElem, LinkElem, LinkMarker, ListElem,
    OutlineBody, OutlineElem, OutlineEntry, ParElem, ParbreakElem, QuoteElem, RefElem,
    StrongElem, TableCell, TableElem, TermsElem, TitleElem, Works,
};
use typst_library::pdf::{ArtifactElem, AttachElem};
use typst_library::text::{
    DecoLine, Decoration, HighlightElem, ItalicToggle, LinebreakElem, LocalName,
    OverlineElem, RawElem, RawLine, ScriptKind, ShiftSettings, Smallcaps, SmallcapsElem,
    SmartQuoteElem, SmartQuotes, SpaceElem, StrikeElem, SubElem, SuperElem, TextElem,
    TextSize, UnderlineElem, WeightDelta,
};
use typst_library::visualize::{
    CircleElem, CurveElem, EllipseElem, ImageElem, LineElem, PathElem, PolygonElem,
    RectElem, SquareElem, Stroke,
};
use typst_utils::{Get, Numeric};

/// Register show rules for the [paged target](Target::Paged).
pub fn register(rules: &mut NativeRuleMap) {
    use Target::Paged;

    // Model.
    rules.register(Paged, STRONG_RULE);
    rules.register(Paged, EMPH_RULE);
    rules.register(Paged, LIST_RULE);
    rules.register(Paged, ENUM_RULE);
    rules.register(Paged, TERMS_RULE);
    rules.register(Paged, LINK_MARKER_RULE);
    rules.register(Paged, LINK_RULE);
    rules.register(Paged, DIRECT_LINK_RULE);
    rules.register(Paged, TITLE_RULE);
    rules.register(Paged, HEADING_RULE);
    rules.register(Paged, FIGURE_RULE);
    rules.register(Paged, FIGURE_CAPTION_RULE);
    rules.register(Paged, QUOTE_RULE);
    rules.register(Paged, FOOTNOTE_RULE);
    rules.register(Paged, FOOTNOTE_ENTRY_RULE);
    rules.register(Paged, OUTLINE_RULE);
    rules.register(Paged, OUTLINE_BODY_RULE);
    rules.register(Paged, OUTLINE_ENTRY_RULE);
    rules.register(Paged, REF_RULE);
    rules.register(Paged, CITE_GROUP_RULE);
    rules.register(Paged, BIBLIOGRAPHY_RULE);
    rules.register(Paged, CSL_LIGHT_RULE);
    rules.register(Paged, CSL_INDENT_RULE);
    rules.register(Paged, TABLE_RULE);
    rules.register(Paged, TABLE_CELL_RULE);

    // Text.
    rules.register(Paged, SUB_RULE);
    rules.register(Paged, SUPER_RULE);
    rules.register(Paged, UNDERLINE_RULE);
    rules.register(Paged, OVERLINE_RULE);
    rules.register(Paged, STRIKE_RULE);
    rules.register(Paged, HIGHLIGHT_RULE);
    rules.register(Paged, SMALLCAPS_RULE);
    rules.register(Paged, RAW_RULE);
    rules.register(Paged, RAW_LINE_RULE);

    // Layout.
    rules.register(Paged, ALIGN_RULE);
    rules.register(Paged, PAD_RULE);
    rules.register(Paged, COLUMNS_RULE);
    rules.register(Paged, STACK_RULE);
    rules.register(Paged, GRID_RULE);
    rules.register(Paged, GRID_CELL_RULE);
    rules.register(Paged, MOVE_RULE);
    rules.register(Paged, SCALE_RULE);
    rules.register(Paged, ROTATE_RULE);
    rules.register(Paged, SKEW_RULE);
    rules.register(Paged, REPEAT_RULE);
    rules.register(Paged, HIDE_RULE);
    rules.register(Paged, LAYOUT_RULE);

    // Visualize.
    rules.register(Paged, IMAGE_RULE);
    rules.register(Paged, LINE_RULE);
    rules.register(Paged, RECT_RULE);
    rules.register(Paged, SQUARE_RULE);
    rules.register(Paged, ELLIPSE_RULE);
    rules.register(Paged, CIRCLE_RULE);
    rules.register(Paged, POLYGON_RULE);
    rules.register(Paged, CURVE_RULE);
    rules.register(Paged, PATH_RULE);

    // Math.
    rules.register(Paged, EQUATION_RULE);

    // PDF.
    rules.register(Paged, ATTACH_RULE);
    rules.register(Paged, ARTIFACT_RULE);
}

const STRONG_RULE: ShowFn<StrongElem> = |elem, _, styles| {
    Ok(elem
        .body
        .clone()
        .set(TextElem::delta, WeightDelta(elem.delta.get(styles))))
};

const EMPH_RULE: ShowFn<EmphElem> =
    |elem, _, _| Ok(elem.body.clone().set(TextElem::emph, ItalicToggle(true)));

const LIST_RULE: ShowFn<ListElem> = |elem, _, styles| {
    let tight = elem.tight.get(styles);

    let mut realized = BlockElem::multi_layouter(elem.clone(), crate::lists::layout_list)
        .pack()
        .spanned(elem.span());

    if tight {
        let spacing = elem
            .spacing
            .get(styles)
            .unwrap_or_else(|| styles.get(ParElem::leading));
        let v = VElem::new(spacing.into()).with_weak(true).with_attach(true).pack();
        realized = v + realized;
    }

    Ok(realized)
};

const ENUM_RULE: ShowFn<EnumElem> = |elem, _, styles| {
    let tight = elem.tight.get(styles);

    let mut realized = BlockElem::multi_layouter(elem.clone(), crate::lists::layout_enum)
        .pack()
        .spanned(elem.span());

    if tight {
        let spacing = elem
            .spacing
            .get(styles)
            .unwrap_or_else(|| styles.get(ParElem::leading));
        let v = VElem::new(spacing.into()).with_weak(true).with_attach(true).pack();
        realized = v + realized;
    }

    Ok(realized)
};

const TERMS_RULE: ShowFn<TermsElem> = |elem, _, styles| {
    let span = elem.span();
    let tight = elem.tight.get(styles);

    let separator = elem.separator.get_ref(styles);
    let indent = elem.indent.get(styles);
    let hanging_indent = elem.hanging_indent.get(styles);
    let gutter = elem.spacing.get(styles).unwrap_or_else(|| {
        if tight { styles.get(ParElem::leading) } else { styles.get(ParElem::spacing) }
    });

    let pad = hanging_indent + indent;
    let unpad = (!hanging_indent.is_zero())
        .then(|| HElem::new((-hanging_indent).into()).pack().spanned(span));

    let mut children = vec![];
    for child in elem.children.iter() {
        let mut seq = vec![];
        seq.extend(unpad.clone());
        seq.push(child.term.clone().strong());
        seq.push(separator.clone());
        seq.push(child.description.clone());

        // Text in wide term lists shall always turn into paragraphs.
        if !tight {
            seq.push(ParbreakElem::shared().clone());
        }

        children.push(StackChild::Block(Content::sequence(seq)));
    }

    let padding =
        Sides::default().with(styles.resolve(TextElem::dir).start(), pad.into());

    let mut realized = StackElem::new(children)
        .with_spacing(Some(gutter.into()))
        .pack()
        .spanned(span)
        .padded(padding)
        .set(TermsElem::within, true);

    if tight {
        let spacing = elem
            .spacing
            .get(styles)
            .unwrap_or_else(|| styles.get(ParElem::leading));
        let v = VElem::new(spacing.into())
            .with_weak(true)
            .with_attach(true)
            .pack()
            .spanned(span);
        realized = v + realized;
    }

    Ok(realized)
};

const LINK_MARKER_RULE: ShowFn<LinkMarker> = |elem, _, _| Ok(elem.body.clone());

const LINK_RULE: ShowFn<LinkElem> = |elem, engine, styles| {
    let body = elem.body.clone();
    let dest = elem.dest.resolve(engine.introspector).at(elem.span())?;
    let alt = dest.alt_text(engine, styles)?;
    Ok(body.linked(dest, Some(alt)))
};

const DIRECT_LINK_RULE: ShowFn<DirectLinkElem> = |elem, _, _| {
    let dest = Destination::Location(elem.loc);
    Ok(elem.body.clone().linked(dest, elem.alt.clone()))
};

const TITLE_RULE: ShowFn<TitleElem> = |elem, _, styles| {
    Ok(BlockElem::new()
        .with_body(Some(BlockBody::Content(elem.resolve_body(styles).at(elem.span())?)))
        .pack())
};

const HEADING_RULE: ShowFn<HeadingElem> = |elem, engine, styles| {
    const SPACING_TO_NUMBERING: Em = Em::new(0.3);

    let span = elem.span();
    let mut realized = elem.body.clone();

    let hanging_indent = elem.hanging_indent.get(styles);
    let mut indent = match hanging_indent {
        Smart::Custom(length) => length.resolve(styles),
        Smart::Auto => Abs::zero(),
    };

    if let Some(numbering) = elem.numbering.get_ref(styles).as_ref() {
        let location = elem.location().unwrap();
        let numbering = Counter::of(HeadingElem::ELEM)
            .display_at_loc(engine, location, styles, numbering)?
            .spanned(span);
        let align = styles.resolve(AlignElem::alignment);

        if hanging_indent.is_auto() && align.x == FixedAlignment::Start {
            let pod = Region::new(Axes::splat(Abs::inf()), Axes::splat(false));

            // We don't have a locator for the numbering here, so we just
            // use the measurement infrastructure for now.
            let link = LocatorLink::measure(location);
            let size = (engine.routines.layout_frame)(
                engine,
                &numbering,
                Locator::link(&link),
                styles,
                pod,
            )?
            .size();

            indent = size.x + SPACING_TO_NUMBERING.resolve(styles);
        }

        let spacing = HElem::new(SPACING_TO_NUMBERING.into()).with_weak(true).pack();

        realized = numbering + spacing + realized;
    }

    let block = if indent != Abs::zero() {
        let body = HElem::new((-indent).into()).pack() + realized;
        let inset = Sides::default()
            .with(styles.resolve(TextElem::dir).start(), Some(indent.into()));
        BlockElem::new()
            .with_body(Some(BlockBody::Content(body)))
            .with_inset(inset)
    } else {
        BlockElem::new().with_body(Some(BlockBody::Content(realized)))
    };

    Ok(block.pack())
};

const FIGURE_RULE: ShowFn<FigureElem> = |elem, _, styles| {
    let span = elem.span();
    let mut realized = elem.body.clone();

    // Build the caption, if any.
    if let Some(caption) = elem.caption.get_cloned(styles) {
        let (first, second) = match caption.position.get(styles) {
            OuterVAlignment::Top => (caption.pack(), realized),
            OuterVAlignment::Bottom => (realized, caption.pack()),
        };
        realized = Content::sequence(vec![
            first,
            VElem::new(elem.gap.get(styles).into())
                .with_weak(true)
                .pack()
                .spanned(span),
            second,
        ]);
    }

    // Ensure that the body is considered a paragraph.
    realized += ParbreakElem::shared().clone().spanned(span);

    // Wrap the contents in a block.
    realized = BlockElem::new()
        .with_body(Some(BlockBody::Content(realized)))
        .pack()
        .spanned(span);

    // Wrap in a float.
    if let Some(align) = elem.placement.get(styles) {
        realized = PlaceElem::new(realized)
            .with_alignment(align.map(|align| HAlignment::Center + align))
            .with_scope(elem.scope.get(styles))
            .with_float(true)
            .pack()
            .spanned(span);
    } else if elem.scope.get(styles) == PlacementScope::Parent {
        bail!(
            span,
            "parent-scoped placement is only available for floating figures";
            hint: "you can enable floating placement with `figure(placement: auto, ..)`"
        );
    }

    Ok(realized)
};

const FIGURE_CAPTION_RULE: ShowFn<FigureCaption> = |elem, engine, styles| {
    Ok(BlockElem::new()
        .with_body(Some(BlockBody::Content(elem.realize(engine, styles)?)))
        .pack())
};

const QUOTE_RULE: ShowFn<QuoteElem> = |elem, _, styles| {
    let span = elem.span();
    let block = elem.block.get(styles);

    let mut realized = elem.body.clone();

    if elem.quotes.get(styles).unwrap_or(!block) {
        // Add zero-width weak spacing to make the quotes "sticky".
        let hole = HElem::hole();
        let sticky = Content::sequence([hole.clone(), realized, hole.clone()]);
        realized = QuoteElem::quoted(sticky, styles);
    }

    let attribution = elem.attribution.get_ref(styles);

    if block {
        realized = BlockElem::new()
            .with_body(Some(BlockBody::Content(realized)))
            .pack()
            .spanned(span);

        if let Some(attribution) = attribution.as_ref() {
            // Bring the attribution a bit closer to the quote.
            let gap = Spacing::Rel(Em::new(0.9).into());
            let v = VElem::new(gap).with_weak(true).pack();
            realized += v;
            realized += BlockElem::new()
                .with_body(Some(BlockBody::Content(attribution.realize(span))))
                .pack()
                .aligned(Alignment::END);
        }

        realized = PadElem::new(realized).pack();
    } else if let Some(Attribution::Label(label)) = attribution {
        realized += SpaceElem::shared().clone();
        realized += CiteElem::new(*label).pack().spanned(span);
    }

    Ok(realized)
};

const FOOTNOTE_RULE: ShowFn<FootnoteElem> = |elem, engine, styles| {
    let span = elem.span();
    let (dest, num) = elem.realize(engine, styles)?;
    let alt = FootnoteElem::alt_text(styles, &num.plain_text());
    let sup = SuperElem::new(num).pack().spanned(span);
    Ok(HElem::hole().clone() + sup.linked(dest, Some(alt)))
};

const FOOTNOTE_ENTRY_RULE: ShowFn<FootnoteEntry> = |elem, engine, styles| {
    let number_gap = Em::new(0.05);
    let (prefix, body) = elem.realize(engine, styles)?;
    Ok(Content::sequence([
        HElem::new(elem.indent.get(styles).into()).pack(),
        prefix,
        HElem::new(number_gap.into()).with_weak(true).pack(),
        body,
    ]))
};

const OUTLINE_RULE: ShowFn<OutlineElem> = |elem, engine, styles| {
    let title = elem.realize_title(styles);
    let entries = elem.realize_flat(engine, styles)?;
    let entries = entries.into_iter().map(|entry| entry.pack());
    let body = OutlineBody::new(Content::sequence(entries)).pack();
    Ok(Content::sequence(title.into_iter().chain(Some(body))))
};

const OUTLINE_BODY_RULE: ShowFn<OutlineBody> = |elem, _, _| Ok(elem.body.clone());

const OUTLINE_ENTRY_RULE: ShowFn<OutlineEntry> = |elem, engine, styles| {
    let span = elem.span();
    let context = Context::new(None, Some(styles));
    let context = context.track();

    let prefix = elem.prefix(engine, context, span)?;
    let body = elem.body().at(span)?;
    let page = elem.page(engine, context, span)?;
    let alt = {
        let prefix = prefix.as_ref().map(|p| p.plain_text()).unwrap_or_default();
        let body = body.plain_text();
        let page_str = PageElem::local_name_in(styles);
        let page_nr = page.plain_text();
        let quotes = SmartQuotes::get(
            styles.get_ref(SmartQuoteElem::quotes),
            styles.get(TextElem::lang),
            styles.get(TextElem::region),
            styles.get(SmartQuoteElem::alternative),
        );
        let open = quotes.double_open;
        let close = quotes.double_close;
        eco_format!("{prefix} {open}{body}{close} {page_str} {page_nr}",)
    };
    let inner = elem.build_inner(context, span, body, page)?;
    let block = if elem.element.is::<EquationElem>() {
        // Equation has no body and no levels, so indenting makes no sense.
        let body = prefix.unwrap_or_default() + inner;
        BlockElem::new()
            .with_body(Some(BlockBody::Content(body)))
            .pack()
            .spanned(span)
    } else {
        elem.indented(engine, context, span, prefix, inner, Em::new(0.5).into())?
    };

    let loc = elem.element_location().at(span)?;
    Ok(block.linked(Destination::Location(loc), Some(alt)))
};

const REF_RULE: ShowFn<RefElem> = |elem, engine, styles| elem.realize(engine, styles);

const CITE_GROUP_RULE: ShowFn<CiteGroup> = |elem, engine, _| elem.realize(engine);

const BIBLIOGRAPHY_RULE: ShowFn<BibliographyElem> = |elem, engine, styles| {
    const COLUMN_GUTTER: Em = Em::new(0.65);
    const INDENT: Em = Em::new(1.5);

    let span = elem.span();

    let mut seq = vec![];
    seq.extend(elem.realize_title(styles));

    let works = Works::generate(engine).at(span)?;
    let references = works.references(elem, styles)?;

    if references.iter().any(|(prefix, ..)| prefix.is_some()) {
        let row_gutter = styles.get(ParElem::spacing);

        let mut cells = vec![];
        for (prefix, reference, loc) in references {
            cells.push(GridChild::Item(GridItem::Cell(
                Packed::new(GridCell::new(
                    prefix.clone().unwrap_or_default().located(*loc),
                ))
                .spanned(span),
            )));
            cells.push(GridChild::Item(GridItem::Cell(
                Packed::new(GridCell::new(reference.clone())).spanned(span),
            )));
        }
        seq.push(
            GridElem::new(cells)
                .with_columns(TrackSizings(smallvec![Sizing::Auto; 2]))
                .with_column_gutter(TrackSizings(smallvec![COLUMN_GUTTER.into()]))
                .with_row_gutter(TrackSizings(smallvec![row_gutter.into()]))
                .pack()
                .spanned(span),
        );
    } else {
        for (_, reference, loc) in references {
            let realized = reference.clone().located(*loc);
            let block = if works.hanging_indent {
                let body = HElem::new((-INDENT).into()).pack() + realized;
                let inset = Sides::default()
                    .with(styles.resolve(TextElem::dir).start(), Some(INDENT.into()));
                BlockElem::new()
                    .with_body(Some(BlockBody::Content(body)))
                    .with_inset(inset)
            } else {
                BlockElem::new().with_body(Some(BlockBody::Content(realized)))
            };

            seq.push(block.pack().spanned(span));
        }
    }

    Ok(Content::sequence(seq))
};

const CSL_LIGHT_RULE: ShowFn<CslLightElem> =
    |elem, _, _| Ok(elem.body.clone().set(TextElem::delta, WeightDelta(-100)));

const CSL_INDENT_RULE: ShowFn<CslIndentElem> =
    |elem, _, _| Ok(PadElem::new(elem.body.clone()).pack());

const TABLE_RULE: ShowFn<TableElem> = |elem, _, _| {
    Ok(BlockElem::multi_layouter(elem.clone(), crate::grid::layout_table).pack())
};

const TABLE_CELL_RULE: ShowFn<TableCell> = |elem, _, styles| {
    show_cell(elem.body.clone(), elem.inset.get(styles), elem.align.get(styles))
};

const SUB_RULE: ShowFn<SubElem> = |elem, _, styles| {
    show_script(
        styles,
        elem.body.clone(),
        elem.typographic.get(styles),
        elem.baseline.get(styles),
        elem.size.get(styles),
        ScriptKind::Sub,
    )
};

const SUPER_RULE: ShowFn<SuperElem> = |elem, _, styles| {
    show_script(
        styles,
        elem.body.clone(),
        elem.typographic.get(styles),
        elem.baseline.get(styles),
        elem.size.get(styles),
        ScriptKind::Super,
    )
};

fn show_script(
    styles: StyleChain,
    body: Content,
    typographic: bool,
    baseline: Smart<Length>,
    size: Smart<TextSize>,
    kind: ScriptKind,
) -> SourceResult<Content> {
    let font_size = styles.resolve(TextElem::size);
    Ok(body.set(
        TextElem::shift_settings,
        Some(ShiftSettings {
            typographic,
            shift: baseline.map(|l| -Em::from_length(l, font_size)),
            size: size.map(|t| Em::from_length(t.0, font_size)),
            kind,
        }),
    ))
}

const UNDERLINE_RULE: ShowFn<UnderlineElem> = |elem, _, styles| {
    Ok(elem.body.clone().set(
        TextElem::deco,
        smallvec![Decoration {
            line: DecoLine::Underline {
                stroke: elem.stroke.resolve(styles).unwrap_or_default(),
                offset: elem.offset.resolve(styles),
                evade: elem.evade.get(styles),
                background: elem.background.get(styles),
            },
            extent: elem.extent.resolve(styles),
        }],
    ))
};

const OVERLINE_RULE: ShowFn<OverlineElem> = |elem, _, styles| {
    Ok(elem.body.clone().set(
        TextElem::deco,
        smallvec![Decoration {
            line: DecoLine::Overline {
                stroke: elem.stroke.resolve(styles).unwrap_or_default(),
                offset: elem.offset.resolve(styles),
                evade: elem.evade.get(styles),
                background: elem.background.get(styles),
            },
            extent: elem.extent.resolve(styles),
        }],
    ))
};

const STRIKE_RULE: ShowFn<StrikeElem> = |elem, _, styles| {
    Ok(elem.body.clone().set(
        TextElem::deco,
        smallvec![Decoration {
            // Note that we do not support evade option for strikethrough.
            line: DecoLine::Strikethrough {
                stroke: elem.stroke.resolve(styles).unwrap_or_default(),
                offset: elem.offset.resolve(styles),
                background: elem.background.get(styles),
            },
            extent: elem.extent.resolve(styles),
        }],
    ))
};

const HIGHLIGHT_RULE: ShowFn<HighlightElem> = |elem, _, styles| {
    Ok(elem.body.clone().set(
        TextElem::deco,
        smallvec![Decoration {
            line: DecoLine::Highlight {
                fill: elem.fill.get_cloned(styles),
                stroke: elem
                    .stroke
                    .resolve(styles)
                    .unwrap_or_default()
                    .map(|stroke| stroke.map(Stroke::unwrap_or_default)),
                top_edge: elem.top_edge.get(styles),
                bottom_edge: elem.bottom_edge.get(styles),
                radius: elem.radius.resolve(styles).unwrap_or_default(),
            },
            extent: elem.extent.resolve(styles),
        }],
    ))
};

const SMALLCAPS_RULE: ShowFn<SmallcapsElem> = |elem, _, styles| {
    let sc = if elem.all.get(styles) { Smallcaps::All } else { Smallcaps::Minuscules };
    Ok(elem.body.clone().set(TextElem::smallcaps, Some(sc)))
};

const RAW_RULE: ShowFn<RawElem> = |elem, _, styles| {
    let lines = elem.lines.as_deref().unwrap_or_default();

    let mut seq = EcoVec::with_capacity((2 * lines.len()).saturating_sub(1));
    for (i, line) in lines.iter().enumerate() {
        if i != 0 {
            seq.push(LinebreakElem::shared().clone());
        }

        seq.push(line.clone().pack());
    }

    let mut realized = Content::sequence(seq);

    if elem.block.get(styles) {
        // Align the text before inserting it into the block.
        realized = realized.aligned(elem.align.get(styles).into());
        realized = BlockElem::new()
            .with_body(Some(BlockBody::Content(realized)))
            .pack()
            .spanned(elem.span());
    }

    Ok(realized)
};

const RAW_LINE_RULE: ShowFn<RawLine> = |elem, _, _| Ok(elem.body.clone());

const ALIGN_RULE: ShowFn<AlignElem> =
    |elem, _, styles| Ok(elem.body.clone().aligned(elem.alignment.get(styles)));

const PAD_RULE: ShowFn<PadElem> = |elem, _, _| {
    Ok(BlockElem::multi_layouter(elem.clone(), crate::pad::layout_pad).pack())
};

const COLUMNS_RULE: ShowFn<ColumnsElem> = |elem, _, _| {
    Ok(BlockElem::multi_layouter(elem.clone(), crate::flow::layout_columns).pack())
};

const STACK_RULE: ShowFn<StackElem> = |elem, _, _| {
    Ok(BlockElem::multi_layouter(elem.clone(), crate::stack::layout_stack).pack())
};

const GRID_RULE: ShowFn<GridElem> = |elem, _, _| {
    Ok(BlockElem::multi_layouter(elem.clone(), crate::grid::layout_grid).pack())
};

const GRID_CELL_RULE: ShowFn<GridCell> = |elem, _, styles| {
    show_cell(elem.body.clone(), elem.inset.get(styles), elem.align.get(styles))
};

/// Function with common code to display a grid cell or table cell.
fn show_cell(
    mut body: Content,
    inset: Smart<Sides<Option<Rel<Length>>>>,
    align: Smart<Alignment>,
) -> SourceResult<Content> {
    let inset = inset.unwrap_or_default().map(Option::unwrap_or_default);

    if inset != Sides::default() {
        // Only pad if some inset is not 0pt.
        // Avoids a bug where using .padded() in any way inside Show causes
        // alignment in align(...) to break.
        body = body.padded(inset);
    }

    if let Smart::Custom(alignment) = align {
        body = body.aligned(alignment);
    }

    Ok(body)
}

const MOVE_RULE: ShowFn<MoveElem> = |elem, _, _| {
    Ok(BlockElem::single_layouter(elem.clone(), crate::transforms::layout_move).pack())
};

const SCALE_RULE: ShowFn<ScaleElem> = |elem, _, _| {
    Ok(BlockElem::single_layouter(elem.clone(), crate::transforms::layout_scale).pack())
};

const ROTATE_RULE: ShowFn<RotateElem> = |elem, _, _| {
    Ok(BlockElem::single_layouter(elem.clone(), crate::transforms::layout_rotate).pack())
};

const SKEW_RULE: ShowFn<SkewElem> = |elem, _, _| {
    Ok(BlockElem::single_layouter(elem.clone(), crate::transforms::layout_skew).pack())
};

const REPEAT_RULE: ShowFn<RepeatElem> = |elem, _, _| {
    Ok(BlockElem::single_layouter(elem.clone(), crate::repeat::layout_repeat).pack())
};

const HIDE_RULE: ShowFn<HideElem> =
    |elem, _, _| Ok(elem.body.clone().set(HideElem::hidden, true));

const LAYOUT_RULE: ShowFn<LayoutElem> = |elem, _, _| {
    Ok(BlockElem::multi_layouter(
        elem.clone(),
        |elem, engine, locator, styles, regions| {
            // Gets the current region's base size, which will be the size of the
            // outer container, or of the page if there is no such container.
            let Size { x, y } = regions.base();
            let loc = elem.location().unwrap();
            let context = Context::new(Some(loc), Some(styles));
            let result = elem
                .func
                .call(engine, context.track(), [dict! { "width" => x, "height" => y }])?
                .display();
            crate::flow::layout_fragment(engine, &result, locator, styles, regions)
        },
    )
    .pack())
};

const IMAGE_RULE: ShowFn<ImageElem> = |elem, _, styles| {
    Ok(BlockElem::single_layouter(elem.clone(), crate::image::layout_image)
        .with_width(elem.width.get(styles))
        .with_height(elem.height.get(styles))
        .pack())
};

const LINE_RULE: ShowFn<LineElem> = |elem, _, _| {
    Ok(BlockElem::single_layouter(elem.clone(), crate::shapes::layout_line).pack())
};

const RECT_RULE: ShowFn<RectElem> = |elem, _, styles| {
    Ok(BlockElem::single_layouter(elem.clone(), crate::shapes::layout_rect)
        .with_width(elem.width.get(styles))
        .with_height(elem.height.get(styles))
        .pack())
};

const SQUARE_RULE: ShowFn<SquareElem> = |elem, _, styles| {
    Ok(BlockElem::single_layouter(elem.clone(), crate::shapes::layout_square)
        .with_width(elem.width.get(styles))
        .with_height(elem.height.get(styles))
        .pack())
};

const ELLIPSE_RULE: ShowFn<EllipseElem> = |elem, _, styles| {
    Ok(BlockElem::single_layouter(elem.clone(), crate::shapes::layout_ellipse)
        .with_width(elem.width.get(styles))
        .with_height(elem.height.get(styles))
        .pack())
};

const CIRCLE_RULE: ShowFn<CircleElem> = |elem, _, styles| {
    Ok(BlockElem::single_layouter(elem.clone(), crate::shapes::layout_circle)
        .with_width(elem.width.get(styles))
        .with_height(elem.height.get(styles))
        .pack())
};

const POLYGON_RULE: ShowFn<PolygonElem> = |elem, _, _| {
    Ok(BlockElem::single_layouter(elem.clone(), crate::shapes::layout_polygon).pack())
};

const CURVE_RULE: ShowFn<CurveElem> = |elem, _, _| {
    Ok(BlockElem::single_layouter(elem.clone(), crate::shapes::layout_curve).pack())
};

const PATH_RULE: ShowFn<PathElem> = |elem, _, _| {
    Ok(BlockElem::single_layouter(elem.clone(), crate::shapes::layout_path).pack())
};

const EQUATION_RULE: ShowFn<EquationElem> = |elem, _, styles| {
    if elem.block.get(styles) {
        Ok(BlockElem::multi_layouter(elem.clone(), crate::math::layout_equation_block)
            .pack())
    } else {
        Ok(InlineElem::layouter(elem.clone(), crate::math::layout_equation_inline).pack())
    }
};

const ATTACH_RULE: ShowFn<AttachElem> = |_, _, _| Ok(Content::empty());

const ARTIFACT_RULE: ShowFn<ArtifactElem> = |elem, _, _| Ok(elem.body.clone());
