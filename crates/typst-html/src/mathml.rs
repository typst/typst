use ecow::{EcoVec, eco_format, eco_vec};
use typst_assets::mathml::*;
use typst_library::diag::{SourceResult, warning};
use typst_library::engine::Engine;
use typst_library::foundations::{Packed, StyleChain};
use typst_library::introspection::Locator;
use typst_library::layout::Axis;
use typst_library::math::*;
use typst_library::routines::Arenas;
use unicode_math_class::MathClass;

use crate::tag::mathml as tag;
use crate::{HtmlElement, HtmlNode};
use crate::{attr::mathml as attr, css};

pub(crate) fn convert_math_to_nodes(
    elem: &Packed<EquationElem>,
    engine: &mut Engine,
    locator: Locator,
    styles: StyleChain,
) -> SourceResult<EcoVec<HtmlNode>> {
    let mut locator = locator.split();

    let arenas = Arenas::default();
    let item = resolve_equation(elem, engine, &mut locator, &arenas, styles)?;

    let mut ctx = MathContext::new(engine);
    let nodes = ctx.handle_into_nodes(&item, styles)?;
    Ok(nodes)
}

trait HtmlNodesExt {
    fn into_node(self) -> HtmlNode;
}

impl HtmlNodesExt for EcoVec<HtmlNode> {
    fn into_node(mut self) -> HtmlNode {
        if self.len() == 1 {
            self.pop().unwrap()
        } else {
            HtmlElement::new(tag::mrow).with_children(self).into()
        }
    }
}

enum NodePosition {
    Start,
    Middle,
    End,
    Only(Option<Form>),
}

impl NodePosition {
    fn get_form(&self) -> Form {
        match self {
            Self::Start => Form::Prefix,
            Self::Middle => Form::Infix,
            Self::End => Form::Postfix,
            Self::Only(form) => form.unwrap_or(Form::Infix),
        }
    }
}

/// The context for math handling.
struct MathContext<'v, 'e> {
    engine: &'v mut Engine<'e>,
    nodes: EcoVec<HtmlNode>,
}

impl<'v, 'e> MathContext<'v, 'e> {
    /// Create a new math context.
    fn new(engine: &'v mut Engine<'e>) -> Self {
        Self { engine, nodes: EcoVec::new() }
    }

    /// Push a node.
    fn push(&mut self, node: impl Into<HtmlNode>) {
        self.nodes.push(node.into());
    }

    /// Handle the given element and return the resulting [`HtmlNode`]s.
    fn handle_into_nodes_with_only_form(
        &mut self,
        item: &MathItem,
        styles: StyleChain,
        only: Option<Form>,
    ) -> SourceResult<EcoVec<HtmlNode>> {
        let prev = std::mem::take(&mut self.nodes);
        self.handle_into_self(item, styles, only)?;
        Ok(std::mem::replace(&mut self.nodes, prev))
    }

    /// Handle the given element and return the resulting [`HtmlNode`]s.
    fn handle_into_nodes(
        &mut self,
        item: &MathItem,
        styles: StyleChain,
    ) -> SourceResult<EcoVec<HtmlNode>> {
        self.handle_into_nodes_with_only_form(item, styles, None)
    }

    /// Handle the given element and return the resulting [`HtmlNode`]s.
    fn handle_into_node(
        &mut self,
        item: &MathItem,
        styles: StyleChain,
    ) -> SourceResult<HtmlNode> {
        Ok(self.handle_into_nodes(item, styles)?.into_node())
    }

    /// Handle the given element and return the resulting [`HtmlNode`]s.
    fn handle_into_node_with_only_form(
        &mut self,
        item: &MathItem,
        styles: StyleChain,
        only: Form,
    ) -> SourceResult<HtmlNode> {
        Ok(self
            .handle_into_nodes_with_only_form(item, styles, Some(only))?
            .into_node())
    }

    fn handle_into_self(
        &mut self,
        item: &MathItem,
        styles: StyleChain,
        only: Option<Form>,
    ) -> SourceResult<()> {
        let outer_styles = item.styles().unwrap_or(styles);

        let items = item.as_slice();
        let len = items.len();
        for (i, item) in items.iter().enumerate() {
            let styles = item.styles().unwrap_or(outer_styles);

            let position = if len == 1 {
                NodePosition::Only(only)
            } else if i == 0 {
                NodePosition::Start
            } else if i == len - 1 {
                NodePosition::End
            } else {
                NodePosition::Middle
            };
            handle_realized(item, self, styles, position)?;
        }

        Ok(())
    }
}

/// Handles a leaf element resulting from realization.
fn handle_realized(
    item: &MathItem,
    ctx: &mut MathContext,
    styles: StyleChain,
    position: NodePosition,
) -> SourceResult<()> {
    let MathItem::Component(comp) = item else {
        match item {
            MathItem::Spacing(amount, _) => ctx.push(
                HtmlElement::new(tag::mspace)
                    .with_attr(attr::width, eco_format!("{}", css::length(*amount))),
            ),
            MathItem::Space => {}
            MathItem::Linebreak => {}
            MathItem::Align => {}
            MathItem::Tag(tag) => ctx.push(HtmlNode::Tag(tag.clone())),
            _ => unreachable!(),
        }
        return Ok(());
    };

    let props = &comp.props;

    // TODO: deal with lspace/rspace for non-Glyph items.
    // if let Some(lspace) = props.lspace {
    //     // TODO: use more accurate text size.
    //     let width = lspace.at(styles.resolve(TextElem::size));
    //     let frag = MathFragment::Space(width);
    //     if ctx.fragments.last().is_some_and(|x| matches!(x, MathFragment::Align)) {
    //         ctx.fragments.insert(ctx.fragments.len() - 1, frag);
    //     } else {
    //         ctx.push(frag);
    //     }
    // }

    match &comp.kind {
        MathKind::Glyph(item) => handle_glyph(item, ctx, styles, props, position)?,
        MathKind::Radical(item) => handle_radical(item, ctx, styles, props)?,
        MathKind::Accent(item) => handle_accent(item, ctx, styles, props)?,
        MathKind::Scripts(item) => handle_scripts(item, ctx, styles, props)?,
        MathKind::Primes(_item) => {}
        MathKind::Table(item) => handle_table(item, ctx, styles, props)?,
        MathKind::Fraction(item) => handle_fraction(item, ctx, styles, props)?,
        MathKind::Text(item) => handle_text(item, ctx, styles, props)?,
        MathKind::Fenced(item) => handle_fenced(item, ctx, styles, props)?,
        MathKind::Group(_item) => {
            // let fragment = ctx.handle_into_fragment(&item.items)?;
            // let italics = fragment.italics_correction();
            // let accent_attach = fragment.accent_attach();
            // ctx.push(
            //     FrameFragment::new(props, fragment.into_frame())
            //         .with_italics_correction(italics)
            //         .with_accent_attach(accent_attach),
            // );
        }

        // Polyfill required for MathML Core.
        MathKind::SkewedFraction(_) | MathKind::Cancel(_) | MathKind::Line(_) => {}

        // Arbitrary content is not allowed, as per the HTML spec.
        // Only MathML token elements (mi, mo, mn, ms, and mtext),
        // when descendants of HTML elements, may contain
        // [phrasing content] from the HTML namespace.
        // https://html.spec.whatwg.org/#phrasing-content-2
        //
        // In MathML 3, nothing is allowed except MathML elements.
        // Further, nesting root-level math elements is disallowed.
        //
        // Since the math element is considered phrasing content,
        // it can be nested in MathML Core (as long as it is itself
        // within a MathML token element).
        MathKind::Box(_) => {}
        MathKind::External(item) => {
            ctx.engine.sink.warn(warning!(
                props.span,
                "{} was ignored during MathML export",
                item.content.elem().name()
            ));
        }
    }

    // if let Some(rspace) = props.rspace {
    //     // TODO: use more accurate text size.
    //     let width = rspace.at(styles.resolve(TextElem::size));
    //     ctx.push(MathFragment::Space(width));
    // }

    Ok(())
}

fn handle_fraction(
    item: &FractionItem,
    ctx: &mut MathContext,
    styles: StyleChain,
    _props: &MathProperties,
) -> SourceResult<()> {
    let num = ctx.handle_into_node(&item.numerator, styles)?;
    let denom = ctx.handle_into_node(&item.denominator, styles)?;
    let line = (!item.line).then_some("0");
    ctx.push(
        HtmlElement::new(tag::mfrac)
            .with_children(eco_vec![num, denom])
            .with_optional_attr(attr::linethickness, line),
    );
    Ok(())
}

fn handle_radical(
    item: &RadicalItem,
    ctx: &mut MathContext,
    styles: StyleChain,
    _props: &MathProperties,
) -> SourceResult<()> {
    let radicand = ctx.handle_into_nodes(&item.radicand, styles)?;
    let index = item
        .index
        .as_ref()
        .map(|index| ctx.handle_into_node(index, styles))
        .transpose()?;
    let (tag, children) = if let Some(index) = index {
        (tag::mroot, eco_vec![radicand.into_node(), index])
    } else {
        (tag::msqrt, radicand)
    };
    ctx.push(HtmlElement::new(tag).with_children(children));
    Ok(())
}

fn handle_glyph(
    item: &GlyphItem,
    ctx: &mut MathContext,
    _styles: StyleChain,
    props: &MathProperties,
    position: NodePosition,
) -> SourceResult<()> {
    let text = &item.text;

    let mut form = None;
    let mut fence = false;
    let mut separator = false;
    let mut largeop = false;
    let tag = match props.class {
        MathClass::Normal
        | MathClass::Alphabetic
        | MathClass::Special
        | MathClass::GlyphPart
        | MathClass::Space => {
            let mathvariant = will_auto_transform(text).then_some("normal");
            ctx.push(
                HtmlElement::new(tag::mi)
                    .with_children(eco_vec![HtmlNode::text(text, props.span)])
                    .with_optional_attr(attr::mathvariant, mathvariant),
            );
            return Ok(());
        }
        MathClass::Relation => tag::mo,
        MathClass::Diacritic => {
            form = Some(Form::Postfix);
            tag::mo
        }
        MathClass::Binary => {
            form = Some(Form::Infix);
            tag::mo
        }
        MathClass::Vary | MathClass::Unary => {
            form = Some(Form::Prefix);
            tag::mo
        }
        MathClass::Punctuation => {
            separator = true;
            tag::mo
        }
        MathClass::Fence => {
            fence = true;
            tag::mo
        }
        MathClass::Large => {
            largeop = true;
            tag::mo
        }
        MathClass::Opening => {
            fence = true;
            form = Some(Form::Prefix);
            tag::mo
        }
        MathClass::Closing => {
            fence = true;
            form = Some(Form::Postfix);
            tag::mo
        }
    };

    let initial_form = position.get_form();
    let info = get_operator_info(text, form.unwrap_or(initial_form));
    let form = form.filter(|f| *f != initial_form).map(|f| match f {
        Form::Prefix => "prefix",
        Form::Infix => "infix",
        Form::Postfix => "postfix",
    });

    let lspace = (props.lspace.unwrap_or_default().get() != info.lspace
        && !matches!(position, NodePosition::Only(_)))
    .then(|| eco_format!("{}em", props.lspace.unwrap_or_default().get()));
    let rspace = (props.rspace.unwrap_or_default().get() != info.rspace
        && !matches!(position, NodePosition::Only(_)))
    .then(|| eco_format!("{}em", props.rspace.unwrap_or_default().get()));

    let fence = (fence != is_fence(text)).then(|| eco_format!("{}", fence));
    let separator =
        (separator != is_separator(text)).then(|| eco_format!("{}", separator));

    let largeop = (largeop != info.properties.contains(Properties::LARGEOP))
        .then(|| eco_format!("{}", largeop));
    // We don't use movablelimits as we handle the positioning ourselves.
    let movablelimits = (info.properties.contains(Properties::MOVABLELIMITS))
        .then(|| eco_format!("false"));

    // TODO: symmetric, minsize, maxsize
    // surpress stretchy with `largeop`
    let mut chars = text.chars();
    let stretch_axis = if let Some(c) = chars.next()
        && chars.next().is_none()
        && stretch_axis_is_inline(c)
    {
        Axis::X
    } else {
        Axis::Y
    };
    let stretch = item.stretch.get().resolve(stretch_axis);
    let stretchy = (stretch.is_some() != info.properties.contains(Properties::STRETCHY))
        .then(|| eco_format!("{}", stretch.is_some()));

    ctx.push(
        HtmlElement::new(tag)
            .with_children(eco_vec![HtmlNode::text(text, props.span)])
            .with_optional_attr(attr::form, form)
            .with_optional_attr(attr::lspace, lspace)
            .with_optional_attr(attr::rspace, rspace)
            .with_optional_attr(attr::fence, fence)
            .with_optional_attr(attr::separator, separator)
            .with_optional_attr(attr::largeop, largeop)
            .with_optional_attr(attr::movablelimits, movablelimits)
            .with_optional_attr(attr::stretchy, stretchy),
    );
    Ok(())
}

fn handle_accent(
    item: &AccentItem,
    ctx: &mut MathContext,
    styles: StyleChain,
    _props: &MathProperties,
) -> SourceResult<()> {
    let (tag, attr) = if item.is_bottom {
        (tag::munder, attr::accentunder)
    } else {
        (tag::mover, attr::accent)
    };

    let base = ctx.handle_into_node(&item.base, styles)?;

    // TODO: maybe only add this if the base is an i or j, or only add when disabling.
    // let dtls = if elem.dotless.get(styles) { "on" } else { "off" };
    // if !accent.is_bottom() {
    //     base.to_packed_mut::<HtmlElem>().unwrap().push_attr(
    //         crate::attr::style,
    //         eco_format!("font-feature-settings: 'dtls' {};", dtls),
    //     );
    // }

    // TODO: convert accent char to non-combining, then lookup with postfix (or infix?) form for stretchy
    // Should surpress "Text run starts with a composing character" warnings from validator.
    let accent =
        ctx.handle_into_node_with_only_form(&item.accent, styles, Form::Postfix)?;

    ctx.push(
        HtmlElement::new(tag)
            .with_children(eco_vec![base, accent])
            .with_attr(attr, "true"),
    );
    Ok(())
}

fn handle_scripts(
    item: &ScriptsItem,
    ctx: &mut MathContext,
    styles: StyleChain,
    _props: &MathProperties,
) -> SourceResult<()> {
    let mut base = ctx.handle_into_node(&item.base, styles)?;

    macro_rules! handle {
        ($content:ident) => {
            item.$content
                .as_ref()
                .map(|x| ctx.handle_into_node_with_only_form(x, styles, Form::Postfix))
                .transpose()?
        };
    }

    let t = handle!(top);
    let tr = handle!(top_right);
    let tl = handle!(top_left);
    let b = handle!(bottom);
    let br = handle!(bottom_right);
    let bl = handle!(bottom_left);

    if let Some((tag, other_children)) = match (tl, tr, bl, br) {
        (None, None, None, None) => None,
        (None, None, None, Some(br)) => Some((tag::msub, eco_vec![br])),
        (None, Some(tr), None, None) => Some((tag::msup, eco_vec![tr])),
        (None, Some(tr), None, Some(br)) => Some((tag::msubsup, eco_vec![br, tr])),
        (tl, tr, bl, br) => {
            let unwrap = |node: Option<HtmlNode>| {
                node.unwrap_or(HtmlElement::new(tag::mrow).into())
            };
            Some((
                tag::mmultiscripts,
                eco_vec![
                    unwrap(br),
                    unwrap(tr),
                    HtmlElement::new(tag::mprescripts).into(),
                    unwrap(bl),
                    unwrap(tl),
                ],
            ))
        }
    } {
        let mut children = eco_vec![base];
        children.extend(other_children);
        base = HtmlElement::new(tag).with_children(children).into();
    }

    if let Some((tag, other_children)) = match (t, b) {
        (None, None) => None,
        (Some(t), None) => Some((tag::mover, eco_vec![t])),
        (None, Some(b)) => Some((tag::munder, eco_vec![b])),
        (Some(t), Some(b)) => Some((tag::munderover, eco_vec![b, t])),
    } {
        let mut children = eco_vec![base];
        children.extend(other_children);
        base = HtmlElement::new(tag).with_children(children).into();
    }

    ctx.push(base);
    Ok(())
}

fn handle_table(
    item: &TableItem,
    ctx: &mut MathContext,
    styles: StyleChain,
    _props: &MathProperties,
) -> SourceResult<()> {
    let cells = item
        .cells
        .iter()
        .map(|row| {
            let cell_nodes = row
                .iter()
                .map(|cell| {
                    // let props =
                    //     css::Properties::new().with("padding", "0em 0.2em");
                    Ok(HtmlElement::new(tag::mtd)
                        .with_children(ctx.handle_into_nodes(cell, styles)?)
                        // .with_styles(props)
                        .into())
                })
                .collect::<SourceResult<EcoVec<HtmlNode>>>();

            cell_nodes.map(|nodes| HtmlElement::new(tag::mtr).with_children(nodes).into())
        })
        .collect::<SourceResult<EcoVec<HtmlNode>>>()?;
    ctx.push(HtmlElement::new(tag::mtable).with_children(cells));
    Ok(())
}

fn handle_text(
    item: &TextItem,
    ctx: &mut MathContext,
    _styles: StyleChain,
    props: &MathProperties,
) -> SourceResult<()> {
    let tag = if item.text.chars().all(|c| c.is_ascii_digit() || c == '.') {
        tag::mn
    } else {
        tag::mtext
    };

    ctx.push(
        HtmlElement::new(tag)
            .with_children(eco_vec![HtmlNode::Text(item.text.into(), props.span)]),
    );
    Ok(())
}

fn handle_fenced(
    item: &FencedItem,
    ctx: &mut MathContext,
    styles: StyleChain,
    _props: &MathProperties,
) -> SourceResult<()> {
    let mut children = EcoVec::new();
    if let Some(open) = item
        .open
        .as_ref()
        .map(|x| ctx.handle_into_node_with_only_form(x, styles, Form::Prefix))
        .transpose()?
    {
        children.push(open);
    }
    let body = ctx.handle_into_node(&item.body, styles)?;
    children.push(body);
    if let Some(close) = item
        .close
        .as_ref()
        .map(|x| ctx.handle_into_node_with_only_form(x, styles, Form::Postfix))
        .transpose()?
    {
        children.push(close);
    }

    ctx.push(HtmlElement::new(tag::mrow).with_children(children));
    Ok(())
}
