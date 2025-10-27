use std::num::NonZeroUsize;

use comemo::{Tracked, TrackedMut};
use ecow::{EcoVec, eco_vec};
use rustc_hash::FxHashSet;
use typst_library::World;
use typst_library::diag::{SourceResult, bail};
use typst_library::engine::{Engine, Route, Sink, Traced};
use typst_library::foundations::{Content, StyleChain, Styles};
use typst_library::introspection::{
    DocumentPosition, HtmlPosition, InnerHtmlPosition, Introspector, IntrospectorBuilder,
    Location, Locator,
};
use typst_library::layout::{Position, Transform};
use typst_library::model::DocumentInfo;
use typst_library::routines::{Arenas, RealizationKind, Routines};
use typst_syntax::Span;
use typst_utils::NonZeroExt;

use crate::convert::{ConversionLevel, Whitespace};
use crate::rules::FootnoteContainer;
use crate::{HtmlDocument, HtmlElem, HtmlElement, HtmlNode, attr, tag};

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

macro_rules! unwrap_elem {
    ($e:expr) => {
        match $e {
            HtmlNode::Element(e) => e,
            _ => panic!("Expected HTML element"),
        }
    };
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

    // Create this upfront to make it as stable as possible.
    let footnote_locator = locator.next(&());

    // Mark the external styles as "outside" so that they are valid at the
    // document level.
    let styles = styles.to_map().outside();
    let styles = StyleChain::new(&styles);

    let arenas = Arenas::default();
    let mut info = DocumentInfo::default();
    let children = (engine.routines.realize)(
        RealizationKind::HtmlDocument { info: &mut info, is_inline: HtmlElem::is_inline },
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

    let (mut tags_and_root, root_index) = prepare_document(
        &mut engine,
        nodes,
        &info,
        footnote_locator,
        StyleChain::new(&Styles::root(&children, styles)),
    )?;

    let mut link_targets = FxHashSet::default();
    let mut introspector = introspect_html(&tags_and_root, &mut link_targets);
    let root = unwrap_elem!(&mut tags_and_root[root_index]);
    crate::link::identify_link_targets(root, &mut introspector, link_targets);

    Ok(HtmlDocument { info, root: root.clone(), introspector })
}

/// Introspects HTML nodes.
#[typst_macros::time(name = "introspect html")]
fn introspect_html(
    output: &[HtmlNode],
    link_targets: &mut FxHashSet<Location>,
) -> Introspector {
    fn discover(
        builder: &mut IntrospectorBuilder,
        sink: &mut Vec<(Content, DocumentPosition)>,
        link_targets: &mut FxHashSet<Location>,
        nodes: &[HtmlNode],
        current_position: &mut EcoVec<usize>,
    ) {
        let mut index = 0;
        for node in nodes {
            match node {
                HtmlNode::Tag(tag) => {
                    current_position.push(index);
                    builder.discover_in_tag(
                        sink,
                        tag,
                        HtmlPosition { element: current_position.clone(), inner: None },
                    );
                    current_position.pop();
                }
                HtmlNode::Text(_, _) => {
                    index += 1;
                }
                HtmlNode::Element(elem) => {
                    let is_root = elem.tag == tag::html;
                    if !is_root {
                        current_position.push(index);
                    }

                    if let Some(parent) = elem.parent {
                        let mut nested = vec![];
                        discover(
                            builder,
                            &mut nested,
                            link_targets,
                            &elem.children,
                            current_position,
                        );
                        builder.register_insertion(parent, nested);
                    } else {
                        discover(
                            builder,
                            sink,
                            link_targets,
                            &elem.children,
                            current_position,
                        );
                    }

                    if !is_root {
                        current_position.pop();
                    }
                    index += 1;
                }
                HtmlNode::Frame(frame) => {
                    current_position.push(index);

                    let mut frame_sink = Vec::new();
                    builder.discover_in_frame(
                        &mut frame_sink,
                        &frame.inner,
                        NonZeroUsize::ONE,
                        Transform::identity(),
                    );

                    for pair in frame_sink {
                        if let DocumentPosition::Paged(Position { point, .. }) = pair.1 {
                            sink.push((
                                pair.0,
                                DocumentPosition::Html(HtmlPosition {
                                    element: current_position.clone(),
                                    inner: Some(InnerHtmlPosition::Frame {
                                        x: point.x,
                                        y: point.y,
                                    }),
                                }),
                            ))
                        }
                    }

                    crate::link::introspect_frame_links(&frame.inner, link_targets);
                    current_position.pop();
                    index += 1;
                }
            }
        }
    }

    let mut elems = Vec::new();
    let mut builder = IntrospectorBuilder::new();
    let mut current_position = EcoVec::new();
    discover(&mut builder, &mut elems, link_targets, output, &mut current_position);
    builder.finalize(elems)
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

/// Wrap the user generated HTML in <html>, <body> or both if needed.
///
/// Returns a vector containing outer introspection tags and the HTML root element.
/// A direct reference to the root element is also returned.
fn prepare_document(
    engine: &mut Engine,
    mut output: EcoVec<HtmlNode>,
    info: &DocumentInfo,
    footnote_locator: Locator<'_>,
    footnote_styles: StyleChain<'_>,
) -> SourceResult<(Vec<HtmlNode>, usize)> {
    fn not_a_tag(node: &HtmlNode) -> bool {
        !matches!(node, HtmlNode::Tag(_))
    }

    fn is_body_or_html(node: &HtmlNode) -> bool {
        match node {
            HtmlNode::Element(HtmlElement { tag, .. }) => {
                *tag == tag::html || *tag == tag::body
            }
            _ => false,
        }
    }

    let preceding_tags_end = output.iter().position(not_a_tag).unwrap_or(0);
    let following_tags_start = output
        .iter()
        .rposition(not_a_tag)
        .map(|x| x + 1)
        .unwrap_or(output.len());
    let non_tag_count = following_tags_start - preceding_tags_end;
    let mut needs_body = false;
    let mut needs_html = false;

    if non_tag_count == 0 {
        // If there are only tags, wrap them in <html> and <body>
        needs_body = true;
        needs_html = true;
    } else if non_tag_count == 1 {
        let unique_node = &output[preceding_tags_end];
        if let HtmlNode::Element(HtmlElement { tag, .. }) = unique_node
            && *tag != tag::html
        {
            if *tag != tag::body {
                needs_body = true;
            }

            needs_html = true;
        }

        if !needs_body {
            // If the user supplied their own <body> or <html>, check that they don't use footnotes.
            FootnoteContainer::unsupported_with_custom_dom(engine)?;
        }
    } else {
        // If there is more than one node, you are not allowed to emit a <body>
        // or <html> element.
        for node in &output {
            if is_body_or_html(node) {
                let elem = unwrap_elem!(node);
                bail!(
                    elem.span,
                    "`{}` element must be the only element in the document",
                    elem.tag,
                )
            }
        }

        // And all elements are necessarily wrapped.
        needs_body = true;
        needs_html = true;
    }

    if needs_body {
        let mut body = HtmlElement::new(tag::body).with_children(output);
        let footnotes = crate::fragment::html_block_fragment(
            engine,
            FootnoteContainer::shared(),
            footnote_locator,
            footnote_styles,
            Whitespace::Normal,
        )?;
        body.children.extend(footnotes);
        output = eco_vec![body.into()];
    }

    if needs_html {
        let mut html = HtmlElement::new(tag::html);
        let head = head_element(info);
        html.children.push(head.into());
        html.children.extend(output);
        Ok((vec![html.into()], 0))
    } else {
        Ok((output.into_iter().collect(), preceding_tags_end))
    }
}
