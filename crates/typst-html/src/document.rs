use std::collections::HashSet;
use std::num::NonZeroUsize;

use comemo::{Tracked, TrackedMut};
use typst_library::diag::{bail, SourceResult};
use typst_library::engine::{Engine, Route, Sink, Traced};
use typst_library::foundations::{Content, StyleChain};
use typst_library::introspection::{
    Introspector, IntrospectorBuilder, Location, Locator,
};
use typst_library::layout::{Point, Position, Transform};
use typst_library::model::DocumentInfo;
use typst_library::routines::{Arenas, RealizationKind, Routines};
use typst_library::World;
use typst_syntax::Span;
use typst_utils::NonZeroExt;

use crate::{attr, tag, HtmlDocument, HtmlElement, HtmlNode};

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
        engine.introspector,
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
    introspector: Tracked<Introspector>,
    traced: Tracked<Traced>,
    sink: TrackedMut<Sink>,
    route: Tracked<Route>,
    content: &Content,
    styles: StyleChain,
) -> SourceResult<HtmlDocument> {
    let mut locator = Locator::root().split();
    let mut engine = Engine {
        routines,
        world,
        introspector,
        traced,
        sink,
        route: Route::extend(route).unnested(),
    };

    // Mark the external styles as "outside" so that they are valid at the page
    // level.
    let styles = styles.to_map().outside();
    let styles = StyleChain::new(&styles);

    let arenas = Arenas::default();
    let mut info = DocumentInfo::default();
    let children = (engine.routines.realize)(
        RealizationKind::HtmlDocument {
            info: &mut info,
            is_inline: crate::convert::is_inline,
        },
        &mut engine,
        &mut locator,
        &arenas,
        content,
        styles,
    )?;

    let output = crate::convert::convert_to_nodes(
        &mut engine,
        &mut locator,
        children.iter().copied(),
    )?;

    let mut link_targets = HashSet::new();
    let mut introspector = introspect_html(&output, &mut link_targets);
    let mut root = root_element(output, &info)?;
    crate::link::identify_link_targets(&mut root, &mut introspector, link_targets);

    Ok(HtmlDocument { info, root, introspector })
}

/// Introspects HTML nodes.
#[typst_macros::time(name = "introspect html")]
fn introspect_html(
    output: &[HtmlNode],
    link_targets: &mut HashSet<Location>,
) -> Introspector {
    fn discover(
        builder: &mut IntrospectorBuilder,
        sink: &mut Vec<(Content, Position)>,
        link_targets: &mut HashSet<Location>,
        nodes: &[HtmlNode],
    ) {
        for node in nodes {
            match node {
                HtmlNode::Tag(tag) => {
                    builder.discover_in_tag(
                        sink,
                        tag,
                        Position { page: NonZeroUsize::ONE, point: Point::zero() },
                    );
                }
                HtmlNode::Text(_, _) => {}
                HtmlNode::Element(elem) => {
                    discover(builder, sink, link_targets, &elem.children)
                }
                HtmlNode::Frame(frame) => {
                    builder.discover_in_frame(
                        sink,
                        &frame.inner,
                        NonZeroUsize::ONE,
                        Transform::identity(),
                    );
                    crate::link::introspect_frame_links(&frame.inner, link_targets);
                }
            }
        }
    }

    let mut elems = Vec::new();
    let mut builder = IntrospectorBuilder::new();
    discover(&mut builder, &mut elems, link_targets, output);
    builder.finalize(elems)
}

/// Wrap the nodes in `<html>` and `<body>` if they are not yet rooted,
/// supplying a suitable `<head>`.
fn root_element(output: Vec<HtmlNode>, info: &DocumentInfo) -> SourceResult<HtmlElement> {
    let head = head_element(info);
    let body = match classify_output(output)? {
        OutputKind::Html(element) => return Ok(element),
        OutputKind::Body(body) => body,
        OutputKind::Leafs(leafs) => HtmlElement::new(tag::body).with_children(leafs),
    };
    Ok(HtmlElement::new(tag::html).with_children(vec![head.into(), body.into()]))
}

/// Generate a `<head>` element.
fn head_element(info: &DocumentInfo) -> HtmlElement {
    let mut children = vec![];

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
                .with_children(vec![HtmlNode::Text(title.clone(), Span::detached())])
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

/// Determine which kind of output the user generated.
fn classify_output(mut output: Vec<HtmlNode>) -> SourceResult<OutputKind> {
    let count = output.iter().filter(|node| !matches!(node, HtmlNode::Tag(_))).count();
    for node in &mut output {
        let HtmlNode::Element(elem) = node else { continue };
        let tag = elem.tag;
        let mut take = || std::mem::replace(elem, HtmlElement::new(tag::html));
        match (tag, count) {
            (tag::html, 1) => return Ok(OutputKind::Html(take())),
            (tag::body, 1) => return Ok(OutputKind::Body(take())),
            (tag::html | tag::body, _) => bail!(
                elem.span,
                "`{}` element must be the only element in the document",
                elem.tag,
            ),
            _ => {}
        }
    }
    Ok(OutputKind::Leafs(output))
}

/// What kinds of output the user generated.
enum OutputKind {
    /// The user generated their own `<html>` element. We do not need to supply
    /// one.
    Html(HtmlElement),
    /// The user generate their own `<body>` element. We do not need to supply
    /// one, but need supply the `<html>` element.
    Body(HtmlElement),
    /// The user generated leafs which we wrap in a `<body>` and `<html>`.
    Leafs(Vec<HtmlNode>),
}
