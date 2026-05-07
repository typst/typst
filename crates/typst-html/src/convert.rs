use ecow::{EcoString, EcoVec, eco_vec};
use typst_library::diag::{SourceResult, bail, warning};
use typst_library::engine::Engine;
use typst_library::foundations::{Content, Packed, StyleChain, Target, TargetElem};
use typst_library::introspection::{SplitLocator, TagElem};
use typst_library::layout::{
    Abs, Axes, BlockBody, BlockElem, BoxElem, HElem, Region, Size,
};
use typst_library::routines::Pair;
use typst_library::text::{
    LinebreakElem, SmartQuoteElem, SmartQuoter, SmartQuotes, SpaceElem, TextElem,
    is_default_ignorable,
};
use typst_syntax::Span;
use typst_utils::SliceExt;

use crate::fragment::{html_block_fragment, html_inline_fragment};
use crate::{
    FrameElem, HtmlElem, HtmlElement, HtmlFrame, HtmlNode, attr, css, property, tag,
};

/// What and how to convert.
pub enum ConversionLevel<'a> {
    /// Converts the top-level nodes or children of a block-level element. The
    /// conversion has its own local smart quoting state and space protection.
    Block,
    /// Converts the children of an inline-level HTML element as part of a
    /// larger context with shared smart quoting state and shared space
    /// protection.
    Inline(&'a mut SmartQuoter),
}

/// How to emit whitespace.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Whitespace {
    /// Ensures that whitespace that would otherwise be collapsed by HTML
    /// rendering engines[^1] is protected by spans with `white-space:
    /// pre-wrap`. The affected by whitespace are ASCII spaces and ASCII tabs.
    ///
    /// Tries to emit spans only when necessary.
    /// - ASCII tabs and consecutive sequences of spaces and/or tabs are always
    ///   wrapped in spans in this mode. This happens directly during
    ///   conversion.
    /// - Single ASCII spaces are only wrapped if they aren't supported by
    ///   normal elements on both sides. This happens in a separate pass that
    ///   runs for the whole block-level context as doing this properly needs
    ///   lookahead and lookbehind across different levels of the element
    ///   hierarchy.
    ///
    /// [^1]: https://www.w3.org/TR/css-text-3/#white-space-rules
    Normal,
    /// The whitespace is emitted as-is. This happens in
    /// - `<pre>` elements as they already have `white-space: pre`,
    /// - raw and escapable raw text elements as normal white space rules do not
    ///   apply to them.
    Pre,
}

/// Converts realized content into HTML nodes.
pub fn convert_to_nodes<'a>(
    engine: &mut Engine,
    locator: &mut SplitLocator,
    children: impl IntoIterator<Item = Pair<'a>>,
    level: ConversionLevel,
    whitespace: Whitespace,
) -> SourceResult<EcoVec<HtmlNode>> {
    let block = matches!(level, ConversionLevel::Block);
    let mut converter = Converter {
        engine,
        locator,
        quoter: match level {
            ConversionLevel::Inline(quoter) => quoter,
            ConversionLevel::Block => &mut SmartQuoter::new(),
        },
        whitespace,
        output: EcoVec::new(),
        trailing: None,
    };

    for (child, styles) in children {
        handle(&mut converter, child, styles)?;
    }

    let mut nodes = converter.finish();
    if block && whitespace == Whitespace::Normal {
        protect_spaces(&mut nodes);
    }

    Ok(nodes)
}

/// Converts one element into HTML node(s).
fn handle(
    converter: &mut Converter,
    child: &Content,
    styles: StyleChain,
) -> SourceResult<()> {
    if let Some(elem) = child.to_packed::<TagElem>() {
        converter.push(elem.tag.clone());
    } else if let Some(elem) = child.to_packed::<HtmlElem>() {
        handle_html_elem(converter, elem, styles)?;
    } else if child.is::<SpaceElem>() {
        converter.push(HtmlNode::text(' ', child.span()));
    } else if let Some(elem) = child.to_packed::<TextElem>() {
        let text = if let Some(case) = styles.get(TextElem::case) {
            case.apply(&elem.text).into()
        } else {
            elem.text.clone()
        };
        handle_text(converter, text, elem.span());
    } else if let Some(elem) = child.to_packed::<HElem>()
        && elem.amount.is_zero()
    {
        // Nothing to do for zero-sized spacing. This is sometimes used to
        // destruct spaces, e.g. in footnotes. See [`HElem::hole`].
    } else if let Some(elem) = child.to_packed::<LinebreakElem>() {
        converter.push(match converter.whitespace {
            Whitespace::Normal => HtmlElement::new(tag::br).spanned(elem.span()).into(),
            Whitespace::Pre => HtmlNode::text("\n", elem.span()),
        });
    } else if let Some(elem) = child.to_packed::<SmartQuoteElem>() {
        let double = elem.double.get(styles);
        let quote = if elem.enabled.get(styles) {
            let before = last_char(&converter.output);
            let quotes = SmartQuotes::get(
                elem.quotes.get_ref(styles),
                styles.get(TextElem::lang),
                styles.get(TextElem::region),
                elem.alternative.get(styles),
            );
            converter.quoter.quote(before, &quotes, double)
        } else {
            SmartQuotes::fallback(double)
        };
        handle_text(converter, quote.into(), child.span());
    } else if let Some(elem) = child.to_packed::<BoxElem>() {
        handle_box(converter, elem, styles)?;
    } else if let Some(elem) = child.to_packed::<BlockElem>() {
        handle_block(converter, elem, styles)?;
    } else if let Some(elem) = child.to_packed::<FrameElem>() {
        let locator = converter.locator.next(&elem.span());
        let style = TargetElem::target.set(Target::Paged).wrap();
        let frame = (converter.engine.library.routines.layout_frame)(
            converter.engine,
            &elem.body,
            locator,
            styles.chain(&style),
            Region::new(Size::splat(Abs::inf()), Axes::splat(false)),
        )?;
        let mut node = HtmlFrame::new(frame, styles, elem.span()).into();
        // A frame is block-level by default like a Typst `image`. It can
        // be wrapped in a `box` to omit the `display` annotation.
        make_block_level(&mut node).unwrap();
        converter.push(node);
    } else {
        converter.engine.sink.warn(warning!(
            child.span(),
            "{} was ignored during HTML export",
            child.elem().name(),
        ));
    }
    Ok(())
}

/// Handles an HTML element.
fn handle_html_elem(
    converter: &mut Converter,
    elem: &Packed<HtmlElem>,
    styles: StyleChain,
) -> SourceResult<()> {
    // See the docs of `HtmlElem::role` for why we filter out roles for `<p>`
    // elements.
    let role = styles.get_cloned(HtmlElem::role).filter(|_| elem.tag != tag::p);

    let mut children = EcoVec::new();
    if let Some(body) = elem.body.get_ref(styles) {
        let whitespace = if converter.whitespace == Whitespace::Pre
            || elem.tag == tag::pre
            || tag::is_raw(elem.tag)
            || tag::is_escapable_raw(elem.tag)
        {
            Whitespace::Pre
        } else {
            Whitespace::Normal
        };

        // The `role` attribute should only apply to the first element in the
        // hierarchy. Thus, we unset it for children if it is currently set.
        let unset;
        let styles = if role.is_some() {
            unset = HtmlElem::role.set(None).wrap();
            styles.chain(&unset)
        } else {
            styles
        };

        if property::Display::default_for(elem.tag) == Some(property::Display::Block) {
            children = html_block_fragment(
                converter.engine,
                body,
                converter.locator.next(&elem.span()),
                styles,
                whitespace,
            )?;

            // Block-level elements reset the inline state. This part is
            // unfortunately untested as it's currently not possible to
            // create inline-level content next to block-level content
            // without a paragraph automatically appearing.
            *converter.quoter = SmartQuoter::new();
        } else {
            children = html_inline_fragment(
                converter.engine,
                body,
                converter.locator,
                converter.quoter,
                styles,
                whitespace,
            )?;
        }
    }

    let mut attrs = elem.attrs.get_cloned(styles);
    if let Some(role) = role {
        attrs.push(attr::role, role);
    }

    converter.push(HtmlElement {
        tag: elem.tag,
        attrs,
        css: elem.css.get_cloned(styles),
        children,
        parent: elem.parent,
        span: elem.span(),
        pre_span: false,
    });

    Ok(())
}

/// Handles arbitrary text while taking care that no whitespace within will be
/// collapsed by browsers.
fn handle_text(converter: &mut Converter, text: EcoString, span: Span) {
    /// Special kinds of characters.
    #[derive(Debug, Copy, Clone, Eq, PartialEq)]
    enum Kind {
        /// ASCII space.
        Space,
        /// ASCII tab.
        Tab,
        /// CR, LF, or CR + LF.
        Newline,
        /// A Unicode default-ignorable. Does not protect spaces from
        /// collapsing.
        Ignorable,
    }

    impl Kind {
        fn of(c: char) -> Option<Kind> {
            match c {
                ' ' => Some(Kind::Space),
                '\t' => Some(Kind::Tab),
                '\r' | '\n' => Some(Kind::Newline),
                c if is_default_ignorable(c) => Some(Kind::Ignorable),
                _ => None,
            }
        }
    }

    if converter.whitespace == Whitespace::Pre {
        converter.push(HtmlNode::Text(text, span));
        return;
    }

    let mut emitted = 0;
    let mut prev_kind = None;

    for (i, c) in text.char_indices() {
        let kind = Kind::of(c);
        let prev_kind = prev_kind.replace(kind);
        let Some(kind) = kind else { continue };

        // A space that is surrounded by normal (i.e. not special) characters is
        // already protected and doesn't need further treatment.
        if kind == Kind::Space
            && let Some(None) = prev_kind
            && let Some(after) = text[i + 1..].chars().next()
            && Kind::of(after).is_none()
        {
            continue;
        }

        // Emit the unspecial text up to the special character.
        if emitted < i {
            converter.push_text(&text[emitted..i], span);
            emitted = i;
        }

        // Process the special character.
        match kind {
            Kind::Space => converter.push_text(' ', span),
            Kind::Tab => converter.push_text('\t', span),
            Kind::Newline => {
                if c == '\r' && text[i + 1..].starts_with('\n') {
                    // Skip the CR because the LF will already turn into
                    // a `<br>`.
                    emitted += 1;
                    continue;
                }
                converter.push(HtmlElement::new(tag::br).spanned(span));
            }
            Kind::Ignorable => converter.push_text(c, span),
        }
        emitted += c.len_utf8();
    }

    // Push the remaining unspecial text.
    if emitted < text.len() {
        converter.push_text(
            // Try to reuse the `EcoString` if possible.
            if emitted == 0 { text } else { text[emitted..].into() },
            span,
        );
    }
}

/// Processes a box element.
fn handle_box(
    converter: &mut Converter,
    elem: &Packed<BoxElem>,
    styles: StyleChain,
) -> SourceResult<()> {
    let mut children = EcoVec::new();
    if let Some(body) = elem.body.get_ref(styles) {
        children = html_inline_fragment(
            converter.engine,
            body,
            converter.locator,
            converter.quoter,
            styles,
            converter.whitespace,
        )?;

        if let Some(node) = to_lone_element(&mut children) {
            make_inline_level(node);
            converter.extend(children);
            return Ok(());
        }
    }

    converter.push(
        // TODO: This is rather incomplete.
        HtmlElement::new(tag::span)
            .with_css(css::Properties::new().with("display", "inline-block"))
            .with_children(children)
            .spanned(elem.span()),
    );

    Ok(())
}

/// Ensures a lone HTML element/frame renders inline.
///
/// Paragraphs only allow "Phrasing content". The default display properties
/// that exists for valid phrasing content are `none`, `inline`, `inline-block`,
/// `contents`, and `ruby`. These can all be left as-is.
///
/// If, however, the `display` property was set to something like `block`
/// through a Typst `block` element, we need to unset it.
fn make_inline_level(node: &mut HtmlNode) {
    set_display(node, None);
}

/// Processes a block element.
fn handle_block(
    converter: &mut Converter,
    elem: &Packed<BlockElem>,
    styles: StyleChain,
) -> SourceResult<()> {
    let body = match elem.body.get_ref(styles) {
        None => None,
        Some(BlockBody::Content(body)) => Some(body),
        // These are only generated by native `typst-layout` show rules.
        Some(BlockBody::SingleLayouter(_) | BlockBody::MultiLayouter(_)) => {
            bail!(
                elem.span(),
                "blocks with layout routines should not occur in \
                 HTML export – this is a bug";
            )
        }
    };

    let mut children = EcoVec::new();
    if let Some(body) = body {
        children = html_block_fragment(
            converter.engine,
            body,
            converter.locator.next(&elem.span()),
            styles,
            converter.whitespace,
        )?;

        if let Some(node) = to_lone_element(&mut children)
            && make_block_level(node).is_ok()
        {
            converter.extend(children);
            return Ok(());
        }
    }

    converter.push(
        // TODO: This is rather incomplete.
        HtmlElement::new(tag::div)
            .with_children(children)
            .spanned(elem.span()),
    );

    Ok(())
}

/// Ensures a lone HTML element/frame renders as its own block.
///
/// Configures the `display` property of the HTML node (if it is an element or
/// frame).
fn make_block_level(node: &mut HtmlNode) -> Result<(), Unblockable> {
    let default = match node {
        HtmlNode::Element(element) => property::Display::default_for(element.tag),
        HtmlNode::Frame(_) => Some(property::Display::Inline),
        _ => return Err(Unblockable),
    };

    let mode = match default {
        // These can be left unchanged / have `display` unset because they are
        // either invisible or already render in a block-level way.
        Some(
            property::Display::None
            | property::Display::Block
            | property::Display::Table
            | property::Display::ListItem
            | property::Display::Contents,
        ) => None,

        // These must be promoted to `block`.
        None | Some(property::Display::Inline | property::Display::InlineBlock) => {
            Some(property::Display::Block)
        }

        // These can't be promoted. They are instead wrapped in a `<div>`.
        _ => return Err(Unblockable),
    };

    set_display(node, mode);
    Ok(())
}

/// Indicates that an element can be made block-level with a `display` property.
#[derive(Debug, Copy, Clone)]
struct Unblockable;

/// Whether, apart from tags, the nodes comprise a single element or frame.
fn to_lone_element(nodes: &mut EcoVec<HtmlNode>) -> Option<&mut HtmlNode> {
    let (start, end) = nodes.split_prefix_suffix(|node| matches!(node, HtmlNode::Tag(_)));
    matches!(&nodes[start..end], [HtmlNode::Element(_) | HtmlNode::Frame(_)])
        .then(|| &mut nodes.make_mut()[start])
}

/// Sets the CSS `display` property for an element or frame.
fn set_display(node: &mut HtmlNode, display: Option<property::Display>) {
    let css = match node {
        HtmlNode::Element(element) => &mut element.css,
        HtmlNode::Frame(frame) => &mut frame.css,
        _ => return,
    };
    match display {
        Some(display) => css.push("display", display.as_str()),
        None => css.remove("display"),
    }
}

/// State during conversion.
struct Converter<'a, 'y, 'z> {
    engine: &'a mut Engine<'y>,
    locator: &'a mut SplitLocator<'z>,
    quoter: &'a mut SmartQuoter,
    whitespace: Whitespace,
    output: EcoVec<HtmlNode>,
    trailing: Option<TrailingWhitespace>,
}

/// Keeps track of a trailing whitespace in the output.
struct TrailingWhitespace {
    /// If `true`, the trailing whitespace consists of exactly one ASCII space.
    single: bool,
    /// The trailing whitespace starts at `output[from..]`.
    from: usize,
}

impl Converter<'_, '_, '_> {
    /// Returns the converted nodes.
    fn finish(mut self) -> EcoVec<HtmlNode> {
        self.flush_whitespace();
        self.output
    }

    /// Pushes a node, taking care to protect consecutive whitespace.
    fn push(&mut self, node: impl Into<HtmlNode>) {
        let node = node.into();

        if let HtmlNode::Text(text, _) = &node
            && (text == " " || text == "\t")
        {
            if let Some(ws) = &mut self.trailing {
                ws.single = false;
            } else {
                self.trailing = Some(TrailingWhitespace {
                    single: text == " ",
                    from: self.output.len(),
                });
            }
        } else if !matches!(node, HtmlNode::Tag(_)) {
            self.flush_whitespace();
        }

        self.output.push(node);
    }

    /// Pushes multiples nodes.
    fn extend(&mut self, nodes: impl IntoIterator<Item = HtmlNode>) {
        for node in nodes {
            self.push(node);
        }
    }

    /// Shorthand for pushing a text node.
    fn push_text(&mut self, text: impl Into<EcoString>, span: Span) {
        self.push(HtmlNode::text(text.into(), span));
    }

    /// If there is trailing whitespace in need of protection, protects it.
    ///
    /// Does not protect single ASCII spaces. Those are handled in a separate
    /// pass as they are more complex and require lookahead. See the
    /// documentation of [`Whitespace`] for more information.
    fn flush_whitespace(&mut self) {
        if self.whitespace == Whitespace::Normal
            && let Some(TrailingWhitespace { single: false, from }) = self.trailing.take()
        {
            let nodes: EcoVec<_> = self.output[from..].iter().cloned().collect();
            self.output.truncate(from);
            self.output.push(HtmlNode::Element(pre_wrap(nodes)));
        }
    }
}

/// Protects all spaces in the given block-level `nodes` against collapsing.
///
/// Does not recurse into block-level elements as those are separate contexts
/// with their own space protection.
fn protect_spaces(nodes: &mut EcoVec<HtmlNode>) {
    let mut p = Protector::new();
    p.visit_nodes(nodes);
    p.collapsing();
}

/// A state machine for whitespace protection.
enum Protector<'a> {
    Collapsing,
    Supportive,
    Space(&'a mut HtmlNode),
}

impl<'a> Protector<'a> {
    /// Creates a new protector.
    fn new() -> Self {
        Self::Collapsing
    }

    /// Visits the given nodes and protects single spaces that need to be saved
    /// from collapsing.
    fn visit_nodes(&mut self, nodes: &'a mut EcoVec<HtmlNode>) {
        for node in nodes.make_mut().iter_mut() {
            match node {
                HtmlNode::Tag(_) => {}
                HtmlNode::Text(text, _) => {
                    if text == " " {
                        match self {
                            Self::Collapsing => {
                                protect_space(node);
                                *self = Self::Supportive;
                            }
                            Self::Supportive => {
                                *self = Self::Space(node);
                            }
                            Self::Space(prev) => {
                                protect_space(prev);
                                *self = Self::Space(node);
                            }
                        }
                    } else if text.chars().any(|c| !is_default_ignorable(c)) {
                        self.supportive();
                    }
                }
                HtmlNode::Element(element) => {
                    if tag::is_whitespace_collapsing(element.tag) {
                        self.collapsing();
                    } else if tag::is_replaced(element.tag) {
                        self.supportive();
                    } else if !element.pre_span {
                        // Recursively visit the children of inline-level
                        // elements while making sure to not revisit pre-wrapped
                        // spans that we've generated ourselves.
                        self.visit_nodes(&mut element.children);
                    }
                }
                HtmlNode::Frame(_) => self.supportive(),
            }
        }
    }

    /// Called when visiting an element that would collapse adjacent single
    /// spaces. A preceding, if any, and succeeding, if any, single space will
    /// then be protected .
    fn collapsing(&mut self) {
        if let Self::Space(node) = std::mem::replace(self, Self::Collapsing) {
            protect_space(node);
        }
    }

    /// Called when visiting an element that supports adjacent single spaces.
    fn supportive(&mut self) {
        *self = Self::Supportive;
    }
}

/// Protects a single spaces against collapsing.
fn protect_space(node: &mut HtmlNode) {
    *node = pre_wrap(eco_vec![node.clone()]).into();
}

/// Wraps a collection of whitespace nodes in a
/// `<span style="white-space: pre-wrap">..</span>` to avoid them being
/// collapsed by HTML rendering engines.
fn pre_wrap(nodes: EcoVec<HtmlNode>) -> HtmlElement {
    let span = Span::find(nodes.iter().map(|c| c.span()));
    let mut elem = HtmlElement::new(tag::span)
        .with_css(css::Properties::new().with("white-space", "pre-wrap"))
        .with_children(nodes)
        .spanned(span);
    elem.pre_span = true;
    elem
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
