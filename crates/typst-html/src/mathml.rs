use ecow::{EcoVec, eco_format, eco_vec};
use typst_assets::mathml::*;
use typst_library::diag::{SourceResult, warning};
use typst_library::engine::Engine;
use typst_library::layout::{Axis, Em};
use typst_library::math::MathSize;
use typst_library::math::ir::{
    AccentItem, FencedItem, FractionItem, GlyphItem, MathItem, MathKind, MathProperties,
    MultilineItem, NumberItem, Position, PrimesItem, RadicalItem, ScriptsItem, Stretch,
    TableItem, TextItem,
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
/* Tables */
mtable {
  math-style: inherit;
}
mtd {
  math-depth: auto-add;
  math-style: compact;
  math-shift: compact;
}

/* Equations */
mtable.multiline-equation mtd {
  math-depth: inherit;
  math-style: inherit;
  math-shift: inherit;
  padding: 0;
}
mtable.multiline-equation.aligned-equation mtd:nth-child(odd) {
  text-align: -webkit-right;
}
mtable.multiline-equation.aligned-equation mtd:nth-child(even) {
  text-align: -webkit-left;
}

/* Fractions */
mfrac {
  padding-inline: 0;
  margin-inline: 0.1em;
}

/* Other rules for scriptlevel, displaystyle and math-shift */
mroot {
  math-shift: inherit;
}
mroot > :first-child,
munder > :nth-child(2),
munderover > :nth-child(2),
munder[accentunder=\"true\" i] > :first-child {
  math-shift: compact
}
munder[accentunder=\"true\" i] > :nth-child(2),
mover[accent=\"true\" i] > :nth-child(2) {
  math-depth: inherit;
  math-style: inherit;
}";

pub(crate) fn convert_math_to_nodes(
    item: MathItem,
    engine: &mut Engine,
    block: bool,
) -> SourceResult<EcoVec<HtmlNode>> {
    let mut ctx = MathContext::new(engine, block);
    ctx.handle_into_nodes(&item)
}

trait HtmlNodesExt {
    fn into_node(self) -> HtmlNode;
}

impl HtmlNodesExt for EcoVec<HtmlNode> {
    fn into_node(self) -> HtmlNode {
        if self.len() == 1 {
            self.into_iter().next().unwrap()
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
    limits: bool,
}

#[derive(Copy, Clone, PartialEq)]
struct CssContext {
    /// True if math-style: normal, false if math-style: compact
    math_style_normal: bool,
    /// True if math-shift: normal, false if math-shift: compact
    math_shift_normal: bool,
    math_depth: u32,
}

impl CssContext {
    fn new(block: bool) -> Self {
        Self {
            math_style_normal: block,
            math_shift_normal: true,
            math_depth: 0,
        }
    }

    fn depth_auto_add(self) -> Self {
        Self {
            math_depth: if !self.math_style_normal {
                self.math_depth + 1
            } else {
                self.math_depth
            },
            ..self
        }
    }

    fn depth_add(self, n: u32) -> Self {
        Self { math_depth: self.math_depth + n, ..self }
    }

    fn style_compact(self) -> Self {
        Self { math_style_normal: false, ..self }
    }

    fn shift_compact(self) -> Self {
        Self { math_shift_normal: false, ..self }
    }

    fn from_math_size(size: MathSize) -> Self {
        match size {
            MathSize::Display => Self {
                math_style_normal: true,
                math_shift_normal: true,
                math_depth: 0,
            },
            MathSize::Text => Self {
                math_style_normal: false,
                math_shift_normal: true,
                math_depth: 0,
            },
            MathSize::Script => Self {
                math_style_normal: false,
                math_shift_normal: true,
                math_depth: 1,
            },
            MathSize::ScriptScript => Self {
                math_style_normal: false,
                math_shift_normal: true,
                math_depth: 2,
            },
        }
    }
}

/// The context for math handling.
struct MathContext<'v, 'e> {
    engine: &'v mut Engine<'e>,
    nodes: EcoVec<HtmlNode>,
    embellishment: Option<EmbellishmentContext>,
    css: CssContext,
}

impl<'v, 'e> MathContext<'v, 'e> {
    /// Create a new math context.
    fn new(engine: &'v mut Engine<'e>, block: bool) -> Self {
        Self {
            engine,
            nodes: EcoVec::new(),
            embellishment: None,
            css: CssContext::new(block),
        }
    }

    /// Push a node.
    fn push(&mut self, node: impl Into<HtmlNode>) {
        self.nodes.push(node.into());
    }

    fn with_css<T>(&mut self, css: CssContext, f: impl FnOnce(&mut Self) -> T) -> T {
        let prev = std::mem::replace(&mut self.css, css);
        let result = f(self);
        self.css = prev;
        result
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
            // We hard-code the space width as `(4/18)em`.
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

    let target = CssContext::from_math_size(props.size);
    // We clamp the tracked depth at 2 so we don't emit unnecessary
    // `scriptlevel="2"` everywhere.
    let scriptlevel = (target.math_depth != ctx.css.math_depth.min(2))
        .then(|| eco_format!("{}", target.math_depth));
    let displaystyle = (target.math_style_normal != ctx.css.math_style_normal)
        .then(|| eco_format!("{}", target.math_style_normal));

    // Push explicit lspace if it won't be added to the attributes of an `mo`.
    if !embellished
        && let Some(lspace) = props.lspace
        && !props.align_form_infix
        && !lspace.is_zero()
    {
        ctx.push(
            HtmlElement::new(tag::mspace)
                .with_attr(attr::width, eco_format!("{}em", lspace.get()))
                .with_optional_attr(attr::scriptlevel, scriptlevel.clone()),
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
            limits: has_limits(item),
        });
    }

    let css = if target != ctx.css { target } else { ctx.css };
    if let Some(node) = ctx.with_css(css, |ctx| -> SourceResult<Option<HtmlNode>> {
        Ok(match &comp.kind {
            MathKind::Glyph(item) => {
                Some(handle_glyph(item, ctx, props, position)?.into())
            }
            MathKind::Radical(item) => Some(handle_radical(item, ctx, props)?.into()),
            MathKind::Accent(item) => Some(handle_accent(item, ctx, props)?.into()),
            MathKind::Scripts(item) => Some(handle_scripts(item, ctx, props)?),
            MathKind::Primes(item) => Some(handle_primes(item, ctx, props)?.into()),
            MathKind::Table(item) => Some(handle_table(item, ctx, props)?.into()),
            MathKind::Fraction(item) => Some(handle_fraction(item, ctx, props)?.into()),
            MathKind::Text(item) => Some(handle_text(item, ctx, props, position)?.into()),
            MathKind::Number(item) => Some(handle_number(item, ctx, props)?.into()),
            MathKind::Fenced(item) => Some(handle_fenced(item, ctx, props)?.into()),
            MathKind::Group(_) => Some(ctx.handle_into_node(item)?),
            MathKind::Multiline(item) => Some(handle_multiline(item, ctx, props)?.into()),

            // Polyfill required for MathML Core.
            MathKind::SkewedFraction(_) | MathKind::Cancel(_) | MathKind::Line(_) => None,

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
            MathKind::Box(_) => None,
            MathKind::External(item) => {
                ctx.engine.sink.warn(warning!(
                    props.span,
                    "{} was ignored during MathML export",
                    item.content.elem().name()
                ));
                None
            }
        })
    })? {
        ctx.push(match node {
            // TODO: add math-shift: compact
            HtmlNode::Element(elem) => elem
                .with_optional_attr(attr::scriptlevel, scriptlevel.clone())
                .with_optional_attr(attr::displaystyle, displaystyle)
                .into(),
            other => other,
        })
    }

    // Push explicit rspace if it won't be added to the attributes of an `mo`.
    if !embellished
        && let Some(rspace) = props.rspace
        && !rspace.is_zero()
    {
        ctx.push(
            HtmlElement::new(tag::mspace)
                .with_attr(attr::width, eco_format!("{}em", rspace.get()))
                .with_optional_attr(attr::scriptlevel, scriptlevel),
        );
    }

    Ok(())
}

fn handle_multiline(
    item: &MultilineItem,
    ctx: &mut MathContext,
    _props: &MathProperties,
) -> SourceResult<HtmlElement> {
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
    Ok(HtmlElement::new(tag::mtable)
        .with_attr(
            crate::attr::class,
            if aligned { ALIGNED_EQUATION_CLASS } else { MULTILINE_EQUATION_CLASS },
        )
        .with_children(cells))
}

fn handle_fraction(
    item: &FractionItem,
    ctx: &mut MathContext,
    _props: &MathProperties,
) -> SourceResult<HtmlElement> {
    let num = ctx.with_css(ctx.css.depth_auto_add().style_compact(), |ctx| {
        ctx.handle_into_node(&item.numerator)
    })?;
    let denom = ctx
        .with_css(ctx.css.depth_auto_add().style_compact().shift_compact(), |ctx| {
            ctx.handle_into_node(&item.denominator)
        })?;
    let line = (!item.line).then_some("0");
    Ok(HtmlElement::new(tag::mfrac)
        .with_children(eco_vec![num, denom])
        .with_optional_attr(attr::linethickness, line))
}

fn handle_radical(
    item: &RadicalItem,
    ctx: &mut MathContext,
    _props: &MathProperties,
) -> SourceResult<HtmlElement> {
    let radicand = ctx
        .with_css(ctx.css.shift_compact(), |ctx| ctx.handle_into_nodes(&item.radicand))?;
    let index = item
        .index
        .as_ref()
        .map(|index| {
            ctx.with_css(ctx.css.depth_add(2).style_compact(), |ctx| {
                ctx.handle_into_node(index)
            })
        })
        .transpose()?;
    let (tag, children) = if let Some(index) = index {
        (tag::mroot, eco_vec![radicand.into_node(), index])
    } else {
        (tag::msqrt, radicand)
    };
    Ok(HtmlElement::new(tag).with_children(children))
}

fn handle_glyph(
    item: &GlyphItem,
    ctx: &mut MathContext,
    props: &MathProperties,
    position: NodePosition,
) -> SourceResult<HtmlElement> {
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
            return Ok(HtmlElement::new(tag::mi)
                .with_children(eco_vec![HtmlNode::text(text, props.span)])
                .with_optional_attr(attr::mathvariant, mathvariant));
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

    Ok(make_mo(
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
    ))
}

fn handle_accent(
    item: &AccentItem,
    ctx: &mut MathContext,
    _props: &MathProperties,
) -> SourceResult<HtmlElement> {
    let (tag, attr) = if item.position == Position::Below {
        (tag::munder, attr::accentunder)
    } else {
        (tag::mover, attr::accent)
    };

    let base =
        ctx.with_css(ctx.css.shift_compact(), |ctx| ctx.handle_into_node(&item.base))?;

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

    Ok(HtmlElement::new(tag)
        .with_children(eco_vec![base, accent])
        .with_attr(attr, "true"))
}

fn handle_scripts(
    item: &ScriptsItem,
    ctx: &mut MathContext,
    _props: &MathProperties,
) -> SourceResult<HtmlNode> {
    let mut base = ctx.handle_into_node(&item.base)?;

    let sup_css = ctx.css.depth_add(1).style_compact();
    let sub_css = sup_css.shift_compact();

    macro_rules! handle {
        ($content:ident, $css:ident) => {
            item.$content
                .as_ref()
                .map(|x| {
                    ctx.with_css($css, |ctx| {
                        ctx.handle_into_node_with_only_form(x, Form::Postfix)
                    })
                })
                .transpose()?
        };
    }

    let t = handle!(top, sup_css);
    let tr = handle!(top_right, sup_css);
    let tl = handle!(top_left, sup_css);
    let b = handle!(bottom, sub_css);
    let br = handle!(bottom_right, sub_css);
    let bl = handle!(bottom_left, sub_css);

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

    Ok(base)
}

fn handle_table(
    item: &TableItem,
    ctx: &mut MathContext,
    _props: &MathProperties,
) -> SourceResult<HtmlElement> {
    let css = ctx.css.depth_auto_add().style_compact().shift_compact();
    let cells = item
        .cells
        .iter()
        .map(|row| {
            let cell_nodes = row
                .iter()
                .flat_map(|cell| {
                    let nodes: EcoVec<SourceResult<HtmlNode>> = cell
                        .iter()
                        .map(|sub_col| {
                            Ok(HtmlElement::new(tag::mtd)
                                .with_children(ctx.with_css(css, |ctx| {
                                    ctx.handle_into_nodes(sub_col)
                                })?)
                                .into())
                        })
                        .collect();

                    nodes.into_iter()
                })
                .collect::<SourceResult<EcoVec<HtmlNode>>>();

            cell_nodes.map(|nodes| HtmlElement::new(tag::mtr).with_children(nodes).into())
        })
        .collect::<SourceResult<EcoVec<HtmlNode>>>()?;
    Ok(HtmlElement::new(tag::mtable).with_children(cells))
}

fn handle_text(
    item: &TextItem,
    ctx: &mut MathContext,
    props: &MathProperties,
    position: NodePosition,
) -> SourceResult<HtmlElement> {
    Ok(if props.class == MathClass::Large {
        make_mo(
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
        )
    } else {
        HtmlElement::new(tag::mtext)
            .with_children(eco_vec![HtmlNode::Text(item.text.clone(), props.span)])
    })
}

fn handle_number(
    item: &NumberItem,
    _ctx: &mut MathContext,
    props: &MathProperties,
) -> SourceResult<HtmlElement> {
    Ok(HtmlElement::new(tag::mn)
        .with_children(eco_vec![HtmlNode::Text(item.text.clone(), props.span)]))
}

fn handle_primes(
    item: &PrimesItem,
    ctx: &mut MathContext,
    _props: &MathProperties,
) -> SourceResult<HtmlElement> {
    let prime = ctx.handle_into_node(&item.prime)?;
    Ok(HtmlElement::new(tag::mrow).with_children(eco_vec![prime; item.count]))
}

fn handle_fenced(
    item: &FencedItem,
    ctx: &mut MathContext,
    _props: &MathProperties,
) -> SourceResult<HtmlElement> {
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

    // We shouldn't always need to wrap this in an `mrow`, but browsers don't
    // follow the spec and insert an inferred `mrow` in situations like
    // `<msqrt>...</msqrt>`, `<mtd>...</mtd>`, and the top-level
    // `<math>...</math>`. For example:
    // https://bugzilla.mozilla.org/show_bug.cgi?id=236963
    Ok(HtmlElement::new(tag::mrow).with_children(children))
}

/// Creates an `mo` element.
#[allow(clippy::too_many_arguments)]
fn make_mo(
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
) -> HtmlElement {
    // If this is the core operator of an embellished operator, use the spacing
    // and position stored in the context.
    let (mut lspace, rspace, position, limits) = ctx
        .embellishment
        .take()
        .map(|e| (e.lspace, e.rspace, e.position, e.limits))
        .unwrap_or((props.lspace, props.rspace, position, false));

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
    let movablelimits = (limits
        && !ctx.css.math_style_normal
        && info.properties.contains(Properties::MOVABLELIMITS))
    .then(|| eco_format!("false"));

    let mut chars = text.chars();
    let stretch_axis = if let Some(c) = chars.next()
        && chars.next().is_none()
        && is_stretch_axis_inline(c)
    {
        Axis::X
    } else {
        Axis::Y
    };
    let semantic = stretch.is_some_and(|stretch| stretch.is_semantic(stretch_axis));
    let stretchy = (semantic ^ info.properties.contains(Properties::STRETCHY))
        .then(|| eco_format!("{}", semantic));

    // We don't need to set `maxsize` as it is infinity by default.
    let (symmetric, minsize) = if semantic {
        let vertical = stretch_axis == Axis::Y;
        let symmetric = (vertical ^ info.properties.contains(Properties::SYMMETRIC))
            .then(|| eco_format!("{}", vertical));

        let minsize = stretch
            .unwrap()
            .resolve_requested(stretch_axis)
            .map(|target| eco_format!("{}", css::rel(target)));

        (symmetric, minsize)
    } else {
        (None, None)
    };

    HtmlElement::new(tag::mo)
        .with_children(eco_vec![HtmlNode::text(text, span)])
        .with_optional_attr(attr::form, form)
        .with_optional_attr(attr::fence, fence)
        .with_optional_attr(attr::separator, separator)
        .with_optional_attr(attr::lspace, lspace)
        .with_optional_attr(attr::rspace, rspace)
        .with_optional_attr(attr::stretchy, stretchy)
        .with_optional_attr(attr::symmetric, symmetric)
        .with_optional_attr(attr::minsize, minsize)
        .with_optional_attr(attr::largeop, largeop)
        .with_optional_attr(attr::movablelimits, movablelimits)
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
            let mut items = group
                .items
                .iter()
                .filter(|item| !item.is_ignorant() && !is_space_like(item));
            items.next().is_some_and(|item| is_embellished_operator(item))
                && items.next().is_none()
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

/// Whether this item, which is considered an embellished operator in MathML
/// Core, has top or bottom attachements somewhere.
fn has_limits(item: &MathItem) -> bool {
    let MathItem::Component(comp) = item else { return false };
    match &comp.kind {
        MathKind::Glyph(_) => false,
        MathKind::Text(_) => false,
        MathKind::Scripts(scripts) => {
            scripts.top.is_some() || scripts.bottom.is_some() || has_limits(&scripts.base)
        }
        MathKind::Accent(accent) => has_limits(&accent.base),
        MathKind::Fraction(fraction) => has_limits(&fraction.numerator),
        MathKind::Group(group) => group.items.iter().any(|item| has_limits(item)),
        _ => false,
    }
}
