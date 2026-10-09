use comemo::Track;
use ecow::{EcoVec, eco_format};
use smallvec::smallvec;
use typst_library::diag::{At, SourceResult, Trace, Tracepoint, bail, eco_vec, error};
use typst_library::foundations::{
    Content, Context, FromValue, IntoValue, NativeElement, NativeRuleMap, Packed,
    Resolve, ShowFn, Smart, StyleChain, SymbolElem, Synthesize, Target, Value, dict,
};
use typst_library::introspection::{Counter, Locator, LocatorLink};
use typst_library::layout::{
    Abs, AlignElem, Alignment, Axes, BaselinePos, BlockBody, BlockElem, BoxElem,
    ColumnsElem, Em, FixedAlignment, GridCell, GridChild, GridElem, GridItem, HAlignment,
    HElem, HideElem, InlineElem, LayoutElem, Length, MoveElem, OuterVAlignment, PadElem,
    PageElem, PlaceElem, PlacementScope, Ratio, Region, Rel, RepeatElem, RotateElem,
    ScaleElem, Sides, Size, Sizing, SkewElem, Spacing, StackChild, StackElem,
    TrackSizings, VAlignment, VElem,
};
use typst_library::math::EquationElem;
use typst_library::model::{
    ArtifactElem, ArtifactKind, ButtonAction, CheckboxField, ChoiceField,
    FieldAppearance, FieldAppearanceKind, Form, FormButtonField, FormCheckboxField,
    FormChoiceField, FormElem, FormField, FormFieldKind, FormFieldMarker, FormLabel,
    FormRadioField, FormRadioGroup, FormTextField, PdfMarkerTag, RadioField, RadioGroup,
    TextField, WidgetAction,
};
use typst_library::model::{
    Attribution, BibliographyElem, CiteElem, CiteGroup, CslIndentElem, CslLightElem,
    Destination, DirectLinkElem, DividerElem, EmphElem, EnumElem, FigureCaption,
    FigureElem, FootnoteElem, FootnoteEntry, HeadingElem, LinkElem, LinkMarker, ListElem,
    OutlineElem, OutlineEntry, ParElem, ParbreakElem, QuoteElem, RefElem, StrongElem,
    TableCell, TableElem, TermsElem, TitleElem, Works,
};
use typst_library::text::{
    DecoLine, Decoration, HighlightElem, ItalicToggle, LinebreakElem, LocalName,
    OverlineElem, RawElem, RawLine, ScriptKind, ShiftSettings, Smallcaps, SmallcapsElem,
    SmartQuoteElem, SmartQuotes, SpaceElem, StrikeElem, SubElem, SuperElem, TextElem,
    TextSize, UnderlineElem, WeightDelta,
};
use typst_library::visualize::{
    CircleElem, Color, CurveElem, EllipseElem, ImageElem, LineElem, PolygonElem,
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
    rules.register(Paged, DIVIDER_RULE);
    rules.register(Paged, TITLE_RULE);
    rules.register(Paged, HEADING_RULE);
    rules.register(Paged, FIGURE_RULE);
    rules.register(Paged, FIGURE_CAPTION_RULE);
    rules.register(Paged, QUOTE_RULE);
    rules.register(Paged, FOOTNOTE_RULE);
    rules.register(Paged, FOOTNOTE_ENTRY_RULE);
    rules.register(Paged, OUTLINE_RULE);
    rules.register(Paged, OUTLINE_ENTRY_RULE);
    rules.register(Paged, REF_RULE);
    rules.register(Paged, CITE_GROUP_RULE);
    rules.register(Paged, BIBLIOGRAPHY_RULE);
    rules.register(Paged, CSL_LIGHT_RULE);
    rules.register(Paged, CSL_INDENT_RULE);
    rules.register(Paged, TABLE_RULE);
    rules.register(Paged, TABLE_CELL_RULE);
    rules.register(Paged, FORM_RULE);
    rules.register(Paged, FORM_FIELD_MARKER_RULE);
    rules.register(Paged, FORM_BUTTON_FIELD_RULE);
    rules.register(Paged, FORM_CHECKBOX_FIELD_RULE);
    rules.register(Paged, FORM_RADIO_GROUP_RULE);
    rules.register(Paged, FORM_RADIO_FIELD_RULE);
    rules.register(Paged, FORM_TEXT_FIELD_RULE);
    rules.register(Paged, FORM_CHOICE_FIELD_RULE);
    rules.register(Paged, FORM_LABEL_RULE);

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

    // Math.
    rules.register(Paged, EQUATION_RULE);

    // PDF.
    rules.register(Paged, ARTIFACT_RULE);
    rules.register(Paged, PDF_MARKER_TAG_RULE);
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
    for child in &elem.children {
        let mut seq = vec![];
        seq.extend(unpad.clone());
        seq.push(PdfMarkerTag::TermsItemLabel(child.term.clone().strong()));
        seq.push(separator.clone().artifact(ArtifactKind::Other));
        seq.push(child.description.clone());

        // Text in wide term lists shall always turn into paragraphs.
        if !tight {
            seq.push(ParbreakElem::shared().clone());
        }

        let item = Content::sequence(seq).spanned(child.span());
        children.push(StackChild::Block(PdfMarkerTag::TermsItemBody(item)));
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
    let span = elem.span();
    let body = elem.body.clone();
    let dest = elem.dest.resolve_early(engine, span)?;
    let alt = dest.alt_text(engine, styles, span)?;
    // Manually construct link marker that spans the whole link elem, not just
    // the body.
    Ok(LinkMarker::new(body, Some(alt))
        .pack()
        .spanned(span)
        .set(LinkElem::current, Some(dest)))
};

const DIRECT_LINK_RULE: ShowFn<DirectLinkElem> = |elem, _, _| {
    let dest = Destination::Location(elem.loc);
    Ok(elem.body.clone().linked(dest, elem.alt.clone()))
};

const DIVIDER_RULE: ShowFn<DividerElem> =
    |elem, _, _| Ok(LineElem::new().pack().spanned(elem.span()));

const TITLE_RULE: ShowFn<TitleElem> =
    |elem, _, styles| Ok(BlockElem::packed(elem.resolve_body(styles).at(elem.span())?));

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
            .display_at(engine, location, styles, numbering, span)?
            .spanned(span);
        let align = styles.resolve(AlignElem::alignment);

        if hanging_indent.is_auto() && align.x == FixedAlignment::Start {
            let pod = Region::new(Axes::splat(Abs::inf()), Axes::splat(false));

            // Add weak spacing (that will collapse) on both sides to disable CJ
            // punctuation adjustment during measurement. In the actual heading
            // the numbering will not be at the boundary (negative spacing
            // before and the body after), so there could be misalignment
            // otherwise. If there is ever a cleaner way to disable punctuation
            // adjustment, that would be preferrable.
            let collapsing = HElem::new(Abs::pt(1.0).into()).with_weak(true).pack();
            let measurable = collapsing.clone() + numbering.clone() + collapsing;

            // We don't have a locator for the numbering here, so we just
            // use the measurement infrastructure for now.
            let link = LocatorLink::measure(location, span);
            let size = (engine.library.routines.layout_frame)(
                engine,
                &measurable,
                Locator::link(&link),
                styles,
                pod,
            )?
            .size();

            indent = size.x + SPACING_TO_NUMBERING.resolve(styles);
        }

        // The spacing is weak to eat up a potential leading space in the body.
        let spacing = HElem::new(SPACING_TO_NUMBERING.into()).with_weak(true).pack();

        realized = numbering + spacing + realized;
    }

    Ok(if indent != Abs::zero() {
        let body = HElem::new((-indent).into()).pack() + realized;
        let inset = Sides::default()
            .with(styles.resolve(TextElem::dir).start(), Some(indent.into()));
        BlockElem::new()
            .with_inset(inset)
            .with_body(Some(BlockBody::Content(body)))
            .pack()
    } else {
        BlockElem::packed(realized)
    })
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
    realized = BlockElem::packed(realized).spanned(span);

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
            hint: "you can enable floating placement with `figure(placement: auto, ..)`";
        );
    }

    Ok(realized)
};

const FIGURE_CAPTION_RULE: ShowFn<FigureCaption> =
    |elem, engine, styles| Ok(BlockElem::packed(elem.realize(engine, styles)?));

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
        realized = BlockElem::packed(realized).spanned(span);

        if let Some(attribution) = attribution.as_ref() {
            // Bring the attribution a bit closer to the quote.
            let gap = Spacing::Rel(Em::new(0.9).into());
            let v = VElem::new(gap).with_weak(true).pack();
            realized += v;
            realized +=
                BlockElem::packed(attribution.realize(span)).aligned(Alignment::END);
        }

        realized = PadElem::new(realized).pack();
    } else if let Some(Attribution::Label(label)) = attribution {
        realized += SpaceElem::shared().clone();
        realized += CiteElem::new(*label).pack().spanned(span);
    }

    Ok(realized)
};

const FOOTNOTE_RULE: ShowFn<FootnoteElem> = |elem, engine, styles| {
    // The footnote number that links to the footnote entry.
    let link = elem.realize(engine, styles)?;
    let sup = SuperElem::new(link).pack().spanned(elem.span());
    Ok(HElem::hole().clone() + PdfMarkerTag::Label(sup))
};

const FOOTNOTE_ENTRY_RULE: ShowFn<FootnoteEntry> = |elem, engine, styles| {
    let number_gap = Em::new(0.05);
    let (sup, body) = elem.realize(engine, styles)?;
    let prefix = PdfMarkerTag::Label(sup);
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
    let body = PdfMarkerTag::OutlineBody(Content::sequence(entries));
    Ok(Content::sequence(title.into_iter().chain(Some(body))))
};

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
        BlockElem::packed(body).spanned(span)
    } else {
        let point = Tracepoint::process::<OutlineEntry>;
        elem.indented(engine, context, span, prefix, inner, Em::new(0.5).into())
            .trace(engine.world, point, span)?
    };

    let loc = elem.element_location().at(span)?;
    Ok(block.linked(Destination::Location(loc), Some(alt)))
};

const REF_RULE: ShowFn<RefElem> = |elem, engine, styles| elem.realize(engine, styles);

const CITE_GROUP_RULE: ShowFn<CiteGroup> = |elem, engine, _| elem.realize(engine);

const BIBLIOGRAPHY_RULE: ShowFn<BibliographyElem> = |elem, engine, styles| {
    const COLUMN_GUTTER: Em = Em::new(0.65);

    let loc = elem.location().unwrap();
    let span = elem.span();

    let mut seq = vec![];
    seq.extend(elem.realize_title(styles));

    let works = Works::generate(engine, elem.span())?;
    let bibliography = works.bibliography(loc, span)?;

    if bibliography.entries.iter().any(|entry| entry.prefix.is_some()) {
        let row_gutter = styles.get(ParElem::spacing);

        let mut cells = vec![];
        for entry in &bibliography.entries {
            let prefix = PdfMarkerTag::ListItemLabel(
                entry.prefix.clone().unwrap_or_default().located(entry.backlink),
            );
            cells.push(GridChild::Item(GridItem::Cell(
                Packed::new(GridCell::new(prefix)).spanned(span),
            )));

            let reference = PdfMarkerTag::BibEntry(entry.body.clone());
            cells.push(GridChild::Item(GridItem::Cell(
                Packed::new(GridCell::new(reference)).spanned(span),
            )));
        }

        let grid = GridElem::new(cells)
            .with_columns(TrackSizings(smallvec![Sizing::Auto; 2]))
            .with_column_gutter(TrackSizings(smallvec![COLUMN_GUTTER.into()]))
            .with_row_gutter(TrackSizings(smallvec![row_gutter.into()]));
        let mut packed = Packed::new(grid).spanned(span);
        packed.synthesize(engine, styles)?;
        // Directly build the block element to avoid the show step for the grid
        // element. This will not generate introspection tags for the element.
        let block = BlockElem::multi_layouter(packed, crate::grid::layout_grid).pack();

        // TODO(accessibility): infer list numbering from style?
        seq.push(PdfMarkerTag::Bibliography(true, block));
    } else {
        let mut body = vec![];
        for entry in &bibliography.entries {
            let realized =
                PdfMarkerTag::BibEntry(entry.body.clone().located(entry.backlink));
            let block = if bibliography.hanging_indent {
                let indent = styles.get(ParElem::hanging_indent);
                let body = HElem::new((-indent).into()).pack() + realized;
                let inset = Sides::default()
                    .with(styles.resolve(TextElem::dir).start(), Some(indent.into()));
                BlockElem::new()
                    .with_inset(inset)
                    .with_body(Some(BlockBody::Content(body)))
                    .pack()
            } else {
                BlockElem::packed(realized)
            };

            body.push(block.spanned(span));
        }
        seq.push(PdfMarkerTag::Bibliography(false, Content::sequence(body)));
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

const FORM_RULE: ShowFn<FormElem> = |elem, _, styles| {
    Ok(elem.body.clone().set(
        FormElem::form,
        Some(Form {
            location: elem.location().unwrap(),
            target: elem.target.get_cloned(styles),
        }),
    ))
};

const FORM_FIELD_MARKER_RULE: ShowFn<FormFieldMarker> =
    |elem, _, _| Ok(elem.body.clone());

const FORM_BUTTON_FIELD_RULE: ShowFn<FormButtonField> = |elem, _, styles| {
    // TODO: this will probably be changed, but used as a PoC for now
    let span = elem.span();
    let location = elem.location().unwrap();
    let name = elem.name.get_cloned(styles).custom();
    let Some(form) = styles.get_ref(FormElem::form) else {
        bail!(span, "button must be placed inside a #form element");
    };

    let action = elem.action.get(styles).map(|action| {
        match action {
            ButtonAction::Submit => {
                let Some(target) = form.target.clone() else {
                    bail!(
                        span, "button has submit action, but no submission target is defined for its form";
                        hint: "set the `target` property of the surrounding #form element";
                    );
                };
                Ok(WidgetAction::Submit { form: form.location, target })
            },
            ButtonAction::Reset => Ok(WidgetAction::Reset { form: form.location }),
        }
    }).transpose()?;

    // TODO: maybe get rid of the BoxElem and just use elem.body?
    // however, what do we do with elem.width/elem.height then?
    // FIXME: we have clipping issues, as box does not expand to accommodate the correct text height
    let inner = BoxElem::new()
        .with_width(elem.width.get(styles))
        .with_height(elem.height.get(styles))
        .with_clip(true) // PDF XObjects are clipped, so ensure the same for png/svg
        .with_baseline(
            // TODO: impl From<...> for BaselinePos
            BaselinePos::from_value(Rel::from(Length::from(Em::new(0.2))).into_value())
                .unwrap(),
        )
        .with_body(Some(
            elem.body.clone().set(
                FormElem::appearance,
                Some(
                    FieldAppearance::new(location, FieldAppearanceKind::Single)
                        .with_action(action),
                ),
            ),
        ))
        .pack()
        .spanned(span);

    Ok(FormFieldMarker::new(inner).pack().spanned(span).set(
        FormElem::field,
        Some(FormField::new(span, location, form.location, name, FormFieldKind::Button)),
    ))
};

const FORM_CHECKBOX_FIELD_RULE: ShowFn<FormCheckboxField> = |elem, _, styles| {
    // TODO: this will probably be changed, but used as a PoC for now
    let span = elem.span();
    let location = elem.location().unwrap();
    let name = elem.name.get_cloned(styles).custom();
    let Some(form) = styles.get_ref(FormElem::form) else {
        bail!(span, "checkbox must be placed inside a #form element");
    };
    let checked = elem.checked.get(styles);

    let inner_width = Smart::Custom(Ratio::one().into());
    let inner_height = Sizing::Rel(Ratio::one().into());

    let width = elem.width.get(styles);
    let width = if width.is_auto() { Sizing::Rel(Em::one().into()) } else { width };
    let height = elem.height.get(styles).or(Smart::Custom(Em::one().into()));

    let checkmark = AlignElem::new(SymbolElem::packed("✓"))
        .with_alignment(HAlignment::Center + VAlignment::Horizon)
        .pack();

    let children = [
        PlaceElem::new(
            RectElem::new()
                .with_width(inner_width)
                .with_height(inner_height)
                .with_outset(Sides::splat(Some(Abs::pt(-0.5).into())))
                .with_inset(Sides::splat(Some(Rel::zero())))
                .with_body(Some(checkmark))
                .pack()
                .spanned(span)
                .set(
                    FormElem::appearance,
                    Some(FieldAppearance::new(
                        location,
                        FieldAppearanceKind::On(checked),
                    )),
                ),
        )
        .with_alignment(Smart::Custom(HAlignment::Left + VAlignment::Top))
        .pack()
        .spanned(span),
        RectElem::new()
            .with_width(inner_width)
            .with_height(inner_height)
            .with_outset(Sides::splat(Some(Abs::pt(-0.5).into())))
            .pack()
            .spanned(span)
            .set(
                FormElem::appearance,
                Some(FieldAppearance::new(location, FieldAppearanceKind::Off(!checked))),
            ),
    ];
    let states = Content::sequence(children).spanned(span);

    let inner = BoxElem::new()
        .with_width(width)
        .with_height(height)
        .with_clip(true) // PDF XObjects are clipped, so ensure the same for png/svg
        .with_baseline(
            // TODO: impl From<...> for BaselinePos
            BaselinePos::from_value(Rel::from(Length::from(Em::new(0.2))).into_value())
                .unwrap(),
        )
        .with_body(Some(states))
        .pack()
        .spanned(span);

    Ok(FormFieldMarker::new(inner).pack().spanned(span).set(
        FormElem::field,
        Some(FormField::new(
            span,
            location,
            form.location,
            name,
            CheckboxField {
                checked,
                required: elem.required.get(styles),
                read_only: elem.read_only.get(styles),
            },
        )),
    ))
};

const FORM_RADIO_GROUP_RULE: ShowFn<FormRadioGroup> = |elem, _, styles| {
    // TODO: this will probably be changed, but used as a PoC for now
    let span = elem.span();
    let location = elem.location().unwrap();
    let name = elem.name.get_cloned(styles).custom();
    let Some(form) = styles.get_ref(FormElem::form) else {
        bail!(span, "radio group must be placed inside a #form element");
    };
    let selected = elem.selected.get_cloned(styles);
    let required = elem.required.get(styles);

    Ok(FormFieldMarker::new(elem.body.clone())
        .pack()
        .spanned(span)
        .set(
            FormElem::field,
            Some(FormField::new(
                span,
                location,
                form.location,
                name,
                RadioField {
                    selected: selected.clone(),
                    required,
                    read_only: elem.read_only.get(styles),
                },
            )),
        )
        .set(
            FormRadioGroup::radio_group,
            Some(RadioGroup {
                location,
                name: Smart::Auto, // FIXME: this is not used in paged export
                selected,
                required,
            }),
        ))
};

const FORM_RADIO_FIELD_RULE: ShowFn<FormRadioField> = |elem, _, styles| {
    // TODO: this will probably be changed, but used as a PoC for now
    let span = elem.span();
    let location = elem.location().unwrap();
    let Some(group) = styles.get_ref(FormRadioGroup::radio_group) else {
        bail!(
            span, "a radio button must appear inside a radio group";
            hint: "try surrounding this button with #form.radio-group[...]";
        )
    };
    let value = &elem.value;
    let selected = group.selected.as_ref().is_some_and(|s| s == value);

    let inner_width = Smart::Custom(Ratio::one().into());
    let inner_height = Sizing::Rel(Ratio::one().into());

    let width = elem.width.get(styles);
    let width = if width.is_auto() { Sizing::Rel(Em::one().into()) } else { width };
    let height = elem.height.get(styles).or(Smart::Custom(Em::one().into()));

    let inner_circle = AlignElem::new(
        CircleElem::new()
            .with_width(inner_width)
            .with_height(inner_height)
            .with_fill(Some(Color::BLACK.into()))
            .pack(),
    )
    .with_alignment(HAlignment::Center + VAlignment::Horizon)
    .pack();

    let children = [
        PlaceElem::new(
            CircleElem::new()
                .with_width(inner_width)
                .with_height(inner_height)
                .with_outset(Sides::splat(Some(Abs::pt(-0.5).into())))
                .with_inset(Sides::splat(Some(Rel::zero())))
                .with_body(Some(inner_circle))
                .pack()
                .spanned(span)
                .set(
                    FormElem::appearance,
                    Some(
                        FieldAppearance::new(
                            group.location,
                            FieldAppearanceKind::On(selected),
                        )
                        .with_widget_location(location)
                        .with_value(Some(value.clone())),
                    ),
                ),
        )
        .with_alignment(Smart::Custom(HAlignment::Left + VAlignment::Top))
        .pack()
        .spanned(span),
        CircleElem::new()
            .with_width(inner_width)
            .with_height(inner_height)
            .with_outset(Sides::splat(Some(Abs::pt(-0.5).into())))
            .pack()
            .spanned(span)
            .set(
                FormElem::appearance,
                Some(
                    FieldAppearance::new(
                        group.location,
                        FieldAppearanceKind::Off(!selected),
                    )
                    .with_widget_location(location)
                    .with_value(Some(value.clone())),
                ),
            ),
    ];
    let states = Content::sequence(children).spanned(span);

    Ok(BoxElem::new()
        .with_width(width)
        .with_height(height)
        .with_clip(true) // PDF XObjects are clipped, so ensure the same for png/svg
        .with_baseline(
            // TODO: impl From<...> for BaselinePos
            BaselinePos::from_value(Rel::from(Length::from(Em::new(0.2))).into_value())
                .unwrap(),
        )
        .with_body(Some(states))
        .pack()
        .spanned(span))
};

const FORM_TEXT_FIELD_RULE: ShowFn<FormTextField> = |elem, _, styles| {
    // TODO: this will probably be changed, but used as a PoC for now
    let span = elem.span();
    let location = elem.location().unwrap();
    let name = elem.name.get_cloned(styles).custom();
    let Some(form) = styles.get_ref(FormElem::form) else {
        bail!(span, "textbox must be placed inside a #form element");
    };

    let inner_width = Smart::Custom(Ratio::one().into());
    let inner_height = Sizing::Rel(Ratio::one().into());

    let width = elem.width.get(styles);
    let width = if width.is_auto() { Sizing::Rel(Em::new(10.0).into()) } else { width };
    let height = elem.height.get(styles).or(Smart::Custom(Em::one().into()));

    let children = [
        PlaceElem::new(
            BlockElem::new()
                .with_width(inner_width)
                .with_height(inner_height)
                .with_outset(Sides::splat(Some(Abs::pt(-0.5).into())))
                .with_inset(Sides::splat(Some(Em::new(0.1).into())))
                .with_body(
                    elem.value
                        .get_cloned(styles)
                        .map(|v| BlockBody::Content(TextElem::new(v).pack())),
                )
                .pack()
                .spanned(span)
                .set(
                    FormElem::appearance,
                    Some(FieldAppearance::new(
                        location,
                        FieldAppearanceKind::VariableText,
                    )),
                ),
        )
        .with_alignment(Smart::Custom(HAlignment::Left + VAlignment::Top))
        .pack()
        .spanned(span),
        RectElem::new()
            .with_width(inner_width)
            .with_height(inner_height)
            .with_outset(Sides::splat(Some(Abs::pt(-0.5).into())))
            .pack()
            .spanned(span),
    ];
    let states = Content::sequence(children).spanned(span);

    let inner = BoxElem::new()
        .with_width(width)
        .with_height(height)
        .with_clip(true) // PDF XObjects are clipped, so ensure the same for png/svg
        .with_baseline(
            // TODO: impl From<...> for BaselinePos
            BaselinePos::from_value(Rel::from(Length::from(Em::new(0.2))).into_value())
                .unwrap(),
        )
        .with_body(Some(states))
        .pack()
        .spanned(span);

    Ok(FormFieldMarker::new(inner).pack().spanned(span).set(
        FormElem::field,
        Some(FormField::new(
            span,
            location,
            form.location,
            name,
            TextField {
                value: elem.value.get_cloned(styles),
                multiline: elem.multiline.get(styles),
                required: elem.required.get(styles),
                max_length: elem.max_length.get(styles),
                read_only: elem.read_only.get(styles),
                spellcheck: elem.spellcheck.get(styles).unwrap_or(true),
            },
        )),
    ))
};

const FORM_CHOICE_FIELD_RULE: ShowFn<FormChoiceField> = |elem, _, styles| {
    // TODO: this will probably be changed, but used as a PoC for now
    let span = elem.span();
    let location = elem.location().unwrap();
    let name = elem.name.get_cloned(styles).custom();
    let Some(form) = styles.get_ref(FormElem::form) else {
        bail!(span, "choice field must be placed inside a #form element");
    };
    let options = elem.options.get_ref(styles);
    let value = elem.value.get_ref(styles);
    let multiple = elem.multiple.get(styles);

    let inner_width = Smart::Custom(Ratio::one().into());

    let width = elem.width.get(styles);
    let width = if width.is_auto() { Sizing::Rel(Em::new(10.0).into()) } else { width };
    let height = elem
        .height
        .get(styles)
        .or(Smart::Custom(Em::new(if multiple { 5.0 } else { 1.0 }).into()));

    let box_body = if multiple {
        let children = options.0.iter().map(|option| {
            let name = option.display_or_mapping_name();
            let fill =
                value.0.contains(&option.mapping_name).then_some(Color::BLUE.into());

            BlockElem::new()
                .with_width(inner_width)
                .with_breakable(false)
                .with_inset(Sides::splat(Some(Em::new(0.1).into())))
                .with_fill(fill)
                .with_body(Some(BlockBody::Content(TextElem::new(name).pack())))
                .pack()
        });

        StackElem::new(children.map(StackChild::Block).collect()).pack()
    } else {
        if value.0.len() > 1 {
            bail!(
                span, "single choice field has more than one selected option";
                hint: "set `multiple: true` to allow multiple options to be selected";
            )
        }
        let value = value
            .0
            .first()
            .map(|value| {
                options
                    .0
                    .iter()
                    .find_map(|option| {
                        (&option.mapping_name == value)
                            .then(|| option.display_or_mapping_name())
                    })
                    .ok_or(eco_vec![error!(
                        span,
                        "value of this choice field is not present in its options"
                    )])
            })
            .unwrap_or_else(|| {
                Ok(options
                    .0
                    .first()
                    .map(|option| option.display_or_mapping_name())
                    .unwrap_or_default())
            })?;

        BlockElem::new()
            .with_width(inner_width)
            .with_breakable(false)
            .with_inset(Sides::splat(Some(Em::new(0.1).into())))
            .with_body(Some(BlockBody::Content(TextElem::new(value).pack())))
            .pack()
    }
    .spanned(span)
    .set(
        FormElem::appearance,
        Some(FieldAppearance::new(location, FieldAppearanceKind::VariableText)),
    );

    let inner = BoxElem::new()
        .with_width(width)
        .with_height(height)
        .with_clip(true) // PDF XObjects are clipped, so ensure the same for png/svg
        .with_baseline(
            // TODO: impl From<...> for BaselinePos
            BaselinePos::from_value(Rel::from(Length::from(Em::new(0.2))).into_value())
                .unwrap(),
        )
        .with_body(Some(box_body))
        .pack()
        .spanned(span);

    Ok(FormFieldMarker::new(inner).pack().spanned(span).set(
        FormElem::field,
        Some(FormField::new(
            span,
            location,
            form.location,
            name,
            ChoiceField {
                value: value.0.clone(),
                options: options.0.clone(),
                multiple,
                required: elem.required.get(styles),
                read_only: elem.read_only.get(styles),
            },
        )),
    ))
};

const FORM_LABEL_RULE: ShowFn<FormLabel> = |elem, _, _| Ok(elem.body.clone());

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
        realized = BlockElem::packed(realized).spanned(elem.span());
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
            let size = dict! { "width" => x, "height" => y };
            let result = elem
                .func
                .call::<Value>(engine, context.track(), [size], elem.span())?
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

const EQUATION_RULE: ShowFn<EquationElem> = |elem, _, styles| {
    if elem.block.get(styles) {
        Ok(BlockElem::multi_layouter(elem.clone(), crate::math::layout_equation_block)
            .pack())
    } else {
        Ok(InlineElem::layouter(elem.clone(), crate::math::layout_equation_inline).pack())
    }
};

const ARTIFACT_RULE: ShowFn<ArtifactElem> = |elem, _, _| Ok(elem.body.clone());

const PDF_MARKER_TAG_RULE: ShowFn<PdfMarkerTag> = |elem, _, _| Ok(elem.body.clone());
