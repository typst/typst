use std::fmt;
use std::sync::LazyLock;

use ecow::{EcoString, eco_format, eco_vec};
use typst_assets::mathml::*;
use typst_library::diag::{SourceResult, warning};
use typst_library::engine::Engine;
use typst_library::foundations::{Content, NativeElement, StyleChain, StyledElem};
use typst_library::introspection::TagElem;
use typst_library::layout::{Axis, Em, FixedAlignment};
use typst_library::math::ir::{
    AccentItem, FencedItem, FractionItem, GlyphItem, MathItem, MathKind, MathProperties,
    MathmlItem, MultilineItem, NumberItem, PRIME_CHAR, Position, PrimesItem, RadicalItem,
    ScriptsItem, Stretch, TableItem, TextItem,
};
use typst_library::math::{FRAC_PADDING, LeftRightAlternator, MathSize};
use typst_library::text::TextElem;
use typst_syntax::Span;
use typst_utils::Numeric;
use unicode_math_class::MathClass;

use crate::HtmlElem;
use crate::css::ToCss;
use crate::tag::mathml as tag;
use crate::{attr::mathml as attr, css};

/// How Typst overrides the [MathML Core User Agent Stylesheet][UA].
///
/// What follows are explanations justifying the inclusion of each rule. The
/// main purpose is to ensure the MathML produced is rendered by browsers as
/// closely as possible to Typst's paged export. Things not included here mean
/// that the UA stylesheet already matches!
///
/// The oracle of truth for Typst's paged export are the styles applied in the
/// [`resolve_*` functions in `typst_library::math::ir::resolve`][resolve].
/// These styles are the [`style_*` functions in `typst_library::math`][style].
/// Their documentation includes how they map to CSS properties in MathML Core.
///
/// # Alignment
///
/// To get the alternating alignment working, we use the `text-align` property
/// on `mtd`. The vendox prefix value is used because it is the only thing that
/// actually works across browsers. In the future however, we should use
/// [`justify-items`][alignment] instead.
///
/// Inline multiline equations and the `cases` class on an `mtable` are always
/// left aligned. The `aligned` class on an `mtable` alternates right-left
/// alignment. Finally the `right-align` and `left-align` classes can be used
/// on either an `mtable` or an `mtd` to provide right and left alignment,
/// respectively.
///
/// The `cases` and `aligned` classes on `mtable` all have the horizontal
/// spacing (padding) set to zero. We also have the `flushed`, `left-flush` and
/// `right-flush` classes for `mtd` that set the corresponding horizontal
/// spacing to zero.
///
/// # Tables
///
/// The UA stylesheet sets `math-style: compact` at the top-level for every
/// `mtable`. We undo this, and instead set on `mtd` the properties matching
/// [`style_for_denominator`][denom].
///
/// # Equations
///
/// For equations we are using `mtable` to do multiline layout and alignment.
/// So we undo all changes to the CSS math properties, and set the padding to
/// zero so that there is no space between columns.
///
/// To ensure that there is some line spacing between each line in a multiline
/// equation, we set bottom padding only for all but the last row of `mtd`.
///
/// # Fractions
///
/// The UA stylesheet adds a fixed inline padding of `1px`. This, however, is
/// added within the denominator and numerator, making the fraction rule longer
/// instead of being added around the fraction. We use `margin-inline` to fix
/// this and use the same value of `0.1em` as in paged export.
///
/// # Accents
///
/// We enable by default the `dtls` OpenType feature for all top accents, and
/// then add a class to explicitly disable this.
///
/// # Other rules for scriptlevel, displaystyle and math-shift
///
/// The first set of rules ensure bottom attachements are [cramped], matching
/// bottom-left and bottom-right ones.
///
/// The second set of rules undo the UA stylesheet's and our immediately prior
/// rules on accents themselves.
///
/// [UA]: https://www.w3.org/TR/mathml-core/#user-agent-stylesheet
/// [resolve]: typst_library::math::ir#functions
/// [style]: typst_library::math#functions
/// [denom]: typst_library::math::style_for_denominator
/// [alignment]: https://github.com/w3c/mathml-core/issues/156
/// [cramped]: typst_library::math::style_cramped
pub(crate) static EQUATION_CSS_STYLES: LazyLock<EcoString> = LazyLock::new(|| {
    eco_format!(
        "\
/* Alignment */
mtable.{RIGHT_ALIGN_CLASS} mtd,
mtable mtd.{RIGHT_ALIGN_CLASS},
mtable.{LEFT_ALIGN_CLASS} mtd.{RIGHT_ALIGN_CLASS},
mtable.{ALIGNED_CLASS} mtd:nth-child(odd) {{
  text-align: {TEXT_ALIGN_RIGHT};
}}
mtable.{CASES_CLASS} mtd,
mtable.{LEFT_ALIGN_CLASS} mtd,
mtable mtd.{LEFT_ALIGN_CLASS},
mtable.{ALIGNED_CLASS} mtd:nth-child(even),
math:is(:not([display])) > mtable.{MULTILINE_EQUATION_CLASS} mtd {{
  text-align: {TEXT_ALIGN_LEFT};
}}
mtable.{CASES_CLASS} mtd,
mtable.{ALIGNED_CLASS} mtd,
mtable mtd.{FLUSHED_CLASS},
mtable mtd.{LEFT_FLUSH_CLASS} {{
  padding-left: 0;
}}
mtable.{CASES_CLASS} mtd,
mtable.{ALIGNED_CLASS} mtd,
mtable mtd.{FLUSHED_CLASS},
mtable mtd.{RIGHT_FLUSH_CLASS} {{
  padding-right: 0;
}}

/* Tables */
mtable {{
  math-style: inherit;
}}
mtd {{
  math-depth: auto-add;
  math-style: compact;
  math-shift: compact;
}}

/* Equations */
mtable.{MULTILINE_EQUATION_CLASS} mtd {{
  math-depth: inherit;
  math-style: inherit;
  math-shift: inherit;
  padding: 0;
}}
math > mtable.{MULTILINE_EQUATION_CLASS} mtr:not(:last-child) mtd {{
  padding-bottom: {};
}}

/* Fractions */
mfrac {{
  padding-inline: 0;
  margin-inline: {};
}}

/* Accents */
mover[accent=\"true\" i] > :first-child {{
  font-feature-settings: \"dtls\";
}}
mover.dotted[accent=\"true\" i] > :first-child {{
  font-feature-settings: \"dtls\" 0;
}}

/* Other rules for scriptlevel, displaystyle and math-shift */
munder > :nth-child(2),
munderover > :nth-child(2) {{
  math-shift: compact
}}
munder[accentunder=\"true\" i] > :not(:first-child),
mover[accent=\"true\" i] > :not(:first-child) {{
  math-depth: inherit;
  math-style: inherit;
  math-shift: inherit;
}}",
        EQUATION_ROW_GAP.to_css(()),
        FRAC_PADDING.to_css(())
    )
});

// CSS classes.
const MULTILINE_EQUATION_CLASS: &str = "multiline-equation";
const ALIGNED_CLASS: &str = "aligned";
const LEFT_ALIGN_CLASS: &str = "left-align";
const RIGHT_ALIGN_CLASS: &str = "right-align";
const CASES_CLASS: &str = "cases";
const FLUSHED_CLASS: &str = "flushed";
const LEFT_FLUSH_CLASS: &str = "left-flush";
const RIGHT_FLUSH_CLASS: &str = "right-flush";

// CSS values.
const TEXT_ALIGN_RIGHT: &str = "-webkit-right";
const TEXT_ALIGN_LEFT: &str = "-webkit-left";
const EQUATION_ROW_GAP: Em = Em::new(0.5);

const SPACE_WIDTH: Em = Em::new(4.0 / 18.0);

pub(crate) fn convert_math_to_nodes(
    item: MathItem,
    engine: &mut Engine,
    styles: StyleChain,
    block: bool,
) -> SourceResult<Vec<Content>> {
    let mut ctx = MathContext::new(engine, styles, block);
    ctx.handle_into_nodes(&item)
}

/// Collapses a collection of nodes into a single node, wrapping multiple nodes
/// in an `mrow`.
trait ContentExt {
    fn into_content(self) -> Content;
}

impl ContentExt for Vec<Content> {
    fn into_content(self) -> Content {
        if self.len() == 1 {
            self.into_iter().next().unwrap()
        } else {
            HtmlElem::new(tag::mrow)
                .with_body(Some(Content::sequence(self)))
                .pack()
        }
    }
}

/// Where an item sits among its siblings. Used to derive the default operator
/// form for an `mo`.
#[derive(Copy, Clone)]
enum NodePosition {
    Start,
    Middle,
    End,
    /// A lone item, with an optional explicit form given.
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

/// Outer spacing and position for an embellished operator, passed down to the
/// core `mo` where they are emitted as attributes.
struct EmbellishmentContext {
    lspace: Option<Em>,
    rspace: Option<Em>,
    position: NodePosition,
    limits: bool,
}

/// The `math-shift` CSS property.
#[derive(Copy, Clone, PartialEq)]
enum MathShift {
    Normal,
    Compact,
}

impl fmt::Display for MathShift {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                MathShift::Normal => "normal",
                MathShift::Compact => "compact",
            }
        )
    }
}

/// The `math-style` CSS property.
#[derive(Copy, Clone, PartialEq)]
enum MathStyle {
    Normal,
    Compact,
}

/// The MathML Core CSS properties currently in effect.
///
/// Tracked so children only emit attributes that differ from what the browser
/// will apply via the UA styelsheet and our stylesheet.
#[derive(Copy, Clone, PartialEq)]
struct CssContext {
    math_style: MathStyle,
    math_shift: MathShift,
    math_depth: u32,
}

impl CssContext {
    fn new(block: bool) -> Self {
        Self {
            math_style: if block { MathStyle::Normal } else { MathStyle::Compact },
            math_shift: MathShift::Normal,
            math_depth: 0,
        }
    }

    fn depth_auto_add(self) -> Self {
        Self {
            math_depth: if self.math_style == MathStyle::Compact {
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
        Self { math_style: MathStyle::Compact, ..self }
    }

    fn shift_compact(self) -> Self {
        Self { math_shift: MathShift::Compact, ..self }
    }

    fn from(size: MathSize, cramped: bool) -> Self {
        let math_shift = if cramped { MathShift::Compact } else { MathShift::Normal };
        match size {
            MathSize::Display => Self {
                math_style: MathStyle::Normal,
                math_shift,
                math_depth: 0,
            },
            MathSize::Text => Self {
                math_style: MathStyle::Compact,
                math_shift,
                math_depth: 0,
            },
            MathSize::Script => Self {
                math_style: MathStyle::Compact,
                math_shift,
                math_depth: 1,
            },
            MathSize::ScriptScript => Self {
                math_style: MathStyle::Compact,
                math_shift,
                math_depth: 2,
            },
        }
    }
}

/// The context for math handling.
struct MathContext<'v, 'e> {
    engine: &'v mut Engine<'e>,
    content: Vec<Content>,
    embellishment: Option<EmbellishmentContext>,
    css: CssContext,
    trunk_depth: usize,
}

impl<'v, 'e> MathContext<'v, 'e> {
    /// Create a new math context.
    fn new(engine: &'v mut Engine<'e>, styles: StyleChain, block: bool) -> Self {
        Self {
            engine,
            content: Vec::new(),
            embellishment: None,
            css: CssContext::new(block),
            trunk_depth: styles.links().count(),
        }
    }

    /// Build a text element with the given diverging styles (relative to the
    /// equation's trunk). This is so that show rules aren't applied again to
    /// this text when the result html elements are realized again.
    fn body_text(&self, text: EcoString, span: Span, styles: StyleChain) -> Content {
        self.apply_diverging(TextElem::packed(text).spanned(span), styles)
    }

    /// Apply the diverging styles (relative to the equation's trunk) to the
    /// given content. This is so that show rules on `html.elem` see the styles
    /// that were active during math realization.
    fn apply_diverging(&self, content: Content, styles: StyleChain) -> Content {
        content.styled_with_map(styles.suffix(self.trunk_depth))
    }

    /// Push a node.
    fn push(&mut self, content: Content) {
        self.content.push(content);
    }

    /// Run `f` with the CSS context temporarily replaced.
    fn with_css<T>(&mut self, css: CssContext, f: impl FnOnce(&mut Self) -> T) -> T {
        let prev = std::mem::replace(&mut self.css, css);
        let result = f(self);
        self.css = prev;
        result
    }

    /// Run `f` with the trunk depth temporarily replaced.
    fn with_trunk_depth<T>(
        &mut self,
        trunk_depth: usize,
        f: impl FnOnce(&mut Self) -> T,
    ) -> T {
        let prev = std::mem::replace(&mut self.trunk_depth, trunk_depth);
        let result = f(self);
        self.trunk_depth = prev;
        result
    }

    /// Handle the given item and return the resulting nodes, using `only` as
    /// the operator form preference if the item turns out to be lone.
    fn handle_into_nodes_with_only_form(
        &mut self,
        item: &MathItem,
        only: Option<Form>,
    ) -> SourceResult<Vec<Content>> {
        let prev = std::mem::take(&mut self.content);
        self.handle_into_self(item, only)?;
        Ok(std::mem::replace(&mut self.content, prev))
    }

    /// Handle the given item and return the resulting nodes.
    fn handle_into_nodes(&mut self, item: &MathItem) -> SourceResult<Vec<Content>> {
        self.handle_into_nodes_with_only_form(item, None)
    }

    /// Handle the given item and collapse the resulting nodes into a single
    /// node.
    fn handle_into_node(&mut self, item: &MathItem) -> SourceResult<Content> {
        Ok(self.handle_into_nodes(item)?.into_content())
    }

    /// Handle the given item and collapse the resulting nodes into a single
    /// node, using `only` as the operator form preference if the item turns
    /// out to be lone.
    fn handle_into_node_with_only_form(
        &mut self,
        item: &MathItem,
        only: Form,
    ) -> SourceResult<Content> {
        Ok(self
            .handle_into_nodes_with_only_form(item, Some(only))?
            .into_content())
    }

    /// Handle the given item as a lone child and strip form/spacing attributes
    /// from a resulting bare `mo`.
    ///
    /// This is for attachments in MathML, where a lone `mo` would otherwise
    /// get form `postfix` and have its `lspace`/`rspace` ignored. See
    /// [§ 3.3.1.2 Layout of `<mrow>`][layout].
    ///
    /// [layout]: https://www.w3.org/TR/mathml-core/#layout-of-mrow
    fn handle_into_node_lone(&mut self, item: &MathItem) -> SourceResult<Content> {
        Ok(strip_inert_mo_attrs(
            self.handle_into_node_with_only_form(item, Form::Postfix)?,
        ))
    }

    /// Dispatch each child to the element handler, tagging it with its
    /// position among siblings. For a single-child item, `only` is the form
    /// preference.
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
                HtmlElem::new(tag::mspace)
                    .with_attr(attr::width, amount.to_css(()))
                    .pack(),
            );
            return Ok(());
        }
        MathItem::Space => {
            // We hard-code the space width as `(4/18)em`.
            ctx.push(
                HtmlElem::new(tag::mspace)
                    .with_attr(attr::width, SPACE_WIDTH.to_css(()))
                    .pack(),
            );
            return Ok(());
        }
        MathItem::Tag(tag) => {
            ctx.push(TagElem::new(tag.clone()).pack());
            return Ok(());
        }
    };

    let props = &comp.props;
    let styles = comp.styles;
    let embellished = is_embellished_operator(item);

    let target = CssContext::from(props.size, props.cramped);
    // We clamp the tracked depth at 2 so we don't emit unnecessary
    // `scriptlevel="2"` everywhere.
    let scriptlevel = (target.math_depth != ctx.css.math_depth.min(2))
        .then(|| eco_format!("{}", target.math_depth));
    let displaystyle = (target.math_style != ctx.css.math_style)
        .then(|| eco_format!("{}", target.math_style == MathStyle::Normal));
    let mut properties = css::Properties::new();
    if target.math_shift != ctx.css.math_shift {
        properties.push("math-shift", target.math_shift.to_string());
    }

    // Push explicit lspace if it won't be added to the attributes of an `mo`.
    if !embellished
        && let Some(lspace) = props.lspace
        && !props.align_form_infix
        && !lspace.is_zero()
    {
        ctx.push(
            HtmlElem::new(tag::mspace)
                .with_attr(attr::width, eco_format!("{}em", lspace.get()))
                .with_optional_attr(attr::scriptlevel, scriptlevel.clone())
                .pack(),
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
    let trunk_depth = styles.links().count();
    // Element's own context, established by the attributes emitted on it below.
    if let Some(mut node) = ctx.with_css(css, |ctx| -> SourceResult<Option<Content>> {
        ctx.with_trunk_depth(trunk_depth, |ctx| {
            Ok(match &comp.kind {
                MathKind::Glyph(item) => {
                    Some(handle_glyph(item, ctx, props, position, styles)?)
                }
                MathKind::Radical(item) => Some(handle_radical(item, ctx, props)?),
                MathKind::Accent(item) => Some(handle_accent(item, ctx, props)?),
                MathKind::Scripts(item) => Some(handle_scripts(item, ctx, props)?),
                MathKind::Primes(item) => {
                    Some(handle_primes(item, ctx, props, position, styles)?)
                }
                MathKind::Table(item) => Some(handle_table(item, ctx, props)?),
                MathKind::Fraction(item) => Some(handle_fraction(item, ctx, props)?),
                MathKind::Text(item) => {
                    Some(handle_text(item, ctx, props, position, styles)?)
                }
                MathKind::Number(item) => Some(handle_number(item, ctx, props, styles)?),
                MathKind::Fenced(item) => Some(handle_fenced(item, ctx, props)?),
                MathKind::Group(_) => Some(ctx.handle_into_node(item)?),
                MathKind::Multiline(item) => Some(handle_multiline(item, ctx, props)?),
                MathKind::Mathml(item) => Some(handle_mathml(item, ctx, props)?),

                // Polyfill required for MathML Core.
                MathKind::SkewedFraction(item) => Some(ignored_math_item(
                    ctx,
                    &item.numerator,
                    props.span,
                    "skewed fraction",
                )?),
                MathKind::Line(item) if matches!(item.position, Position::Above) => {
                    Some(ignored_math_item(ctx, &item.base, props.span, "overline")?)
                }
                MathKind::Line(item) => {
                    Some(ignored_math_item(ctx, &item.base, props.span, "underline")?)
                }
                MathKind::Cancel(item) => {
                    Some(ignored_math_item(ctx, &item.base, props.span, "cancel")?)
                }

                // TODO: enforce phrasing content.
                //
                // Arbitrary content is not allowed, as per the HTML spec. Only
                // MathML token elements (mi, mo, mn, ms, and mtext), when
                // descendants of HTML elements, may contain phrasing content
                // from the HTML namespace. See
                // https://html.spec.whatwg.org/#phrasing-content-2
                //
                // In MathML 3, nothing is allowed except MathML elements.
                // Further, nesting root-level math elements is disallowed.
                // Since the math element is considered phrasing content, it
                // can be nested in MathML Core (as long as it is itself within
                // a MathML token element).
                MathKind::Box(item) => Some(item.elem.clone().pack()),
                MathKind::External(item) => Some(item.content.clone()),
            })
        })
    })? {
        let modified = modify_inner_html_elem(&mut node, |elem| {
            elem.with_optional_attr(attr::scriptlevel, scriptlevel.clone())
                .with_optional_attr(attr::displaystyle, displaystyle)
                .with_css(properties)
        });
        ctx.push(if modified { ctx.apply_diverging(node, styles) } else { node })
    }

    // Push explicit rspace if it won't be added to the attributes of an `mo`.
    if !embellished
        && let Some(rspace) = props.rspace
        && !rspace.is_zero()
    {
        ctx.push(
            HtmlElem::new(tag::mspace)
                .with_attr(attr::width, eco_format!("{}em", rspace.get()))
                .with_optional_attr(attr::scriptlevel, scriptlevel)
                .pack(),
        );
    }

    Ok(())
}

fn handle_multiline(
    item: &MultilineItem,
    ctx: &mut MathContext,
    _props: &MathProperties,
) -> SourceResult<Content> {
    let cells = item
        .rows
        .iter()
        .map(|row| {
            let cell_nodes = row
                .iter()
                .map(|cell| {
                    Ok(HtmlElem::new(tag::mtd)
                        .with_body(Some(Content::sequence(ctx.handle_into_nodes(cell)?)))
                        .pack())
                })
                .collect::<SourceResult<Vec<Content>>>();

            cell_nodes.map(|nodes| {
                HtmlElem::new(tag::mtr)
                    .with_body(Some(Content::sequence(nodes)))
                    .pack()
            })
        })
        .collect::<SourceResult<Vec<Content>>>()?;

    let mut class = EcoString::from(MULTILINE_EQUATION_CLASS);
    if item.rows.first().is_some_and(|row| row.len() > 1) {
        class.push(' ');
        class.push_str(ALIGNED_CLASS);
    }

    Ok(HtmlElem::new(tag::mtable)
        .with_attr(crate::attr::class, class)
        .with_body(Some(Content::sequence(cells)))
        .pack())
}

fn handle_fraction(
    item: &FractionItem,
    ctx: &mut MathContext,
    _props: &MathProperties,
) -> SourceResult<Content> {
    // UA stylesheet.
    let num = ctx.with_css(ctx.css.depth_auto_add().style_compact(), |ctx| {
        ctx.handle_into_node(&item.numerator)
    })?;
    // UA stylesheet.
    let denom = ctx
        .with_css(ctx.css.depth_auto_add().style_compact().shift_compact(), |ctx| {
            ctx.handle_into_node(&item.denominator)
        })?;
    let line = (!item.line).then_some("0");
    Ok(HtmlElem::new(tag::mfrac)
        .with_body(Some(num + denom))
        .with_optional_attr(attr::linethickness, line)
        .pack())
}

fn handle_radical(
    item: &RadicalItem,
    ctx: &mut MathContext,
    _props: &MathProperties,
) -> SourceResult<Content> {
    // UA stylesheet.
    let radicand = ctx
        .with_css(ctx.css.shift_compact(), |ctx| ctx.handle_into_nodes(&item.radicand))?;
    let index = item
        .index
        .as_ref()
        .map(|index| {
            // UA stylesheet.
            ctx.with_css(ctx.css.depth_add(2).style_compact().shift_compact(), |ctx| {
                ctx.handle_into_node(index)
            })
        })
        .transpose()?;
    let (tag, body) = if let Some(index) = index {
        (tag::mroot, radicand.into_content() + index)
    } else {
        (tag::msqrt, Content::sequence(radicand))
    };
    Ok(HtmlElem::new(tag).with_body(Some(body)).pack())
}

fn handle_glyph(
    item: &GlyphItem,
    ctx: &mut MathContext,
    props: &MathProperties,
    position: NodePosition,
    styles: StyleChain,
) -> SourceResult<Content> {
    let text = &item.text;

    // Prime and factorial characters have math class Normal but are
    // semantically postfix operators, so emit `mo`. But only do this when the
    // math class hasn't been explicitly set by the user to something else.
    if props.class == MathClass::Normal && (is_prime(text) || text == "!") {
        return Ok(make_mo(
            text,
            props.span,
            ctx,
            props,
            position,
            Some(Form::Postfix),
            false,
            false,
            false,
            None,
            styles,
        ));
    }

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
            return Ok(HtmlElem::new(tag::mi)
                .with_body(Some(ctx.body_text(text.clone(), props.span, styles)))
                .with_optional_attr(attr::mathvariant, mathvariant)
                .pack());
        }
        MathClass::Diacritic => form = Some(Form::Postfix),
        MathClass::Binary | MathClass::Relation => form = Some(Form::Infix),
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
        styles,
    ))
}

fn handle_accent(
    item: &AccentItem,
    ctx: &mut MathContext,
    _props: &MathProperties,
) -> SourceResult<Content> {
    let (tag, attr, css) = if item.position == Position::Below {
        (tag::munder, attr::accentunder, ctx.css)
    } else {
        (tag::mover, attr::accent, ctx.css.shift_compact())
    };

    // UA stylesheet.
    let base = ctx.with_css(css, |ctx| ctx.handle_into_node(&item.base))?;

    // Firefox seems to be the only browser that applies the `dtls` OpenType
    // feature for the base of accents by default. We use our own CSS to force
    // enable/disable it on all browsers.
    let dotted =
        (!item.exact_frame_width && item.position == Position::Above && !item.dotless)
            .then_some("dotted");

    // Whilst the validator https://validator.w3.org/nu/#file warns "Text run
    // starts with a composing character", using non-combining characters isn't
    // possible at the moment as not all accents have a non-combining
    // equivalent and the display of them is bad in browsers.
    // See https://github.com/w3c/mathml-core/issues/311.
    let accent = ctx.handle_into_node_lone(&item.accent)?;

    Ok(HtmlElem::new(tag)
        .with_body(Some(base + accent))
        .with_attr(attr, "true")
        .with_optional_attr(crate::attr::class, dotted)
        .pack())
}

fn handle_scripts(
    item: &ScriptsItem,
    ctx: &mut MathContext,
    _props: &MathProperties,
) -> SourceResult<Content> {
    let mut base = ctx.handle_into_node(&item.base)?;

    // UA stylesheet + Typst stylesheet making bottom attachments cramped.
    let sup_css = ctx.css.depth_add(1).style_compact();
    let sub_css = sup_css.shift_compact();

    macro_rules! handle {
        ($content:ident, $css:ident) => {
            item.$content
                .as_ref()
                .map(|x| ctx.with_css($css, |ctx| ctx.handle_into_node_lone(x)))
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
        (None, None, None, Some(br)) => Some((tag::msub, vec![br])),
        (None, Some(tr), None, None) => Some((tag::msup, vec![tr])),
        (None, Some(tr), None, Some(br)) => Some((tag::msubsup, vec![br, tr])),
        (tl, tr, bl, br) => {
            let unwrap =
                |node: Option<Content>| node.unwrap_or(HtmlElem::new(tag::mrow).pack());
            Some((
                tag::mmultiscripts,
                vec![
                    unwrap(br),
                    unwrap(tr),
                    HtmlElem::new(tag::mprescripts).pack(),
                    unwrap(bl),
                    unwrap(tl),
                ],
            ))
        }
    } {
        let mut children = vec![base];
        children.extend(other_children);
        base = HtmlElem::new(tag).with_body(Some(Content::sequence(children))).pack();
    }

    if let Some((tag, other_children)) = match (t, b) {
        (None, None) => None,
        (Some(t), None) => Some((tag::mover, eco_vec![t])),
        (None, Some(b)) => Some((tag::munder, eco_vec![b])),
        (Some(t), Some(b)) => Some((tag::munderover, eco_vec![b, t])),
    } {
        let mut children = vec![base];
        children.extend(other_children);
        base = HtmlElem::new(tag).with_body(Some(Content::sequence(children))).pack();
    }

    Ok(base)
}

fn handle_table(
    item: &TableItem,
    ctx: &mut MathContext,
    _props: &MathProperties,
) -> SourceResult<Content> {
    let (ncols, has_sub_cols) = item
        .cells
        .first()
        .map_or((0, false), |row| (row.len(), row.iter().any(|cell| cell.len() > 1)));

    // Typst stylesheet.
    let css = ctx.css.depth_auto_add().style_compact().shift_compact();
    let cells = item
        .cells
        .iter()
        .map(|row| {
            let nodes = row
                .iter()
                .flat_map(|cell| {
                    cell.iter().enumerate().map(|(i, sub_col)| (i, cell.len(), sub_col))
                })
                .map(|(i, count, sub_col)| {
                    Ok(HtmlElem::new(tag::mtd)
                        .with_body(Some(Content::sequence(
                            ctx.with_css(css, |ctx| ctx.handle_into_nodes(sub_col))?,
                        )))
                        .with_optional_attr(
                            crate::attr::class,
                            table_mtd_class(i, count, ncols, item.alternator),
                        )
                        .pack())
                })
                .collect::<SourceResult<Vec<Content>>>()?;
            Ok(HtmlElem::new(tag::mtr)
                .with_body(Some(Content::sequence(nodes)))
                .pack())
        })
        .collect::<SourceResult<Vec<Content>>>()?;

    let class = match (item.alternator, item.align) {
        (LeftRightAlternator::None, _) if ncols == 1 => Some(CASES_CLASS),
        (LeftRightAlternator::Right, _) if ncols == 1 && has_sub_cols => {
            Some(ALIGNED_CLASS)
        }
        (_, FixedAlignment::Start) => Some(LEFT_ALIGN_CLASS),
        (_, FixedAlignment::End) => Some(RIGHT_ALIGN_CLASS),
        (_, FixedAlignment::Center) => None,
    };

    Ok(HtmlElem::new(tag::mtable)
        .with_body(Some(Content::sequence(cells)))
        .with_optional_attr(crate::attr::class, class)
        .pack())
}

fn handle_text(
    item: &TextItem,
    ctx: &mut MathContext,
    props: &MathProperties,
    position: NodePosition,
    styles: StyleChain,
) -> SourceResult<Content> {
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
            styles,
        )
    } else {
        HtmlElem::new(tag::mtext)
            .with_body(Some(ctx.body_text(item.text.clone(), props.span, styles)))
            .pack()
    })
}

fn handle_number(
    item: &NumberItem,
    ctx: &mut MathContext,
    props: &MathProperties,
    styles: StyleChain,
) -> SourceResult<Content> {
    Ok(HtmlElem::new(tag::mn)
        .with_body(Some(ctx.body_text(item.text.clone(), props.span, styles)))
        .pack())
}

fn handle_primes(
    item: &PrimesItem,
    ctx: &mut MathContext,
    props: &MathProperties,
    position: NodePosition,
    styles: StyleChain,
) -> SourceResult<Content> {
    let text: EcoString = std::iter::repeat_n(PRIME_CHAR, item.count).collect();
    Ok(make_mo(
        &text,
        props.span,
        ctx,
        props,
        position,
        Some(Form::Postfix),
        false,
        false,
        false,
        None,
        styles,
    ))
}

fn handle_fenced(
    item: &FencedItem,
    ctx: &mut MathContext,
    _props: &MathProperties,
) -> SourceResult<Content> {
    let mut children = Vec::new();
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
    // https://bugzilla.mozilla.org/show_bug.cgi?id=2018403
    Ok(HtmlElem::new(tag::mrow)
        .with_body(Some(Content::sequence(children)))
        .pack())
}

fn handle_mathml(
    item: &MathmlItem,
    ctx: &mut MathContext,
    _props: &MathProperties,
) -> SourceResult<Content> {
    let body = match &item.body {
        Some(body) => Some(Content::sequence(ctx.handle_into_nodes(body)?)),
        None => None,
    };

    let mut elem = item.elem.to_packed::<HtmlElem>().unwrap().clone();
    elem.body.set(body);
    Ok(elem.pack())
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
    styles: StyleChain,
) -> Content {
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

    // We force emitting `lspace` and `rspace` for a fraction slash (for
    // horizontal style) as it renders in browsers with spacing around it.
    // Browsers haven't updated the operator dictionary. See
    // https://phabricator.wikimedia.org/T375337
    // https://github.com/w3c/mathml-core/issues/260
    // https://github.com/w3c/mathml-core/pull/265
    // https://issues.chromium.org/issues/414588528
    // https://bugzilla.mozilla.org/show_bug.cgi?id=1586575
    let force_space = text == "/" && form.unwrap_or(initial_form) == Form::Infix;

    let form = form.filter(|f| *f != initial_form).map(|f| match f {
        Form::Prefix => "prefix",
        Form::Infix => "infix",
        Form::Postfix => "postfix",
    });

    let lspace = lspace.unwrap_or(Em::zero()).get();
    let lspace =
        (force_space || lspace != info.lspace).then(|| eco_format!("{}em", lspace));
    let rspace = rspace.unwrap_or(Em::zero()).get();
    let rspace =
        (force_space || rspace != info.rspace).then(|| eco_format!("{}em", rspace));

    let fence = (fence != is_fence(text)).then(|| eco_format!("{}", fence));
    let separator =
        (separator != is_separator(text)).then(|| eco_format!("{}", separator));

    let largeop = (largeop != info.properties.contains(Properties::LARGEOP))
        .then(|| eco_format!("{}", largeop));

    // In compact styles the browser will move top/bottom attachments to the tl
    // and br positions, so we need to explicitly disable this.
    let movablelimits = (limits
        && ctx.css.math_style == MathStyle::Compact
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
    let explicit = stretch.is_some_and(|stretch| stretch.is_explicit(stretch_axis));
    let stretchy = (explicit != info.properties.contains(Properties::STRETCHY))
        .then(|| eco_format!("{}", explicit));

    // We don't need to set `maxsize` as it is infinity by default.
    let (symmetric, minsize) = if explicit {
        let symmetric = (stretch_axis == Axis::Y
            && !info.properties.contains(Properties::SYMMETRIC))
        .then(|| eco_format!("true"));

        let minsize = stretch
            .unwrap()
            .resolve_requested(stretch_axis)
            .map(|target| target.to_css(()));

        (symmetric, minsize)
    } else {
        (None, None)
    };

    HtmlElem::new(tag::mo)
        .with_body(Some(ctx.body_text(text.into(), span, styles)))
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
        .pack()
}

/// The class for an `mtd` that is one of multiple sub-columns in a table.
fn table_mtd_class(
    index: usize,
    count: usize,
    ncols: usize,
    alternator: LeftRightAlternator,
) -> Option<EcoString> {
    if count <= 1 || ncols <= 1 || !matches!(alternator, LeftRightAlternator::Right) {
        return None;
    }

    let mut class = EcoString::from(if index % 2 == 0 {
        RIGHT_ALIGN_CLASS
    } else {
        LEFT_ALIGN_CLASS
    });
    class.push(' ');
    if index == 0 {
        class.push_str(RIGHT_FLUSH_CLASS);
    } else if index + 1 == count {
        class.push_str(LEFT_FLUSH_CLASS);
    } else {
        class.push_str(FLUSHED_CLASS);
    }
    Some(class)
}

/// Strips `form`, `lspace`, and `rspace` from a singleton `mo`.
fn strip_inert_mo_attrs(mut node: Content) -> Content {
    modify_inner_html_elem(&mut node, |mut elem| {
        if elem.tag == tag::mo {
            elem.attrs.as_option_mut().get_or_insert_default().0.retain(|(a, _)| {
                *a != attr::form && *a != attr::lspace && *a != attr::rspace
            });
        }
        elem
    });
    node
}

/// Modifies the inner [`HtmlElem`] of a content node, peeking through any
/// outer [`StyledElem`] wrapper.
///
/// Returns `true` if a modification was made.
fn modify_inner_html_elem(
    content: &mut Content,
    f: impl FnOnce(HtmlElem) -> HtmlElem,
) -> bool {
    if let Some(styled) = content.to_packed_mut::<StyledElem>() {
        return modify_inner_html_elem(&mut styled.child, f);
    }
    if content.is::<HtmlElem>() {
        let taken = std::mem::take(content);
        *content = f(taken.into_packed::<HtmlElem>().unwrap().unpack()).pack();
        return true;
    }
    false
}

/// Whether the text is one of the dedicated Unicode prime characters.
fn is_prime(text: &str) -> bool {
    let mut chars = text.chars();
    let Some(c) = chars.next() else { return false };
    chars.next().is_none() && matches!(c, '′' | '″' | '‴' | '⁗')
}

/// Whether this item is considered an embellished operator in MathML Core. See
/// [§ 3.2.4.1 Embellished operators][embellished].
///
/// [embellished]: https://www.w3.org/TR/mathml-core/#embellished-operators
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

/// Whether this item is considered a space-like element in MathML Core. See
/// [§ 3.2.5.1 Definition of space-like elements][space].
///
/// [space]: https://www.w3.org/TR/mathml-core/#definition-of-space-like-elements
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

/// Warn on ignored math items, but still handle their main part.
fn ignored_math_item(
    ctx: &mut MathContext,
    body: &MathItem,
    span: Span,
    name: &str,
) -> SourceResult<Content> {
    ctx.engine
        .sink
        .warn(warning!(span, "{} was ignored during MathML export", name));
    ctx.handle_into_node(body)
}
