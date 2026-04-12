use comemo::{Track, Tracked, TrackedMut};
use ecow::{EcoVec, eco_vec};
use typst_library::World;
use typst_library::diag::{SourceResult, bail, error};
use typst_library::engine::{Engine, Route, Sink, Traced};
use typst_library::foundations::{Content, NativeElement, StyleChain, Styles};
use typst_library::introspection::{
    Introspector, Locator, LocatorLink, QueryIntrospection,
};
use typst_library::math::EquationElem;
use typst_library::model::{DocumentInfo, FootnoteContainer, FootnoteMarker};
use typst_library::routines::{Arenas, RealizationKind, Routines};
use typst_syntax::Span;
use typst_utils::Protected;

use crate::convert::{ConversionLevel, Whitespace};
use crate::mathml::EQUATION_CSS_STYLES;
use crate::{HtmlDocument, HtmlElement, HtmlNode, attr, css, tag};

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
    let mut document = html_document_common(
        routines,
        world,
        introspector,
        traced,
        sink,
        route,
        content,
        Locator::root(),
        styles,
    )?;

    // Assigns HTML fragment IDs to linked-to elements.
    let targets = document.introspector().link_targets();
    let anchors = crate::link::create_link_anchors(&mut document, &targets);
    document.introspector_mut().set_anchors(anchors);

    Ok(document)
}

/// Produce an HTML document from content, as part of a bundle compilation
/// process.
#[typst_macros::time(name = "html document")]
pub fn html_document_for_bundle(
    engine: &mut Engine,
    content: &Content,
    locator: Locator,
    styles: StyleChain,
) -> SourceResult<HtmlDocument> {
    html_document_for_bundle_impl(
        engine.routines,
        engine.world,
        engine.introspector.into_raw(),
        engine.traced,
        TrackedMut::reborrow_mut(&mut engine.sink),
        engine.route.track(),
        content,
        locator.track(),
        styles,
    )
}

/// The internal implementation of `html_document_for_bundle`.
#[comemo::memoize]
#[allow(clippy::too_many_arguments)]
fn html_document_for_bundle_impl(
    routines: &Routines,
    world: Tracked<dyn World + '_>,
    introspector: Tracked<dyn Introspector + '_>,
    traced: Tracked<Traced>,
    sink: TrackedMut<Sink>,
    route: Tracked<Route>,
    content: &Content,
    locator: Tracked<Locator>,
    styles: StyleChain,
) -> SourceResult<HtmlDocument> {
    let link = LocatorLink::new(locator);
    html_document_common(
        routines,
        world,
        introspector,
        traced,
        sink,
        route,
        content,
        Locator::link(&link),
        styles,
    )
}

/// The shared, unmemoized implementation of `html_document` and
/// `html_document_for_bundle`.
#[allow(clippy::too_many_arguments)]
fn html_document_common(
    routines: &Routines,
    world: Tracked<dyn World + '_>,
    introspector: Tracked<dyn Introspector + '_>,
    traced: Tracked<Traced>,
    sink: TrackedMut<Sink>,
    route: Tracked<Route>,
    content: &Content,
    locator: Locator,
    styles: StyleChain,
) -> SourceResult<HtmlDocument> {
    let introspector = Protected::from_raw(introspector);
    let mut locator = locator.split();
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
    info.populate(styles);
    info.populate_locale(styles);

    let children = (engine.routines.realize)(
        RealizationKind::HtmlDocument { info: &mut info },
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

    let mut output = finalize_dom(
        &mut engine,
        nodes,
        &info,
        footnote_locator,
        StyleChain::new(&Styles::root(&children, styles)),
    )?;

    // Since `finalize_dom` might have inserted more DOM nodes that have styles,
    // the styles must be resolved last.
    css::resolve_inline_styles(output.root_mut());

    Ok(HtmlDocument::new(output, info))
}

/// The introspectible output of HTML compilation.
#[derive(Debug, Clone)]
pub struct HtmlOutput {
    nodes: EcoVec<HtmlNode>,
    root_index: usize,
}

impl HtmlOutput {
    /// All nodes.
    pub fn nodes(&self) -> &[HtmlNode] {
        &self.nodes
    }

    /// The root note.
    pub fn root(&self) -> &HtmlElement {
        match &self.nodes[self.root_index] {
            HtmlNode::Element(root) => root,
            _ => panic!("expected HTML element"),
        }
    }

    /// The root note, mutably.
    pub fn root_mut(&mut self) -> &mut HtmlElement {
        match &mut self.nodes.make_mut()[self.root_index] {
            HtmlNode::Element(root) => root,
            _ => panic!("expected HTML element"),
        }
    }

    /// The document's root HTML element, in its containing node wrapper.
    pub fn root_node(&self) -> &HtmlNode {
        &self.nodes[self.root_index]
    }
}

/// Wrap the user generated HTML in <html>, <body> or both if needed.
///
/// Returns a vector containing outer introspection tags and the HTML root element.
/// A direct reference to the root element is also returned.
fn finalize_dom(
    engine: &mut Engine,
    nodes: EcoVec<HtmlNode>,
    info: &DocumentInfo,
    footnote_locator: Locator<'_>,
    footnote_styles: StyleChain<'_>,
) -> SourceResult<HtmlOutput> {
    let count = nodes.iter().filter(|node| !matches!(node, HtmlNode::Tag(_))).count();

    let has_equations = !engine
        .introspect(QueryIntrospection(EquationElem::ELEM.select(), Span::detached()))
        .is_empty();

    let mut needs_body = true;
    for (idx, node) in nodes.iter().enumerate() {
        let HtmlNode::Element(elem) = node else { continue };
        let tag = elem.tag;
        match (tag, count) {
            (tag::html, 1) => {
                footnotes_unsupported_with_custom_dom(engine)?;
                return Ok(HtmlOutput { nodes, root_index: idx });
            }
            (tag::body, 1) => {
                footnotes_unsupported_with_custom_dom(engine)?;
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
        let mut body = HtmlElement::new(tag::body).with_children(nodes);
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
        nodes
    };

    let mut html = HtmlElement::new(tag::html)
        .with_attr(attr::lang, info.locale.unwrap_or_default().rfc_3066());
    let head = head_element(info, has_equations);
    html.children.push(head.into());
    html.children.extend(body);
    Ok(HtmlOutput { nodes: eco_vec![html.into()], root_index: 0 })
}

/// Generate a `<head>` element.
fn head_element(info: &DocumentInfo, has_equations: bool) -> HtmlElement {
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

    if has_equations {
        children.push(
            HtmlElement::new(tag::style)
                .with_children(eco_vec![HtmlNode::Text(
                    EQUATION_CSS_STYLES.clone(),
                    Span::detached(),
                )])
                .into(),
        )
    }

    HtmlElement::new(tag::head).with_children(children)
}

/// Fails with an error if there are footnotes.
fn footnotes_unsupported_with_custom_dom(engine: &mut Engine) -> SourceResult<()> {
    let markers = engine
        .introspect(QueryIntrospection(FootnoteMarker::ELEM.select(), Span::detached()));

    if markers.is_empty() {
        return Ok(());
    }

    Err(markers
        .iter()
        .map(|marker| {
            error!(
                marker.span(),
                "footnotes are not currently supported in combination \
                 with a custom `<html>` or `<body>` element";
                hint: "you can still use footnotes with a custom footnote show rule";
            )
        })
        .collect())
}
