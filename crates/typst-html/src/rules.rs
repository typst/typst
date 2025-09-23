use std::num::NonZeroUsize;

use comemo::Track;
use ecow::{EcoVec, eco_format};
use typst_library::diag::{At, bail, warning};
use typst_library::foundations::{
    Content, Context, NativeElement, NativeRuleMap, ShowFn, Smart, StyleChain, Target,
};
use typst_library::introspection::Counter;
use typst_library::layout::resolve::{Cell, CellGrid, Entry, table_to_cellgrid};
use typst_library::layout::{
    BlockBody, BlockElem, BoxElem, HElem, OuterVAlignment, Sizing,
};
use typst_library::model::{
    Attribution, BibliographyElem, CiteElem, CiteGroup, Destination, EmphElem, EnumElem,
    FigureCaption, FigureElem, FootnoteElem, FootnoteEntry, HeadingElem, LinkElem,
    LinkTarget, ListElem, OutlineElem, OutlineEntry, OutlineNode, ParElem, ParbreakElem,
    QuoteElem, RefElem, StrongElem, TableCell, TableElem, TermsElem, TitleElem,
};
use typst_library::text::{
    HighlightElem, LinebreakElem, OverlineElem, RawElem, RawLine, SmallcapsElem,
    SpaceElem, StrikeElem, SubElem, SuperElem, UnderlineElem,
};
use typst_library::visualize::{Color, ImageElem};
use typst_macros::elem;
use typst_utils::singleton;

use crate::{FrameElem, HtmlAttrs, HtmlElem, HtmlTag, attr, css, tag};

/// Registers show rules for the [HTML target](Target::Html).
pub fn register(rules: &mut NativeRuleMap) {
    use Target::{Html, Paged};

    // Model.
    rules.register(Html, PAR_RULE);
    rules.register(Html, STRONG_RULE);
    rules.register(Html, EMPH_RULE);
    rules.register(Html, LIST_RULE);
    rules.register(Html, ENUM_RULE);
    rules.register(Html, TERMS_RULE);
    rules.register(Html, LINK_RULE);
    rules.register(Html, TITLE_RULE);
    rules.register(Html, HEADING_RULE);
    rules.register(Html, FIGURE_RULE);
    rules.register(Html, FIGURE_CAPTION_RULE);
    rules.register(Html, QUOTE_RULE);
    rules.register(Html, FOOTNOTE_RULE);
    rules.register(Html, FOOTNOTE_CONTAINER_RULE);
    rules.register(Html, FOOTNOTE_ENTRY_RULE);
    rules.register(Html, OUTLINE_RULE);
    rules.register(Html, OUTLINE_ENTRY_RULE);
    rules.register(Html, REF_RULE);
    rules.register(Html, CITE_GROUP_RULE);
    rules.register(Html, BIBLIOGRAPHY_RULE);
    rules.register(Html, TABLE_RULE);

    // Text.
    rules.register(Html, SUB_RULE);
    rules.register(Html, SUPER_RULE);
    rules.register(Html, UNDERLINE_RULE);
    rules.register(Html, OVERLINE_RULE);
    rules.register(Html, STRIKE_RULE);
    rules.register(Html, HIGHLIGHT_RULE);
    rules.register(Html, SMALLCAPS_RULE);
    rules.register(Html, RAW_RULE);
    rules.register(Html, RAW_LINE_RULE);

    // Layout.
    rules.register(Html, BLOCK_RULE);
    rules.register(Html, BOX_RULE);

    // Visualize.
    rules.register(Html, IMAGE_RULE);

    // For the HTML target, `html.frame` is a primitive. In the laid-out target,
    // it should be a no-op so that nested frames don't break (things like `show
    // math.equation: html.frame` can result in nested ones).
    rules.register::<FrameElem>(Paged, |elem, _, _| Ok(elem.body.clone()));
}

const PAR_RULE: ShowFn<ParElem> =
    |elem, _, _| Ok(HtmlElem::new(tag::p).with_body(Some(elem.body.clone())).pack());

const STRONG_RULE: ShowFn<StrongElem> =
    |elem, _, _| Ok(HtmlElem::new(tag::strong).with_body(Some(elem.body.clone())).pack());

const EMPH_RULE: ShowFn<EmphElem> =
    |elem, _, _| Ok(HtmlElem::new(tag::em).with_body(Some(elem.body.clone())).pack());

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
        .pack())
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
        if let Smart::Custom(nr) = item.number.get(styles) {
            li = li.with_attr(attr::value, eco_format!("{nr}"));
        }
        // Text in wide enums shall always turn into paragraphs.
        let mut body = item.body.clone();
        if !elem.tight.get(styles) {
            body += ParbreakElem::shared();
        }
        li.with_body(Some(body)).pack().spanned(item.span())
    }));

    Ok(ol.with_body(Some(body)).pack())
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
    let dest = elem.dest.resolve(engine.introspector).at(elem.span())?;

    let href = match dest {
        Destination::Url(url) => Some(url.clone().into_inner()),
        Destination::Location(location) => {
            let id = engine
                .introspector
                .html_id(location)
                .cloned()
                .ok_or("failed to determine link anchor")
                .at(elem.span())?;
            Some(eco_format!("#{id}"))
        }
        Destination::Position(_) => {
            engine.sink.warn(warning!(
                elem.span(),
                "positional link was ignored during HTML export"
            ));
            None
        }
    };

    Ok(HtmlElem::new(tag::a)
        .with_optional_attr(attr::href, href)
        .with_body(Some(elem.body.clone()))
        .pack())
};

const TITLE_RULE: ShowFn<TitleElem> = |elem, _, styles| {
    Ok(HtmlElem::new(tag::h1)
        .with_body(Some(elem.resolve_body(styles).at(elem.span())?))
        .pack())
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
    } else {
        let t = [tag::h2, tag::h3, tag::h4, tag::h5, tag::h6][level - 1];
        HtmlElem::new(t).with_body(Some(realized)).pack()
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

    Ok(HtmlElem::new(tag::figure).with_body(Some(realized)).pack())
};

const FIGURE_CAPTION_RULE: ShowFn<FigureCaption> = |elem, engine, styles| {
    Ok(HtmlElem::new(tag::figcaption)
        .with_body(Some(elem.realize(engine, styles)?))
        .pack())
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
        if let Some(Attribution::Content(attribution)) = attribution
            && let Some(link) = attribution.to_packed::<LinkElem>()
            && let LinkTarget::Dest(Destination::Url(url)) = &link.dest
        {
            blockquote = blockquote.with_attr(attr::cite, url.clone().into_inner());
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

const FOOTNOTE_RULE: ShowFn<FootnoteElem> = |elem, engine, styles| {
    let span = elem.span();
    let (dest, num) = elem.realize(engine, styles)?;
    let sup = SuperElem::new(num).pack().spanned(span);

    // Link to the footnote entry.
    let link = LinkElem::new(dest.into(), sup)
        .pack()
        .styled(HtmlElem::role.set(Some("doc-noteref".into())));

    Ok(HElem::hole().clone() + link)
};

/// This is inserted at the end of the body to display footnotes. In the future,
/// we can expose this to allow customizing where the footnotes appear. It could
/// also be exposed for paged export.
#[elem]
pub struct FootnoteContainer {}

impl FootnoteContainer {
    /// Get the globally shared footnote container element.
    pub fn shared() -> &'static Content {
        singleton!(Content, FootnoteContainer::new().pack())
    }
}

const FOOTNOTE_CONTAINER_RULE: ShowFn<FootnoteContainer> = |_, engine, _| {
    let notes = engine.introspector.query(&FootnoteElem::ELEM.select());
    if notes.is_empty() {
        return Ok(Content::empty());
    }

    // Create entries for all footnotes in the document.
    let items = notes.into_iter().map(|note| {
        let note = note.into_packed::<FootnoteElem>().unwrap();
        let loc = note.location().unwrap();
        let span = note.span();
        HtmlElem::new(tag::li)
            .with_body(Some(
                FootnoteEntry::new(note).pack().spanned(span).located(loc.variant(1)),
            ))
            .with_parent(loc)
            .pack()
            .spanned(span)
    });

    // There can be multiple footnotes in a container, so they semantically
    // represent an ordered list. However, the list is already numbered with the
    // footnote superscripts in the DOM, so we turn off CSS' list enumeration.
    let list = HtmlElem::new(tag::ol)
        .with_styles(css::Properties::new().with("list-style-type", "none"))
        .with_body(Some(Content::sequence(items)))
        .pack();

    // The user may want to style the whole footnote element so we wrap it in an
    // additional selectable container. `aside` has the right semantics as a
    // container for auxiliary page content. There is no ARIA role for
    // footnotes, so we use a class instead. (There is `doc-endnotes`, but has
    // footnotes and endnotes have somewhat different semantics.)
    Ok(HtmlElem::new(tag::aside)
        .with_attr(attr::class, "footnotes")
        .with_body(Some(list))
        .pack())
};

const FOOTNOTE_ENTRY_RULE: ShowFn<FootnoteEntry> = |elem, engine, styles| {
    let span = elem.span();
    let (dest, num, body) = elem.realize(engine, styles)?;
    let sup = SuperElem::new(num).pack().spanned(span);

    // We create a link back to the first footnote reference.
    let link = LinkElem::new(dest.into(), sup)
        .pack()
        .spanned(span)
        .styled(HtmlElem::role.set(Some("doc-backlink".into())));

    // We want to use the Digital Publishing ARIA role `doc-footnote` and the
    // fallback role `note` for each individual footnote. Because the enclosing
    // `li`, as a child of an `ol`, must have the implicit `listitem` role, we
    // need an additional container. We chose a `div` instead of a `span` to
    // allow for block-level content in the footnote.
    Ok(HtmlElem::new(tag::div)
        .with_attr(attr::role, "doc-footnote note")
        .with_body(Some(link + body))
        .pack())
};

const OUTLINE_RULE: ShowFn<OutlineElem> = |elem, engine, styles| {
    fn convert_list(list: Vec<OutlineNode>) -> Content {
        // The Digital Publishing ARIA spec also proposed to add
        // `role="directory"` to the `<ol>` element, but this role is
        // deprecated, so we don't do that. The elements are already easily
        // selectable via `nav[role="doc-toc"] ol`.
        HtmlElem::new(tag::ol)
            .with_styles(css::Properties::new().with("list-style-type", "none"))
            .with_body(Some(Content::sequence(list.into_iter().map(convert_node))))
            .pack()
    }

    fn convert_node(node: OutlineNode) -> Content {
        let body = if !node.children.is_empty() {
            // The `<div>` is not technically necessary, but otherwise it
            // auto-wraps in a `<p>`, which results in bad spacing. Perhaps, we
            // can remove this in the future. See also:
            // <https://github.com/typst/typst/issues/5907>
            HtmlElem::new(tag::div).with_body(Some(node.entry.pack())).pack()
                + convert_list(node.children)
        } else {
            node.entry.pack()
        };
        HtmlElem::new(tag::li).with_body(Some(body)).pack()
    }

    let title = elem.realize_title(styles);
    let tree = elem.realize_tree(engine, styles)?;
    let list = convert_list(tree);

    Ok(HtmlElem::new(tag::nav)
        .with_attr(attr::role, "doc-toc")
        .with_body(Some(title.unwrap_or_default() + list))
        .pack())
};

const OUTLINE_ENTRY_RULE: ShowFn<OutlineEntry> = |elem, engine, styles| {
    let span = elem.span();
    let context = Context::new(None, Some(styles));

    let mut realized = elem.body().at(span)?;

    if let Some(prefix) = elem.prefix(engine, context.track(), span)? {
        let wrapped = HtmlElem::new(tag::span)
            .with_attr(attr::class, "prefix")
            .with_body(Some(prefix))
            .pack()
            .spanned(span);

        let separator = match elem.element.to_packed::<FigureElem>() {
            Some(elem) => elem.resolve_separator(styles),
            None => SpaceElem::shared().clone(),
        };

        realized = Content::sequence([wrapped, separator, realized]);
    }

    let loc = elem.element_location().at(span)?;
    let dest = Destination::Location(loc);

    Ok(LinkElem::new(dest.into(), realized).pack())
};

const REF_RULE: ShowFn<RefElem> = |elem, engine, styles| elem.realize(engine, styles);

const CITE_GROUP_RULE: ShowFn<CiteGroup> = |elem, engine, _| {
    let (dest, content) = elem.realize(engine)?;

    if let Some(bibliography_destination) = dest {
        let link = LinkElem::new(bibliography_destination.into(), content)
            .pack()
            .styled(HtmlElem::role.set(Some("doc-noteref".into())));
        return Ok(link);
    }

    Ok(content)
};

const BIBLIOGRAPHY_RULE: ShowFn<BibliographyElem> = |elem, engine, styles| {
    let span = elem.span();
    let mut content_sequence = vec![];

    // Add title if set
    if let Some(title) = elem.realize_title(styles) {
        content_sequence.push(title);
    }

    // Add bibliography references as a list
    let works = elem.realize_works(engine, styles)?;
    let references = works.references.as_ref().unwrap();

    // Get all citation groups to create backlinks
    let cite_groups = engine.introspector.query(&CiteGroup::ELEM.select());
    let mut first_occurrences = std::collections::HashMap::new();

    // Build a map from citation keys to their first occurrence locations
    for group_elem in cite_groups {
        if let Some(group) = group_elem.to_packed::<CiteGroup>() {
            if let Some(location) = group_elem.location() {
                for cite_elem in &group.children {
                    let key = cite_elem.key.resolve();
                    first_occurrences.entry(key).or_insert(location);
                }
            }
        }
    }

    // Get the bibliography keys in order
    let bibliography_keys: Vec<_> = BibliographyElem::keys(engine.introspector)
        .into_iter()
        .map(|(key, _)| key)
        .collect();

    let list_items = references.iter().enumerate().map(|(index, (prefix, reference))| {
        let mut item_content = vec![];

        if let Some(prefix) = prefix {
            // Create backlink from bibliography entry to first citation occurrence
            let mut prefix_content = HtmlElem::new(tag::span)
                .with_attr(attr::class, "prefix")
                .with_body(Some(prefix.clone()))
                .pack()
                .spanned(span);

            // Try to create a backlink if we can map this entry to a citation
            if let Some(key) = bibliography_keys.get(index) {
                if let Some(&first_location) = first_occurrences.get(&key.resolve()) {
                    // Found the first citation for this bibliography entry
                    let dest = Destination::Location(first_location);
                    prefix_content = LinkElem::new(dest.into(), prefix_content)
                        .pack()
                        .styled(HtmlElem::role.set(Some("doc-backlink".into())));
                } else {
                    // Citation key exists but no citation found, just add role
                    prefix_content = prefix_content
                        .styled(HtmlElem::role.set(Some("doc-backlink".into())));
                }
            } else {
                // No key mapping found, just add role
                prefix_content = prefix_content
                    .styled(HtmlElem::role.set(Some("doc-backlink".into())));
            }

            item_content.push(prefix_content);
            item_content.push(SpaceElem::shared().clone());
        }

        item_content.push(reference.clone());

        HtmlElem::new(tag::li)
            .with_body(Some(Content::sequence(item_content)))
            .pack()
            .spanned(span)
    });

    let bibliography_list = HtmlElem::new(tag::ul)
        .with_body(Some(Content::sequence(list_items)))
        .pack()
        .spanned(span);

    content_sequence.push(bibliography_list);

    Ok(HtmlElem::new(tag::section)
        .with_attr(attr::role, "doc-bibliography")
        .with_body(Some(Content::sequence(content_sequence)))
        .pack())
};

const TABLE_RULE: ShowFn<TableElem> = |elem, engine, styles| {
    Ok(show_cellgrid(table_to_cellgrid(elem, engine, styles)?, styles))
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

const SUB_RULE: ShowFn<SubElem> =
    |elem, _, _| Ok(HtmlElem::new(tag::sub).with_body(Some(elem.body.clone())).pack());

const SUPER_RULE: ShowFn<SuperElem> =
    |elem, _, _| Ok(HtmlElem::new(tag::sup).with_body(Some(elem.body.clone())).pack());

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

const SMALLCAPS_RULE: ShowFn<SmallcapsElem> = |elem, _, styles| {
    Ok(HtmlElem::new(tag::span)
        .with_attr(
            attr::style,
            if elem.all.get(styles) {
                "font-variant-caps: all-small-caps"
            } else {
                "font-variant-caps: small-caps"
            },
        )
        .with_body(Some(elem.body.clone()))
        .pack())
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

    let code = HtmlElem::new(tag::code)
        .with_body(Some(Content::sequence(seq)))
        .pack()
        .spanned(elem.span());

    Ok(if elem.block.get(styles) {
        HtmlElem::new(tag::pre).with_body(Some(code)).pack()
    } else {
        code
    })
};

/// This is used by `RawElem::synthesize` through a routine.
///
/// It's a temporary workaround until `TextElem::fill` is supported in HTML
/// export.
#[doc(hidden)]
pub fn html_span_filled(content: Content, color: Color) -> Content {
    let span = content.span();
    HtmlElem::new(tag::span)
        .with_styles(css::Properties::new().with("color", css::color(color)))
        .with_body(Some(content))
        .pack()
        .spanned(span)
}

const RAW_LINE_RULE: ShowFn<RawLine> = |elem, _, _| Ok(elem.body.clone());

// TODO: This is rather incomplete.
const BLOCK_RULE: ShowFn<BlockElem> = |elem, _, styles| {
    let body = match elem.body.get_cloned(styles) {
        None => None,
        Some(BlockBody::Content(body)) => Some(body),
        // These are only generated by native `typst-layout` show rules.
        Some(BlockBody::SingleLayouter(_) | BlockBody::MultiLayouter(_)) => {
            bail!(
                elem.span(),
                "blocks with layout routines should not occur in \
                 HTML export – this is a bug"
            )
        }
    };

    Ok(HtmlElem::new(tag::div).with_body(body).pack())
};

// TODO: This is rather incomplete.
const BOX_RULE: ShowFn<BoxElem> = |elem, _, styles| {
    Ok(HtmlElem::new(tag::span)
        .with_styles(css::Properties::new().with("display", "inline-block"))
        .with_body(elem.body.get_cloned(styles))
        .pack())
};

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
