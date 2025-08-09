use ecow::EcoVec;
use typst_library::diag::{SourceResult, warning};
use typst_library::engine::Engine;
use typst_library::foundations::{Content, StyleChain, Target, TargetElem};
use typst_library::introspection::{SplitLocator, TagElem};
use typst_library::layout::{Abs, Axes, Region, Size};
use typst_library::routines::Pair;
use typst_library::text::{
    LinebreakElem, SmartQuoteElem, SmartQuoter, SmartQuotes, SpaceElem, TextElem,
    is_default_ignorable,
};

use crate::fragment::{html_block_fragment, html_inline_fragment};
use crate::{FrameElem, HtmlElem, HtmlElement, HtmlFrame, HtmlNode, tag};

/// Converts realized content into HTML nodes.
pub fn convert_to_nodes<'a>(
    engine: &mut Engine,
    locator: &mut SplitLocator,
    quoter: &mut SmartQuoter,
    children: impl IntoIterator<Item = Pair<'a>>,
) -> SourceResult<EcoVec<HtmlNode>> {
    let mut output = EcoVec::new();
    for (child, styles) in children {
        handle(engine, child, locator, styles, quoter, &mut output)?;
    }
    Ok(output)
}

/// Convert one element into HTML node(s).
fn handle(
    engine: &mut Engine,
    child: &Content,
    locator: &mut SplitLocator,
    styles: StyleChain,
    quoter: &mut SmartQuoter,
    output: &mut EcoVec<HtmlNode>,
) -> SourceResult<()> {
    if let Some(elem) = child.to_packed::<TagElem>() {
        output.push(HtmlNode::Tag(elem.tag.clone()));
    } else if let Some(elem) = child.to_packed::<HtmlElem>() {
        let mut children = EcoVec::new();
        if let Some(body) = elem.body.get_ref(styles) {
            if tag::is_block_by_default(elem.tag) {
                children = html_block_fragment(
                    engine,
                    body,
                    locator.next(&elem.span()),
                    styles,
                )?;

                // Block-level elements reset the smart quoting state. This part
                // is unfortunately untested as it's currently not possible to
                // create inline-level content next to block-level content
                // without a paragraph automatically appearing.
                *quoter = SmartQuoter::new();
            } else {
                children = html_inline_fragment(engine, body, locator, quoter, styles)?;
            }
        }
        let element = HtmlElement {
            tag: elem.tag,
            attrs: elem.attrs.get_cloned(styles),
            children,
            span: elem.span(),
        };
        output.push(element.into());
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
        let double = elem.double.get(styles);
        if elem.enabled.get(styles) {
            let before = last_char(output);
            let quotes = SmartQuotes::get(
                elem.quotes.get_ref(styles),
                styles.get(TextElem::lang),
                styles.get(TextElem::region),
                elem.alternative.get(styles),
            );
            let quote = quoter.quote(before, &quotes, double);
            output.push(HtmlNode::text(quote, child.span()));
        } else {
            output.push(HtmlNode::text(if double { '"' } else { '\'' }, child.span()));
        }
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
        output.push(HtmlNode::Frame(HtmlFrame::new(frame, styles, elem.span())));
    } else {
        engine.sink.warn(warning!(
            child.span(),
            "{} was ignored during HTML export",
            child.elem().name()
        ));
    }
    Ok(())
}

/// Returns the last non-default ignorable character from the passed nodes.
fn last_char(nodes: &[HtmlNode]) -> Option<char> {
    for node in nodes.iter().rev() {
        if let Some(c) = match node {
            HtmlNode::Text(s, _) => s.chars().rev().find(|&c| !is_default_ignorable(c)),
            HtmlNode::Element(e) => last_char(&e.children),
            _ => None,
        } {
            return Some(c);
        }
    }
    None
}

/// Checks whether the given element is an inline-level HTML element.
pub fn is_inline(elem: &Content) -> bool {
    elem.to_packed::<HtmlElem>()
        .is_some_and(|elem| tag::is_inline_by_default(elem.tag))
}
