use typst_library::diag::{SourceResult, warning};
use typst_library::engine::Engine;
use typst_library::foundations::{Content, StyleChain, Target, TargetElem};
use typst_library::introspection::{SplitLocator, TagElem};
use typst_library::layout::{Abs, Axes, BlockBody, BlockElem, BoxElem, Region, Size};
use typst_library::model::ParElem;
use typst_library::routines::Pair;
use typst_library::text::{LinebreakElem, SmartQuoteElem, SpaceElem, TextElem};

use crate::fragment::html_fragment;
use crate::{FrameElem, HtmlElem, HtmlElement, HtmlFrame, HtmlNode, attr, tag};

/// Converts realized content into HTML nodes.
pub fn convert_to_nodes<'a>(
    engine: &mut Engine,
    locator: &mut SplitLocator,
    children: impl IntoIterator<Item = Pair<'a>>,
) -> SourceResult<Vec<HtmlNode>> {
    let mut output = Vec::new();
    for (child, styles) in children {
        handle(engine, child, locator, styles, &mut output)?;
    }
    Ok(output)
}

/// Convert one element into HTML node(s).
fn handle(
    engine: &mut Engine,
    child: &Content,
    locator: &mut SplitLocator,
    styles: StyleChain,
    output: &mut Vec<HtmlNode>,
) -> SourceResult<()> {
    if let Some(elem) = child.to_packed::<TagElem>() {
        output.push(HtmlNode::Tag(elem.tag.clone()));
    } else if let Some(elem) = child.to_packed::<HtmlElem>() {
        let mut children = vec![];
        if let Some(body) = elem.body.get_ref(styles) {
            children = html_fragment(engine, body, locator.next(&elem.span()), styles)?;
        }
        let element = HtmlElement {
            tag: elem.tag,
            attrs: elem.attrs.get_cloned(styles),
            children,
            span: elem.span(),
        };
        output.push(element.into());
    } else if let Some(elem) = child.to_packed::<ParElem>() {
        let children =
            html_fragment(engine, &elem.body, locator.next(&elem.span()), styles)?;
        output.push(
            HtmlElement::new(tag::p)
                .with_children(children)
                .spanned(elem.span())
                .into(),
        );
    } else if let Some(elem) = child.to_packed::<BoxElem>() {
        // TODO: This is rather incomplete.
        if let Some(body) = elem.body.get_ref(styles) {
            let children =
                html_fragment(engine, body, locator.next(&elem.span()), styles)?;
            output.push(
                HtmlElement::new(tag::span)
                    .with_attr(attr::style, "display: inline-block;")
                    .with_children(children)
                    .spanned(elem.span())
                    .into(),
            )
        }
    } else if let Some((elem, body)) =
        child
            .to_packed::<BlockElem>()
            .and_then(|elem| match elem.body.get_ref(styles) {
                Some(BlockBody::Content(body)) => Some((elem, body)),
                _ => None,
            })
    {
        // TODO: This is rather incomplete.
        let children = html_fragment(engine, body, locator.next(&elem.span()), styles)?;
        output.push(
            HtmlElement::new(tag::div)
                .with_children(children)
                .spanned(elem.span())
                .into(),
        );
    } else if child.is::<SpaceElem>() {
        output.push(HtmlNode::text(' ', child.span()));
    } else if let Some(elem) = child.to_packed::<TextElem>() {
        let text = if let Some(case) = styles.get(TextElem::case) {
            case.apply(&elem.text).into()
        } else {
            elem.text.clone()
        };
        output.push(HtmlNode::text(text, elem.span()));
    } else if let Some(elem) = child.to_packed::<LinebreakElem>() {
        output.push(HtmlElement::new(tag::br).spanned(elem.span()).into());
    } else if let Some(elem) = child.to_packed::<SmartQuoteElem>() {
        output.push(HtmlNode::text(
            if elem.double.get(styles) { '"' } else { '\'' },
            child.span(),
        ));
    } else if let Some(elem) = child.to_packed::<FrameElem>() {
        let locator = locator.next(&elem.span());
        let style = TargetElem::target.set(Target::Paged).wrap();
        let frame = (engine.routines.layout_frame)(
            engine,
            &elem.body,
            locator,
            styles.chain(&style),
            Region::new(Size::splat(Abs::inf()), Axes::splat(false)),
        )?;
        output.push(HtmlNode::Frame(HtmlFrame::new(frame, styles)));
    } else {
        engine.sink.warn(warning!(
            child.span(),
            "{} was ignored during HTML export",
            child.elem().name()
        ));
    }
    Ok(())
}

/// Checks whether the given element is an inline-level HTML element.
pub fn is_inline(elem: &Content) -> bool {
    elem.to_packed::<HtmlElem>()
        .is_some_and(|elem| tag::is_inline_by_default(elem.tag))
}
