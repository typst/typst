use ecow::{EcoVec, eco_format, eco_vec};
use typst_assets::mathml::*;
use typst_library::diag::{SourceResult, warning};
use typst_library::engine::Engine;
use typst_library::foundations::{Packed, StyleChain};
use typst_library::introspection::Locator;
use typst_library::math::*;
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
    let resolver = MathResolver::new();
    let run = resolver.resolve(elem, engine, &mut locator, styles)?;

    let mut ctx = MathContext::new(engine);
    let nodes = ctx.handle_into_nodes(&run)?;
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
    fn handle_into_nodes(&mut self, run: &MathRun) -> SourceResult<EcoVec<HtmlNode>> {
        let prev = std::mem::take(&mut self.nodes);
        self.handle_into_self(run, run.styles())?;
        Ok(std::mem::replace(&mut self.nodes, prev))
    }

    /// Handle the given element and return the resulting [`HtmlNode`]s.
    fn handle_into_node(&mut self, run: &MathRun) -> SourceResult<HtmlNode> {
        Ok(self.handle_into_nodes(run)?.into_node())
    }

    fn handle_into_self(
        &mut self,
        run: &MathRun,
        styles: StyleChain,
    ) -> SourceResult<()> {
        let outer_styles = styles;
        for item in run.iter() {
            let styles = item.styles().unwrap_or(outer_styles);
            handle_realized(item, self, styles)?;
        }
        Ok(())
    }
}

/// Handles a leaf element resulting from realization.
fn handle_realized(
    item: &MathItem,
    ctx: &mut MathContext,
    _styles: StyleChain,
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
        MathKind::Glyph(item) => handle_glyph(item, ctx, props)?,
        MathKind::Radical(item) => handle_radical(item, ctx, props)?,
        MathKind::Accent(item) => handle_accent(item, ctx, props)?,
        MathKind::Scripts(item) => handle_scripts(item, ctx, props)?,
        MathKind::Primes(_item) => {}
        MathKind::Table(item) => handle_table(item, ctx, props)?,
        MathKind::Fraction(item) => handle_fraction(item, ctx, props)?,
        MathKind::Text(item) => handle_text(item, ctx, props)?,
        MathKind::Fenced(item) => handle_fenced(item, ctx, props)?,
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
    _props: &MathProperties,
) -> SourceResult<()> {
    let num = ctx.handle_into_node(&item.numerator)?;
    let denom = ctx.handle_into_node(&item.denominator)?;
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
    _props: &MathProperties,
) -> SourceResult<()> {
    let radicand = ctx.handle_into_nodes(&item.radicand)?;
    let index = item
        .index
        .as_ref()
        .map(|index| ctx.handle_into_node(index))
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
    props: &MathProperties,
) -> SourceResult<()> {
    let initial_form = Form::Infix;
    let text = &item.text;

    let mut form = None;
    let mut fence = false;
    let mut separator = false;
    let mut largeop = false;
    // TODO: lspace, rspace
    let tag = match props.class {
        MathClass::Normal
        | MathClass::Alphabetic
        | MathClass::Special
        | MathClass::GlyphPart
        | MathClass::Space => tag::mi,
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

    let info = get_operator_info(text, form.unwrap_or(initial_form));
    let form = form.filter(|f| *f != initial_form).map(|f| match f {
        Form::Prefix => "prefix",
        Form::Infix => "infix",
        Form::Postfix => "postfix",
    });

    let fence = (fence != is_fence(text)).then(|| eco_format!("{}", fence));
    let separator =
        (separator != is_separator(text)).then(|| eco_format!("{}", separator));

    let largeop = (largeop != info.properties.contains(Properties::LARGEOP))
        .then(|| eco_format!("{}", largeop));
    // We don't use movablelimits as we handle the positioning ourselves.
    let movablelimits = (info.properties.contains(Properties::MOVABLELIMITS))
        .then(|| eco_format!("false"));

    // TODO: symmetric, minsize, maxsize
    let stretchy = item.stretch.is_some();
    let stretchy = (stretchy != info.properties.contains(Properties::STRETCHY))
        .then(|| eco_format!("{}", stretchy));

    let mathvariant = will_auto_transform(text).then_some("normal");

    ctx.push(
        HtmlElement::new(tag)
            .with_children(eco_vec![HtmlNode::text(text.clone(), props.span)])
            .with_optional_attr(attr::form, form)
            .with_optional_attr(attr::fence, fence)
            .with_optional_attr(attr::separator, separator)
            .with_optional_attr(attr::largeop, largeop)
            .with_optional_attr(attr::movablelimits, movablelimits)
            .with_optional_attr(attr::stretchy, stretchy)
            .with_optional_attr(attr::mathvariant, mathvariant),
    );
    Ok(())
}

fn handle_accent(
    item: &AccentItem,
    ctx: &mut MathContext,
    _props: &MathProperties,
) -> SourceResult<()> {
    let (tag, attr) = if item.is_bottom {
        (tag::munder, attr::accentunder)
    } else {
        (tag::mover, attr::accent)
    };

    let base = ctx.handle_into_node(&item.base)?;

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
    let accent = ctx.handle_into_node(&item.accent)?;

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
    _props: &MathProperties,
) -> SourceResult<()> {
    let mut base = ctx.handle_into_node(&item.base)?;

    macro_rules! handle {
        ($content:ident) => {
            item.$content.as_ref().map(|x| ctx.handle_into_node(x)).transpose()?
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
                        .with_children(ctx.handle_into_nodes(cell)?)
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
    props: &MathProperties,
) -> SourceResult<()> {
    let tag = if item.text.chars().all(|c| c.is_ascii_digit() || c == '.') {
        tag::mn
    } else {
        tag::mtext
    };

    ctx.push(
        HtmlElement::new(tag)
            .with_children(eco_vec![HtmlNode::Text(item.text.clone(), props.span)]),
    );
    Ok(())
}

fn handle_fenced(
    item: &FencedItem,
    ctx: &mut MathContext,
    _props: &MathProperties,
) -> SourceResult<()> {
    let mut children = EcoVec::new();
    if let Some(open) = item.open.as_ref().map(|x| ctx.handle_into_node(x)).transpose()? {
        children.push(open);
    }
    let body = ctx.handle_into_node(&item.body)?;
    children.push(body);
    if let Some(close) =
        item.close.as_ref().map(|x| ctx.handle_into_node(x)).transpose()?
    {
        children.push(close);
    }

    ctx.push(HtmlElement::new(tag::mrow).with_children(children));
    Ok(())
}
