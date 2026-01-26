use std::sync::Arc;

use comemo::{Tracked, TrackedMut};
use ecow::{EcoVec, eco_vec};
use typst_library::World;
use typst_library::diag::{SourceResult, bail};
use typst_library::engine::{Engine, Route, Sink, Traced};
use typst_library::foundations::{Content, StyleChain, Styles};
use typst_library::introspection::{Introspector, Locator};
use typst_library::model::DocumentInfo;
use typst_library::routines::{Arenas, RealizationKind, Routines};
use typst_syntax::Span;
use typst_utils::Protected;

use crate::convert::{ConversionLevel, Whitespace};
use crate::rules::FootnoteContainer;
use crate::{HtmlDocument, HtmlElem, HtmlElement, HtmlIntrospector, HtmlNode, attr, tag};

/// Produce an HTML document from content.
///
/// This first performs root-level realization and then turns the resulting
/// elements into HTML.
#[typst_macros::time(name = "html document")]
pub fn html_document(
    engine: &mut Engine,
    content: &Content,
    styles: StyleChain,
) -> SourceResult<HtmlDocument> {
    html_document_impl(
        engine.routines,
        engine.world,
        engine.introspector.into_raw(),
        engine.traced,
        TrackedMut::reborrow_mut(&mut engine.sink),
        engine.route.track(),
        content,
        styles,
    )
}

/// The internal implementation of `html_document`.
#[comemo::memoize]
#[allow(clippy::too_many_arguments)]
fn html_document_impl(
    routines: &Routines,
    world: Tracked<dyn World + '_>,
    introspector: Tracked<dyn Introspector + '_>,
    traced: Tracked<Traced>,
    sink: TrackedMut<Sink>,
    route: Tracked<Route>,
    content: &Content,
    styles: StyleChain,
) -> SourceResult<HtmlDocument> {
    let introspector = Protected::from_raw(introspector);
    let mut locator = Locator::root().split();
    let mut engine = Engine {
        routines,
        world,
        introspector,
        traced,
        sink,
        route: Route::extend(route).unnested(),
    };

    // Create this upfront to make it as stable as possible.
    let footnote_locator = locator.next(&());

    // Mark the external styles as "outside" so that they are valid at the
    // document level.
    let styles = styles.to_map().outside();
    let styles = StyleChain::new(&styles);

    let arenas = Arenas::default();
    let mut info = DocumentInfo::default();
    let children = (engine.routines.realize)(
        RealizationKind::HtmlDocument {
            info: &mut info,
            is_phrasing: HtmlElem::is_phrasing,
        },
        &mut engine,
        &mut locator,
        &arenas,
        content,
        styles,
    )?;

    let nodes = crate::convert::convert_to_nodes(
        &mut engine,
        &mut locator,
        children.iter().copied(),
        ConversionLevel::Block,
        Whitespace::Normal,
    )?;

    let (mut tags_and_root, root_index) = finalize_dom(
        &mut engine,
        nodes,
        &info,
        footnote_locator,
        StyleChain::new(&Styles::root(&children, styles)),
    )?;

    let mut introspector = HtmlIntrospector::new(&tags_and_root);
    let HtmlNode::Element(mut root) = tags_and_root.remove(root_index) else {
        panic!("expected HTML element")
    };

    introspector.set_anchors(crate::link::create_link_anchors(
        &mut root,
        &introspector,
        &introspector.link_targets(),
    ));

    Ok(HtmlDocument { info, root, introspector: Arc::new(introspector) })
}

/// Wrap the user generated HTML in <html>, <body> or both if needed.
///
/// Returns a vector containing outer introspection tags and the HTML root element.
/// A direct reference to the root element is also returned.
fn finalize_dom(
    engine: &mut Engine,
    output: EcoVec<HtmlNode>,
    info: &DocumentInfo,
    footnote_locator: Locator<'_>,
    footnote_styles: StyleChain<'_>,
) -> SourceResult<(EcoVec<HtmlNode>, usize)> {
    let count = output.iter().filter(|node| !matches!(node, HtmlNode::Tag(_))).count();

    let mut needs_body = true;
    for (idx, node) in output.iter().enumerate() {
        let HtmlNode::Element(elem) = node else { continue };
        let tag = elem.tag;
        match (tag, count) {
            (tag::html, 1) => {
                FootnoteContainer::unsupported_with_custom_dom(engine)?;
                return Ok((output, idx));
            }
            (tag::body, 1) => {
                FootnoteContainer::unsupported_with_custom_dom(engine)?;
                needs_body = false;
            }
            (tag::html | tag::body, _) => bail!(
                elem.span,
                "`{}` element must be the only element in the document",
                elem.tag,
            ),
            _ => {}
        }
    }

    let body = if needs_body {
        let mut body = HtmlElement::new(tag::body).with_children(output);
        let footnotes = crate::fragment::html_block_fragment(
            engine,
            FootnoteContainer::shared(),
            footnote_locator,
            footnote_styles,
            Whitespace::Normal,
        )?;
        body.children.extend(footnotes);
        eco_vec![body.into()]
    } else {
        output
    };

    let mut html = HtmlElement::new(tag::html)
        .with_attr(attr::lang, info.locale.unwrap_or_default().rfc_3066());
    let head = head_element(info);
    html.children.push(head.into());
    html.children.extend(body);
    Ok((eco_vec![html.into()], 0))
}

/// Generate a `<head>` element.
fn head_element(info: &DocumentInfo) -> HtmlElement {
    let mut children = EcoVec::new();

    children.push(HtmlElement::new(tag::meta).with_attr(attr::charset, "utf-8").into());

    children.push(
        HtmlElement::new(tag::meta)
            .with_attr(attr::name, "viewport")
            .with_attr(attr::content, "width=device-width, initial-scale=1")
            .into(),
    );

    if let Some(title) = &info.title {
        children.push(
            HtmlElement::new(tag::title)
                .with_children(eco_vec![HtmlNode::Text(title.clone(), Span::detached())])
                .into(),
        );
    }

    if let Some(description) = &info.description {
        children.push(
            HtmlElement::new(tag::meta)
                .with_attr(attr::name, "description")
                .with_attr(attr::content, description.clone())
                .into(),
        );
    }

    if !info.author.is_empty() {
        children.push(
            HtmlElement::new(tag::meta)
                .with_attr(attr::name, "authors")
                .with_attr(attr::content, info.author.join(", "))
                .into(),
        )
    }

    if !info.keywords.is_empty() {
        children.push(
            HtmlElement::new(tag::meta)
                .with_attr(attr::name, "keywords")
                .with_attr(attr::content, info.keywords.join(", "))
                .into(),
        )
    }

    HtmlElement::new(tag::head).with_children(children)
}
