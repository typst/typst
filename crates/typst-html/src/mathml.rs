use ecow::{EcoVec, eco_format, eco_vec};
use typst_assets::mathml::*;
use typst_library::diag::{SourceResult, warning};
use typst_library::engine::Engine;
use typst_library::layout::{Axis, Em};
use typst_library::math::ir::{
    AccentItem, FencedItem, FractionItem, GlyphItem, MathItem, MathKind, MathProperties,
    MultilineItem, NumberItem, Position, RadicalItem, ScriptsItem, Stretch, TableItem,
    TextItem,
};
use typst_syntax::Span;
use typst_utils::Numeric;
use unicode_math_class::MathClass;

use crate::tag::mathml as tag;
use crate::{HtmlElement, HtmlNode};
use crate::{attr::mathml as attr, css};

pub(crate) const MULTILINE_EQUATION_CLASS: &str = "multiline-equation";
pub(crate) const ALIGNED_EQUATION_CLASS: &str = "multiline-equation aligned-equation";

// TODO: vertical spacing between rows.
pub(crate) const EQUATION_CSS_STYLES: &str = "\
mtable.multiline-equation {
  math-style: unset;
}

mtable.multiline-equation mtd {
  padding: 0;
}

mtable.multiline-equation.aligned-equation mtd:nth-child(odd) {
  text-align: -webkit-right;
}

mtable.multiline-equation.aligned-equation mtd:nth-child(even) {
  text-align: -webkit-left;
}";

pub(crate) fn convert_math_to_nodes(
    item: MathItem,
    engine: &mut Engine,
) -> SourceResult<EcoVec<HtmlNode>> {
    let mut ctx = MathContext::new(engine);
    ctx.handle_into_nodes(&item)
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

#[derive(Copy, Clone)]
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

struct EmbellishmentContext {
    lspace: Option<Em>,
    rspace: Option<Em>,
    position: NodePosition,
}

/// The context for math handling.
struct MathContext<'v, 'e> {
    engine: &'v mut Engine<'e>,
    nodes: EcoVec<HtmlNode>,
    embellishment: Option<EmbellishmentContext>,
}

impl<'v, 'e> MathContext<'v, 'e> {
    /// Create a new math context.
    fn new(engine: &'v mut Engine<'e>) -> Self {
        Self { engine, nodes: EcoVec::new(), embellishment: None }
    }

    /// Push a node.
    fn push(&mut self, node: impl Into<HtmlNode>) {
        self.nodes.push(node.into());
    }

    /// Handle the given element and return the resulting [`HtmlNode`]s.
    fn handle_into_nodes_with_only_form(
        &mut self,
        item: &MathItem,
        only: Option<Form>,
    ) -> SourceResult<EcoVec<HtmlNode>> {
        let prev = std::mem::take(&mut self.nodes);
        self.handle_into_self(item, only)?;
        Ok(std::mem::replace(&mut self.nodes, prev))
    }

    /// Handle the given element and return the resulting [`HtmlNode`]s.
    fn handle_into_nodes(&mut self, item: &MathItem) -> SourceResult<EcoVec<HtmlNode>> {
        self.handle_into_nodes_with_only_form(item, None)
    }

    /// Handle the given element and return the resulting [`HtmlNode`]s.
    fn handle_into_node(&mut self, item: &MathItem) -> SourceResult<HtmlNode> {
        Ok(self.handle_into_nodes(item)?.into_node())
    }

    /// Handle the given element and return the resulting [`HtmlNode`]s.
    fn handle_into_node_with_only_form(
        &mut self,
        item: &MathItem,
        only: Form,
    ) -> SourceResult<HtmlNode> {
        Ok(self.handle_into_nodes_with_only_form(item, Some(only))?.into_node())
    }

    fn handle_into_self(
        &mut self,
        item: &MathItem,
        only: Option<Form>,
    ) -> SourceResult<()> {
        let items = item.as_slice();
        let len = items.len();
        for (i, item) in items.iter().enumerate() {
            let position = if len == 1 {
                NodePosition::Only(only)
            } else if i == 0 {
                NodePosition::Start
            } else if i == len - 1 {
                NodePosition::End
            } else {
                NodePosition::Middle
            };
            handle_realized(item, self, position)?;
        }

        Ok(())
    }
}

/// Handles a leaf element resulting from realization.
fn handle_realized(
    item: &MathItem,
    ctx: &mut MathContext,
    position: NodePosition,
) -> SourceResult<()> {
    // Handle non-component items first.
    let comp = match item {
        MathItem::Component(comp) => comp,
        MathItem::Spacing(amount, _, _) => {
            ctx.push(
                HtmlElement::new(tag::mspace)
                    .with_attr(attr::width, eco_format!("{}", css::length(*amount))),
            );
            return Ok(());
        }
        MathItem::Space => {
            ctx.push(HtmlElement::new(tag::mspace).with_attr(
                attr::width,
                eco_format!("{}", css::length(Em::new(0.2222222222222222))),
            ));
            return Ok(());
        }
        MathItem::Tag(tag) => {
            ctx.push(HtmlNode::Tag(tag.clone()));
            return Ok(());
        }
    };

    let props = &comp.props;
    let embellished = is_embellished_operator(item);

    // Push explicit lspace if it won't be added to the attributes of an `mo`.
    if !embellished
        && let Some(lspace) = props.lspace
        && !props.align_form_infix
        && !lspace.is_zero()
    {
        ctx.push(
            HtmlElement::new(tag::mspace)
                .with_attr(attr::width, eco_format!("{}em", lspace.get())),
        );
    }

    // For embellished operators which aren't an `mo` element, pass the outer
    // spacing and position to the core operator.
    if embellished
        && !matches!(comp.kind, MathKind::Glyph(_) | MathKind::Text(_))
        && ctx.embellishment.is_none()
        && !props.align_form_infix
    {
        ctx.embellishment = Some(EmbellishmentContext {
            lspace: props.lspace,
            rspace: props.rspace,
            position,
        });
    }

    match &comp.kind {
        MathKind::Glyph(item) => handle_glyph(item, ctx, props, position)?,
        MathKind::Radical(item) => handle_radical(item, ctx, props)?,
        MathKind::Accent(item) => handle_accent(item, ctx, props)?,
        MathKind::Scripts(item) => handle_scripts(item, ctx, props)?,
        MathKind::Primes(_item) => {}
        MathKind::Table(item) => handle_table(item, ctx, props)?,
        MathKind::Fraction(item) => handle_fraction(item, ctx, props)?,
        MathKind::Text(item) => handle_text(item, ctx, props, position)?,
        MathKind::Number(item) => handle_number(item, ctx, props)?,
        MathKind::Fenced(item) => handle_fenced(item, ctx, props)?,
        MathKind::Group(_) => {
            let node = ctx.handle_into_node(item)?;
            ctx.push(node);
        }
        MathKind::Multiline(item) => handle_multiline(item, ctx, props)?,

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

    // Push explicit rspace if it won't be added to the attributes of an `mo`.
    if !embellished
        && let Some(rspace) = props.rspace
        && !rspace.is_zero()
    {
        ctx.push(
            HtmlElement::new(tag::mspace)
                .with_attr(attr::width, eco_format!("{}em", rspace.get())),
        );
    }

    Ok(())
}

fn handle_multiline(
    item: &MultilineItem,
    ctx: &mut MathContext,
    _props: &MathProperties,
) -> SourceResult<()> {
    let aligned = item.rows.first().is_some_and(|row| row.len() > 1);
    let cells = item
        .rows
        .iter()
        .map(|row| {
            let cell_nodes = row
                .iter()
                .map(|cell| {
                    Ok(HtmlElement::new(tag::mtd)
                        .with_children(ctx.handle_into_nodes(cell)?)
                        .into())
                })
                .collect::<SourceResult<EcoVec<HtmlNode>>>();

            cell_nodes.map(|nodes| HtmlElement::new(tag::mtr).with_children(nodes).into())
        })
        .collect::<SourceResult<EcoVec<HtmlNode>>>()?;
    ctx.push(
        HtmlElement::new(tag::mtable)
            .with_attr(
                crate::attr::class,
                if aligned { ALIGNED_EQUATION_CLASS } else { MULTILINE_EQUATION_CLASS },
            )
            .with_children(cells),
    );
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
    position: NodePosition,
) -> SourceResult<()> {
    let text = &item.text;

    let mut form = None;
    let mut fence = false;
    let mut separator = false;
    let mut largeop = false;
    match props.class {
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
        MathClass::Relation => {}
        MathClass::Diacritic => form = Some(Form::Postfix),
        MathClass::Binary => form = Some(Form::Infix),
        MathClass::Vary | MathClass::Unary => form = Some(Form::Prefix),
        MathClass::Punctuation => separator = true,
        MathClass::Fence => fence = true,
        MathClass::Large => {
            largeop = true;
            form = Some(Form::Prefix);
        }
        MathClass::Opening => {
            fence = true;
            form = Some(Form::Prefix);
        }
        MathClass::Closing => {
            fence = true;
            form = Some(Form::Postfix);
        }
    }

    push_mo(
        text,
        props.span,
        ctx,
        props,
        position,
        form,
        fence,
        separator,
        largeop,
        Some(item.stretch.get()),
    );
    Ok(())
}

fn handle_accent(
    item: &AccentItem,
    ctx: &mut MathContext,
    _props: &MathProperties,
) -> SourceResult<()> {
    let (tag, attr) = if item.position == Position::Below {
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
    let accent = ctx.handle_into_node_with_only_form(&item.accent, Form::Postfix)?;

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
            item.$content
                .as_ref()
                .map(|x| ctx.handle_into_node_with_only_form(x, Form::Postfix))
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
    _props: &MathProperties,
) -> SourceResult<()> {
    let cells = item
        .cells
        .iter()
        .map(|row| {
            let cell_nodes = row
                .iter()
                .flat_map(|cell| {
                    // let props =
                    //     css::Properties::new().with("padding", "0em 0.2em");
                    let nodes: EcoVec<SourceResult<HtmlNode>> = cell
                        .iter()
                        .map(|sub_col| {
                            Ok(HtmlElement::new(tag::mtd)
                                .with_children(ctx.handle_into_nodes(sub_col)?)
                                // .with_styles(props)
                                .into())
                        })
                        .collect();

                    nodes.into_iter()
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
    position: NodePosition,
) -> SourceResult<()> {
    if props.class == MathClass::Large {
        push_mo(
            &item.text,
            props.span,
            ctx,
            props,
            position,
            Some(Form::Prefix),
            false,
            false,
            false,
            None,
        );
    } else {
        ctx.push(
            HtmlElement::new(tag::mtext)
                .with_children(eco_vec![HtmlNode::Text(item.text.clone(), props.span)]),
        );
    }
    Ok(())
}

fn handle_number(
    item: &NumberItem,
    ctx: &mut MathContext,
    props: &MathProperties,
) -> SourceResult<()> {
    ctx.push(
        HtmlElement::new(tag::mn)
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
    if let Some(open) = item
        .open
        .as_ref()
        .map(|x| ctx.handle_into_node_with_only_form(x, Form::Prefix))
        .transpose()?
    {
        children.push(open);
    }
    let body = ctx.handle_into_node(&item.body)?;
    children.push(body);
    if let Some(close) = item
        .close
        .as_ref()
        .map(|x| ctx.handle_into_node_with_only_form(x, Form::Postfix))
        .transpose()?
    {
        children.push(close);
    }

    ctx.push(HtmlElement::new(tag::mrow).with_children(children));
    Ok(())
}

/// Creates and adds an `mo` node.
#[allow(clippy::too_many_arguments)]
fn push_mo(
    text: &str,
    span: Span,
    ctx: &mut MathContext,
    props: &MathProperties,
    position: NodePosition,
    mut form: Option<Form>,
    fence: bool,
    separator: bool,
    largeop: bool,
    stretch: Option<Stretch>,
) {
    // If this is the core operator of an embellished operator, use the spacing
    // and position stored in the context.
    let (mut lspace, rspace, position) = ctx
        .embellishment
        .take()
        .map(|e| (e.lspace, e.rspace, e.position))
        .unwrap_or((props.lspace, props.rspace, position));

    if props.align_form_infix {
        form = Some(Form::Infix);
        // The lspace at this stage is gone, but since it is infix the spacing
        // is symmetric, so we can just use the rspace.
        lspace = rspace;
    }

    let initial_form = position.get_form();
    let info = OperatorInfo::of(
        text,
        form.unwrap_or(initial_form),
        form.filter(|f| *f != initial_form).is_some(),
    );
    let form = form.filter(|f| *f != initial_form).map(|f| match f {
        Form::Prefix => "prefix",
        Form::Infix => "infix",
        Form::Postfix => "postfix",
    });

    let lspace = lspace
        .filter(|l| l.get() != info.lspace)
        .filter(|_| !matches!(position, NodePosition::Only(_)))
        .map(|l| eco_format!("{}em", l.get()));
    let rspace = rspace
        .filter(|r| r.get() != info.rspace)
        .filter(|_| !matches!(position, NodePosition::Only(_)))
        .map(|r| eco_format!("{}em", r.get()));

    let fence = (fence != is_fence(text)).then(|| eco_format!("{}", fence));
    let separator =
        (separator != is_separator(text)).then(|| eco_format!("{}", separator));

    let largeop = (largeop != info.properties.contains(Properties::LARGEOP))
        .then(|| eco_format!("{}", largeop));
    // We don't use movablelimits as we handle the positioning ourselves.
    let movablelimits = (info.properties.contains(Properties::MOVABLELIMITS))
        .then(|| eco_format!("false"));

    // TODO: symmetric, maxsize
    let (stretchy, minsize) = if let Some(stretch) = stretch {
        let mut chars = text.chars();
        let stretch_axis = if let Some(c) = chars.next()
            && chars.next().is_none()
            && is_stretch_axis_inline(c)
        {
            Axis::X
        } else {
            Axis::Y
        };
        let semantic = stretch.is_semantic(stretch_axis);
        let stretchy = (semantic ^ info.properties.contains(Properties::STRETCHY))
            .then(|| eco_format!("{}", semantic));
        let minsize = stretch
            .resolve_requested(stretch_axis)
            .map(|target| eco_format!("{}", css::rel(target)));
        (stretchy, minsize)
    } else {
        (None, None)
    };

    ctx.push(
        HtmlElement::new(tag::mo)
            .with_children(eco_vec![HtmlNode::text(text, span)])
            .with_optional_attr(attr::form, form)
            .with_optional_attr(attr::lspace, lspace)
            .with_optional_attr(attr::rspace, rspace)
            .with_optional_attr(attr::fence, fence)
            .with_optional_attr(attr::separator, separator)
            .with_optional_attr(attr::largeop, largeop)
            .with_optional_attr(attr::movablelimits, movablelimits)
            .with_optional_attr(attr::minsize, minsize)
            .with_optional_attr(attr::stretchy, stretchy),
    );
}

/// Whether this item is considered an embellished operator in MathML Core.
fn is_embellished_operator(item: &MathItem) -> bool {
    let MathItem::Component(comp) = item else { return false };
    match &comp.kind {
        MathKind::Glyph(_) => !matches!(
            comp.props.class,
            MathClass::Normal
                | MathClass::Alphabetic
                | MathClass::Special
                | MathClass::GlyphPart
                | MathClass::Space
        ),
        MathKind::Text(_) => comp.props.class == MathClass::Large,
        MathKind::Scripts(scripts) => is_embellished_operator(&scripts.base),
        MathKind::Accent(accent) => is_embellished_operator(&accent.base),
        MathKind::Fraction(fraction) => is_embellished_operator(&fraction.numerator),
        MathKind::Group(group) => {
            group
                .items
                .iter()
                .filter(|item| !item.is_ignorant())
                .filter(|item| !is_space_like(item))
                .count()
                == 1
                && group
                    .items
                    .iter()
                    .filter(|item| !item.is_ignorant())
                    .any(|item| is_embellished_operator(item))
        }
        _ => false,
    }
}

/// Whether this item is considered a space-like element in MathML Core.
fn is_space_like(item: &MathItem) -> bool {
    match item {
        MathItem::Spacing(..) | MathItem::Space => true,
        MathItem::Component(comp) => match &comp.kind {
            MathKind::Text(_) => comp.props.class != MathClass::Large,
            MathKind::Group(group) => group
                .items
                .iter()
                .filter(|item| !item.is_ignorant())
                .all(is_space_like),
            _ => false,
        },
        _ => false,
    }
}
