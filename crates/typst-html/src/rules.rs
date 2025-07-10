use std::num::NonZeroUsize;

use ecow::{eco_format, EcoVec};
use typst_library::diag::warning;
use typst_library::foundations::{
    Content, NativeElement, NativeRuleMap, ShowFn, Smart, StyleChain, Target,
};
use typst_library::introspection::{Counter, Locator};
use typst_library::layout::resolve::{table_to_cellgrid, Cell, CellGrid, Entry};
use typst_library::layout::{OuterVAlignment, Sizing};
use typst_library::model::{
    Attribution, CiteElem, CiteGroup, Destination, EmphElem, EnumElem, FigureCaption,
    FigureElem, HeadingElem, LinkElem, LinkTarget, ListElem, ParbreakElem, QuoteElem,
    RefElem, StrongElem, TableCell, TableElem, TermsElem,
};
use typst_library::text::{
    HighlightElem, LinebreakElem, OverlineElem, RawElem, RawLine, SpaceElem, StrikeElem,
    SubElem, SuperElem, UnderlineElem,
};
use typst_library::visualize::ImageElem;

use crate::{attr, css, tag, FrameElem, HtmlAttrs, HtmlElem, HtmlTag};

/// Registers show rules for the [HTML target](Target::Html).
pub fn register(rules: &mut NativeRuleMap) {
    use Target::{Html, Paged};

    // Model.
    rules.register(Html, STRONG_RULE);
    rules.register(Html, EMPH_RULE);
    rules.register(Html, LIST_RULE);
    rules.register(Html, ENUM_RULE);
    rules.register(Html, TERMS_RULE);
    rules.register(Html, LINK_RULE);
    rules.register(Html, HEADING_RULE);
    rules.register(Html, FIGURE_RULE);
    rules.register(Html, FIGURE_CAPTION_RULE);
    rules.register(Html, QUOTE_RULE);
    rules.register(Html, REF_RULE);
    rules.register(Html, CITE_GROUP_RULE);
    rules.register(Html, TABLE_RULE);

    // Text.
    rules.register(Html, SUB_RULE);
    rules.register(Html, SUPER_RULE);
    rules.register(Html, UNDERLINE_RULE);
    rules.register(Html, OVERLINE_RULE);
    rules.register(Html, STRIKE_RULE);
    rules.register(Html, HIGHLIGHT_RULE);
    rules.register(Html, RAW_RULE);
    rules.register(Html, RAW_LINE_RULE);

    // Visualize.
    rules.register(Html, IMAGE_RULE);

    // For the HTML target, `html.frame` is a primitive. In the laid-out target,
    // it should be a no-op so that nested frames don't break (things like `show
    // math.equation: html.frame` can result in nested ones).
    rules.register::<FrameElem>(Paged, |elem, _, _| Ok(elem.body.clone()));
}

const STRONG_RULE: ShowFn<StrongElem> = |elem, _, _| {
    Ok(HtmlElem::new(tag::strong)
        .with_body(Some(elem.body.clone()))
        .pack()
        .spanned(elem.span()))
};

const EMPH_RULE: ShowFn<EmphElem> = |elem, _, _| {
    Ok(HtmlElem::new(tag::em)
        .with_body(Some(elem.body.clone()))
        .pack()
        .spanned(elem.span()))
};

const LIST_RULE: ShowFn<ListElem> = |elem, _, styles| {
    Ok(HtmlElem::new(tag::ul)
        .with_body(Some(Content::sequence(elem.children.iter().map(|item| {
            // Text in wide lists shall always turn into paragraphs.
            let mut body = item.body.clone();
            if !elem.tight.get(styles) {
                body += ParbreakElem::shared();
            }
            HtmlElem::new(tag::li)
                .with_body(Some(body))
                .pack()
                .spanned(item.span())
        }))))
        .pack()
        .spanned(elem.span()))
};

const ENUM_RULE: ShowFn<EnumElem> = |elem, _, styles| {
    let mut ol = HtmlElem::new(tag::ol);

    if elem.reversed.get(styles) {
        ol = ol.with_attr(attr::reversed, "reversed");
    }

    if let Some(n) = elem.start.get(styles).custom() {
        ol = ol.with_attr(attr::start, eco_format!("{n}"));
    }

    let body = Content::sequence(elem.children.iter().map(|item| {
        let mut li = HtmlElem::new(tag::li);
        if let Some(nr) = item.number.get(styles) {
            li = li.with_attr(attr::value, eco_format!("{nr}"));
        }
        // Text in wide enums shall always turn into paragraphs.
        let mut body = item.body.clone();
        if !elem.tight.get(styles) {
            body += ParbreakElem::shared();
        }
        li.with_body(Some(body)).pack().spanned(item.span())
    }));

    Ok(ol.with_body(Some(body)).pack().spanned(elem.span()))
};

const TERMS_RULE: ShowFn<TermsElem> = |elem, _, styles| {
    Ok(HtmlElem::new(tag::dl)
        .with_body(Some(Content::sequence(elem.children.iter().flat_map(|item| {
            // Text in wide term lists shall always turn into paragraphs.
            let mut description = item.description.clone();
            if !elem.tight.get(styles) {
                description += ParbreakElem::shared();
            }

            [
                HtmlElem::new(tag::dt)
                    .with_body(Some(item.term.clone()))
                    .pack()
                    .spanned(item.term.span()),
                HtmlElem::new(tag::dd)
                    .with_body(Some(description))
                    .pack()
                    .spanned(item.description.span()),
            ]
        }))))
        .pack())
};

const LINK_RULE: ShowFn<LinkElem> = |elem, engine, _| {
    let body = elem.body.clone();
    Ok(if let LinkTarget::Dest(Destination::Url(url)) = &elem.dest {
        HtmlElem::new(tag::a)
            .with_attr(attr::href, url.clone().into_inner())
            .with_body(Some(body))
            .pack()
            .spanned(elem.span())
    } else {
        engine.sink.warn(warning!(
            elem.span(),
            "non-URL links are not yet supported by HTML export"
        ));
        body
    })
};

const HEADING_RULE: ShowFn<HeadingElem> = |elem, engine, styles| {
    let span = elem.span();

    let mut realized = elem.body.clone();
    if let Some(numbering) = elem.numbering.get_ref(styles).as_ref() {
        let location = elem.location().unwrap();
        let numbering = Counter::of(HeadingElem::ELEM)
            .display_at_loc(engine, location, styles, numbering)?
            .spanned(span);
        realized = numbering + SpaceElem::shared().clone() + realized;
    }

    // HTML's h1 is closer to a title element. There should only be one.
    // Meanwhile, a level 1 Typst heading is a section heading. For this
    // reason, levels are offset by one: A Typst level 1 heading becomes
    // a `<h2>`.
    let level = elem.resolve_level(styles).get();
    Ok(if level >= 6 {
        engine.sink.warn(warning!(
            span,
            "heading of level {} was transformed to \
             <div role=\"heading\" aria-level=\"{}\">, which is not \
             supported by all assistive technology",
            level, level + 1;
            hint: "HTML only supports <h1> to <h6>, not <h{}>", level + 1;
            hint: "you may want to restructure your document so that \
                   it doesn't contain deep headings"
        ));
        HtmlElem::new(tag::div)
            .with_body(Some(realized))
            .with_attr(attr::role, "heading")
            .with_attr(attr::aria_level, eco_format!("{}", level + 1))
            .pack()
            .spanned(span)
    } else {
        let t = [tag::h2, tag::h3, tag::h4, tag::h5, tag::h6][level - 1];
        HtmlElem::new(t).with_body(Some(realized)).pack().spanned(span)
    })
};

const FIGURE_RULE: ShowFn<FigureElem> = |elem, _, styles| {
    let span = elem.span();
    let mut realized = elem.body.clone();

    // Build the caption, if any.
    if let Some(caption) = elem.caption.get_cloned(styles) {
        realized = match caption.position.get(styles) {
            OuterVAlignment::Top => caption.pack() + realized,
            OuterVAlignment::Bottom => realized + caption.pack(),
        };
    }

    // Ensure that the body is considered a paragraph.
    realized += ParbreakElem::shared().clone().spanned(span);

    Ok(HtmlElem::new(tag::figure)
        .with_body(Some(realized))
        .pack()
        .spanned(span))
};

const FIGURE_CAPTION_RULE: ShowFn<FigureCaption> = |elem, engine, styles| {
    Ok(HtmlElem::new(tag::figcaption)
        .with_body(Some(elem.realize(engine, styles)?))
        .pack()
        .spanned(elem.span()))
};

const QUOTE_RULE: ShowFn<QuoteElem> = |elem, _, styles| {
    let span = elem.span();
    let block = elem.block.get(styles);

    let mut realized = elem.body.clone();

    if elem.quotes.get(styles).unwrap_or(!block) {
        realized = QuoteElem::quoted(realized, styles);
    }

    let attribution = elem.attribution.get_ref(styles);

    if block {
        let mut blockquote = HtmlElem::new(tag::blockquote).with_body(Some(realized));
        if let Some(Attribution::Content(attribution)) = attribution {
            if let Some(link) = attribution.to_packed::<LinkElem>() {
                if let LinkTarget::Dest(Destination::Url(url)) = &link.dest {
                    blockquote =
                        blockquote.with_attr(attr::cite, url.clone().into_inner());
                }
            }
        }

        realized = blockquote.pack().spanned(span);

        if let Some(attribution) = attribution.as_ref() {
            realized += attribution.realize(span);
        }
    } else if let Some(Attribution::Label(label)) = attribution {
        realized += SpaceElem::shared().clone();
        realized += CiteElem::new(*label).pack().spanned(span);
    }

    Ok(realized)
};

const REF_RULE: ShowFn<RefElem> = |elem, engine, styles| elem.realize(engine, styles);

const CITE_GROUP_RULE: ShowFn<CiteGroup> = |elem, engine, _| elem.realize(engine);

const TABLE_RULE: ShowFn<TableElem> = |elem, engine, styles| {
    // The locator is not used by HTML export, so we can just fabricate one.
    let locator = Locator::root();
    Ok(show_cellgrid(table_to_cellgrid(elem, engine, locator, styles)?, styles))
};

fn show_cellgrid(grid: CellGrid, styles: StyleChain) -> Content {
    let elem = |tag, body| HtmlElem::new(tag).with_body(Some(body)).pack();
    let mut rows: Vec<_> = grid.entries.chunks(grid.non_gutter_column_count()).collect();

    let tr = |tag, row: &[Entry]| {
        let row = row
            .iter()
            .flat_map(|entry| entry.as_cell())
            .map(|cell| show_cell(tag, cell, styles));
        elem(tag::tr, Content::sequence(row))
    };

    // TODO(subfooters): similarly to headers, take consecutive footers from
    // the end for 'tfoot'.
    let footer = grid.footer.map(|ft| {
        let rows = rows.drain(ft.start..);
        elem(tag::tfoot, Content::sequence(rows.map(|row| tr(tag::td, row))))
    });

    // Store all consecutive headers at the start in 'thead'. All remaining
    // headers are just 'th' rows across the table body.
    let mut consecutive_header_end = 0;
    let first_mid_table_header = grid
        .headers
        .iter()
        .take_while(|hd| {
            let is_consecutive = hd.range.start == consecutive_header_end;
            consecutive_header_end = hd.range.end;
            is_consecutive
        })
        .count();

    let (y_offset, header) = if first_mid_table_header > 0 {
        let removed_header_rows =
            grid.headers.get(first_mid_table_header - 1).unwrap().range.end;
        let rows = rows.drain(..removed_header_rows);

        (
            removed_header_rows,
            Some(elem(tag::thead, Content::sequence(rows.map(|row| tr(tag::th, row))))),
        )
    } else {
        (0, None)
    };

    // TODO: Consider improving accessibility properties of multi-level headers
    // inside tables in the future, e.g. indicating which columns they are
    // relative to and so on. See also:
    // https://www.w3.org/WAI/tutorials/tables/multi-level/
    let mut next_header = first_mid_table_header;
    let mut body =
        Content::sequence(rows.into_iter().enumerate().map(|(relative_y, row)| {
            let y = relative_y + y_offset;
            if let Some(current_header) =
                grid.headers.get(next_header).filter(|h| h.range.contains(&y))
            {
                if y + 1 == current_header.range.end {
                    next_header += 1;
                }

                tr(tag::th, row)
            } else {
                tr(tag::td, row)
            }
        }));

    if header.is_some() || footer.is_some() {
        body = elem(tag::tbody, body);
    }

    let content = header.into_iter().chain(core::iter::once(body)).chain(footer);
    elem(tag::table, Content::sequence(content))
}

fn show_cell(tag: HtmlTag, cell: &Cell, styles: StyleChain) -> Content {
    let cell = cell.body.clone();
    let Some(cell) = cell.to_packed::<TableCell>() else { return cell };
    let mut attrs = HtmlAttrs::new();
    let span = |n: NonZeroUsize| (n != NonZeroUsize::MIN).then(|| n.to_string());
    if let Some(colspan) = span(cell.colspan.get(styles)) {
        attrs.push(attr::colspan, colspan);
    }
    if let Some(rowspan) = span(cell.rowspan.get(styles)) {
        attrs.push(attr::rowspan, rowspan);
    }
    HtmlElem::new(tag)
        .with_body(Some(cell.body.clone()))
        .with_attrs(attrs)
        .pack()
        .spanned(cell.span())
}

const SUB_RULE: ShowFn<SubElem> = |elem, _, _| {
    Ok(HtmlElem::new(tag::sub)
        .with_body(Some(elem.body.clone()))
        .pack()
        .spanned(elem.span()))
};

const SUPER_RULE: ShowFn<SuperElem> = |elem, _, _| {
    Ok(HtmlElem::new(tag::sup)
        .with_body(Some(elem.body.clone()))
        .pack()
        .spanned(elem.span()))
};

const UNDERLINE_RULE: ShowFn<UnderlineElem> = |elem, _, _| {
    // Note: In modern HTML, `<u>` is not the underline element, but
    // rather an "Unarticulated Annotation" element (see HTML spec
    // 4.5.22). Using `text-decoration` instead is recommended by MDN.
    Ok(HtmlElem::new(tag::span)
        .with_attr(attr::style, "text-decoration: underline")
        .with_body(Some(elem.body.clone()))
        .pack())
};

const OVERLINE_RULE: ShowFn<OverlineElem> = |elem, _, _| {
    Ok(HtmlElem::new(tag::span)
        .with_attr(attr::style, "text-decoration: overline")
        .with_body(Some(elem.body.clone()))
        .pack())
};

const STRIKE_RULE: ShowFn<StrikeElem> =
    |elem, _, _| Ok(HtmlElem::new(tag::s).with_body(Some(elem.body.clone())).pack());

const HIGHLIGHT_RULE: ShowFn<HighlightElem> =
    |elem, _, _| Ok(HtmlElem::new(tag::mark).with_body(Some(elem.body.clone())).pack());

const RAW_RULE: ShowFn<RawElem> = |elem, _, styles| {
    let lines = elem.lines.as_deref().unwrap_or_default();

    let mut seq = EcoVec::with_capacity((2 * lines.len()).saturating_sub(1));
    for (i, line) in lines.iter().enumerate() {
        if i != 0 {
            seq.push(LinebreakElem::shared().clone());
        }

        seq.push(line.clone().pack());
    }

    Ok(HtmlElem::new(if elem.block.get(styles) { tag::pre } else { tag::code })
        .with_body(Some(Content::sequence(seq)))
        .pack()
        .spanned(elem.span()))
};

const RAW_LINE_RULE: ShowFn<RawLine> = |elem, _, _| Ok(elem.body.clone());

const IMAGE_RULE: ShowFn<ImageElem> = |elem, engine, styles| {
    let image = elem.decode(engine, styles)?;

    let mut attrs = HtmlAttrs::new();
    attrs.push(attr::src, typst_svg::convert_image_to_base64_url(&image));

    if let Some(alt) = elem.alt.get_cloned(styles) {
        attrs.push(attr::alt, alt);
    }

    let mut inline = css::Properties::new();

    // TODO: Exclude in semantic profile.
    if let Some(value) = typst_svg::convert_image_scaling(image.scaling()) {
        inline.push("image-rendering", value);
    }

    // TODO: Exclude in semantic profile?
    match elem.width.get(styles) {
        Smart::Auto => {}
        Smart::Custom(rel) => inline.push("width", css::rel(rel)),
    }

    // TODO: Exclude in semantic profile?
    match elem.height.get(styles) {
        Sizing::Auto => {}
        Sizing::Rel(rel) => inline.push("height", css::rel(rel)),
        Sizing::Fr(_) => {}
    }

    Ok(HtmlElem::new(tag::img).with_attrs(attrs).with_styles(inline).pack())
};
