use std::convert::TryFrom;
use std::fmt::Debug;
use std::hash::Hash;
use std::iter::Sum;
use std::mem;
use std::ops::{Add, AddAssign};

use super::{Property, StyleMap, Styled};
use crate::diag::StrResult;
use crate::geom::SpecAxis;
use crate::layout::{Layout, PackedNode, RootNode};
use crate::library::{
    FlowChild, FlowNode, PageNode, ParChild, ParNode, PlaceNode, SpacingKind, TextNode,
};
use crate::util::EcoString;

/// Composable representation of styled content.
///
/// This results from:
/// - anything written between square brackets in Typst
/// - any class constructor
///
/// This enum has two notable variants:
///
/// 1. A `Styled` template attaches a style map to a template. This map affects
///    the whole subtemplate. For example, a single bold word could be
///    represented as a `Styled(Text("Hello"), [TextNode::STRONG: true])`
///    template.
///
/// 2. A `Sequence` template simply combines multiple templates and will be
///    layouted as a [flow](FlowNode). So, when you write `[Hi] + [you]` in
///    Typst, this type's [`Add`] implementation is invoked and the two
///    templates are combined into a single [`Sequence`](Self::Sequence)
///    template.
///
///    A sequence may contain nested sequences (meaning this variant effectively
///    allows nodes to form trees). All nested sequences can equivalently be
///    represented as a single flat sequence, but allowing nesting doesn't hurt
///    since we can just recurse into the nested sequences during packing. Also,
///    in theory, this allows better complexity when adding (large) sequence
///    nodes (just like for a text rope).
#[derive(Debug, PartialEq, Clone, Hash)]
pub enum Template {
    /// A word space.
    Space,
    /// A line break.
    Linebreak,
    /// A paragraph break.
    Parbreak,
    /// A column break.
    Colbreak,
    /// A page break.
    Pagebreak,
    /// Plain text.
    Text(EcoString),
    /// Spacing.
    Spacing(SpecAxis, SpacingKind),
    /// An inline node.
    Inline(PackedNode),
    /// A block node.
    Block(PackedNode),
    /// A page node.
    Page(PageNode),
    /// A template with attached styles.
    Styled(Box<Self>, StyleMap),
    /// A sequence of multiple subtemplates.
    Sequence(Vec<Self>),
}

impl Template {
    /// Create an empty template.
    pub fn new() -> Self {
        Self::Sequence(vec![])
    }

    /// Create a template from an inline-level node.
    pub fn inline<T>(node: T) -> Self
    where
        T: Layout + Debug + Hash + Sync + Send + 'static,
    {
        Self::Inline(node.pack())
    }

    /// Create a template from a block-level node.
    pub fn block<T>(node: T) -> Self
    where
        T: Layout + Debug + Hash + Sync + Send + 'static,
    {
        Self::Block(node.pack())
    }

    /// Style this template with a single property.
    pub fn styled<P: Property>(mut self, key: P, value: P::Value) -> Self {
        if let Self::Styled(_, map) = &mut self {
            map.set(key, value);
            self
        } else {
            self.styled_with_map(StyleMap::with(key, value))
        }
    }

    /// Style this template with a full style map.
    pub fn styled_with_map(mut self, styles: StyleMap) -> Self {
        if styles.is_empty() {
            self
        } else if let Self::Styled(_, map) = &mut self {
            map.apply(&styles);
            self
        } else {
            Self::Styled(Box::new(self), styles)
        }
    }

    /// Style this template in monospace.
    pub fn monospaced(self) -> Self {
        self.styled(TextNode::MONOSPACE, true)
    }

    /// Lift to a type-erased block-level template.
    pub fn into_block(self) -> PackedNode {
        if let Template::Block(packed) = self {
            packed
        } else {
            let mut packer = Packer::new(false);
            packer.walk(self, StyleMap::new());
            packer.into_block()
        }
    }

    /// Lift to a root layout tree node.
    pub fn into_root(self) -> RootNode {
        let mut packer = Packer::new(true);
        packer.walk(self, StyleMap::new());
        packer.into_root()
    }

    /// Repeat this template `n` times.
    pub fn repeat(&self, n: i64) -> StrResult<Self> {
        let count = usize::try_from(n)
            .map_err(|_| format!("cannot repeat this template {} times", n))?;

        Ok(Self::Sequence(vec![self.clone(); count]))
    }
}

impl Default for Template {
    fn default() -> Self {
        Self::new()
    }
}

impl Add for Template {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::Sequence(match (self, rhs) {
            (Self::Sequence(mut left), Self::Sequence(right)) => {
                left.extend(right);
                left
            }
            (Self::Sequence(mut left), right) => {
                left.push(right);
                left
            }
            (left, Self::Sequence(mut right)) => {
                right.insert(0, left);
                right
            }
            (left, right) => {
                vec![left, right]
            }
        })
    }
}

impl AddAssign for Template {
    fn add_assign(&mut self, rhs: Self) {
        *self = mem::take(self) + rhs;
    }
}

impl Sum for Template {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        Self::Sequence(iter.collect())
    }
}

/// Packs a [`Template`] into a flow or root node.
struct Packer {
    /// Whether this packer produces a root node.
    top: bool,
    /// The accumulated page nodes.
    pages: Vec<Styled<PageNode>>,
    /// The accumulated flow children.
    flow: Builder<Styled<FlowChild>>,
    /// The accumulated paragraph children.
    par: Builder<Styled<ParChild>>,
}

impl Packer {
    /// Start a new template-packing session.
    fn new(top: bool) -> Self {
        Self {
            top,
            pages: vec![],
            flow: Builder::default(),
            par: Builder::default(),
        }
    }

    /// Finish up and return the resulting flow.
    fn into_block(mut self) -> PackedNode {
        self.parbreak(None, false);
        FlowNode(self.flow.children).pack()
    }

    /// Finish up and return the resulting root node.
    fn into_root(mut self) -> RootNode {
        self.pagebreak();
        RootNode(self.pages)
    }

    /// Consider a template with the given styles.
    fn walk(&mut self, template: Template, styles: StyleMap) {
        match template {
            Template::Space => {
                // A text space is "soft", meaning that it can be eaten up by
                // adjacent line breaks or explicit spacings.
                self.par.last.soft(Styled::new(ParChild::text(' '), styles), false);
            }
            Template::Linebreak => {
                // A line break eats up surrounding text spaces.
                self.par.last.hard();
                self.push_inline(Styled::new(ParChild::text('\n'), styles));
                self.par.last.hard();
            }
            Template::Parbreak => {
                // An explicit paragraph break is styled according to the active
                // styles (`Some(_)`) whereas paragraph breaks forced by
                // incompatibility take their styles from the preceding
                // paragraph.
                self.parbreak(Some(styles), true);
            }
            Template::Colbreak => {
                // Explicit column breaks end the current paragraph and then
                // discards the paragraph break.
                self.parbreak(None, false);
                self.make_flow_compatible(&styles);
                self.flow.children.push(Styled::new(FlowChild::Skip, styles));
                self.flow.last.hard();
            }
            Template::Pagebreak => {
                // We must set the flow styles after the page break such that an
                // empty page created by two page breaks in a row has styles at
                // all.
                self.pagebreak();
                self.flow.styles = styles;
            }
            Template::Text(text) => {
                self.push_inline(Styled::new(ParChild::text(text), styles));
            }
            Template::Spacing(SpecAxis::Horizontal, kind) => {
                // Just like a line break, explicit horizontal spacing eats up
                // surrounding text spaces.
                self.par.last.hard();
                self.push_inline(Styled::new(ParChild::Spacing(kind), styles));
                self.par.last.hard();
            }
            Template::Spacing(SpecAxis::Vertical, kind) => {
                // Explicit vertical spacing ends the current paragraph and then
                // discards the paragraph break.
                self.parbreak(None, false);
                self.make_flow_compatible(&styles);
                self.flow.children.push(Styled::new(FlowChild::Spacing(kind), styles));
                self.flow.last.hard();
            }
            Template::Inline(inline) => {
                self.push_inline(Styled::new(ParChild::Node(inline), styles));
            }
            Template::Block(block) => {
                self.push_block(Styled::new(block, styles));
            }
            Template::Page(page) => {
                if self.top {
                    self.pagebreak();
                    self.pages.push(Styled::new(page, styles));
                } else {
                    self.push_block(Styled::new(page.0, styles));
                }
            }
            Template::Styled(template, mut map) => {
                map.apply(&styles);
                self.walk(*template, map);
            }
            Template::Sequence(seq) => {
                // For a list of templates, we apply the list's styles to each
                // templates individually.
                for item in seq {
                    self.walk(item, styles.clone());
                }
            }
        }
    }

    /// Insert an inline-level element into the current paragraph.
    fn push_inline(&mut self, child: Styled<ParChild>) {
        // The child's map must be both compatible with the current page and the
        // current paragraph.
        self.make_flow_compatible(&child.map);
        self.make_par_compatible(&child.map);

        if let Some(styled) = self.par.last.any() {
            self.push_coalescing(styled);
        }

        self.push_coalescing(child);
        self.par.last.any();
    }

    /// Push a paragraph child, coalescing text nodes with compatible styles.
    fn push_coalescing(&mut self, child: Styled<ParChild>) {
        if let ParChild::Text(right) = &child.item {
            if let Some(Styled { item: ParChild::Text(left), map }) =
                self.par.children.last_mut()
            {
                if child.map.compatible::<TextNode>(map) {
                    left.0.push_str(&right.0);
                    return;
                }
            }
        }

        self.par.children.push(child);
    }

    /// Insert a block-level element into the current flow.
    fn push_block(&mut self, node: Styled<PackedNode>) {
        let placed = node.item.is::<PlaceNode>();

        self.parbreak(Some(node.map.clone()), false);
        self.make_flow_compatible(&node.map);
        self.flow.children.extend(self.flow.last.any());
        self.flow.children.push(node.map(FlowChild::Node));
        self.parbreak(None, false);

        // Prevent paragraph spacing between the placed node and the paragraph
        // below it.
        if placed {
            self.flow.last.hard();
        }
    }

    /// Advance to the next paragraph.
    fn parbreak(&mut self, break_styles: Option<StyleMap>, important: bool) {
        // Erase any styles that will be inherited anyway.
        let Builder { mut children, styles, .. } = mem::take(&mut self.par);
        for Styled { map, .. } in &mut children {
            map.erase(&styles);
        }

        // We don't want empty paragraphs.
        if !children.is_empty() {
            // The paragraph's children are all compatible with the page, so the
            // paragraph is too, meaning we don't need to check or intersect
            // anything here.
            let par = ParNode(children).pack();
            self.flow.children.extend(self.flow.last.any());
            self.flow.children.push(Styled::new(FlowChild::Node(par), styles));
        }

        // Actually styled breaks have precedence over whatever was before.
        if break_styles.is_some() {
            if let Last::Soft(_, false) = self.flow.last {
                self.flow.last = Last::Any;
            }
        }

        // For explicit paragraph breaks, `break_styles` is already `Some(_)`.
        // For page breaks due to incompatibility, we fall back to the styles
        // of the preceding thing.
        let break_styles = break_styles
            .or_else(|| self.flow.children.last().map(|styled| styled.map.clone()))
            .unwrap_or_default();

        // Insert paragraph spacing.
        self.flow
            .last
            .soft(Styled::new(FlowChild::Break, break_styles), important);
    }

    /// Advance to the next page.
    fn pagebreak(&mut self) {
        if self.top {
            self.parbreak(None, false);

            // Take the flow and erase any styles that will be inherited anyway.
            let Builder { mut children, styles, .. } = mem::take(&mut self.flow);
            for Styled { map, .. } in &mut children {
                map.erase(&styles);
            }

            let flow = FlowNode(children).pack();
            self.pages.push(Styled::new(PageNode(flow), styles));
        }
    }

    /// Break to a new paragraph if the `styles` contain paragraph styles that
    /// are incompatible with the current paragraph.
    fn make_par_compatible(&mut self, styles: &StyleMap) {
        if self.par.children.is_empty() {
            self.par.styles = styles.clone();
            return;
        }

        if !self.par.styles.compatible::<ParNode>(styles) {
            self.parbreak(Some(styles.clone()), false);
            self.par.styles = styles.clone();
            return;
        }

        self.par.styles.intersect(styles);
    }

    /// Break to a new page if the `styles` contain page styles that are
    /// incompatible with the current flow.
    fn make_flow_compatible(&mut self, styles: &StyleMap) {
        if self.flow.children.is_empty() && self.par.children.is_empty() {
            self.flow.styles = styles.clone();
            return;
        }

        if self.top && !self.flow.styles.compatible::<PageNode>(styles) {
            self.pagebreak();
            self.flow.styles = styles.clone();
            return;
        }

        self.flow.styles.intersect(styles);
    }
}

/// Container for building a flow or paragraph.
struct Builder<T> {
    /// The intersection of the style properties of all `children`.
    styles: StyleMap,
    /// The accumulated flow or paragraph children.
    children: Vec<T>,
    /// The kind of thing that was last added.
    last: Last<T>,
}

impl<T> Default for Builder<T> {
    fn default() -> Self {
        Self {
            styles: StyleMap::new(),
            children: vec![],
            last: Last::None,
        }
    }
}

/// The kind of child that was last added to a flow or paragraph. A small finite
/// state machine used to coalesce spaces.
///
/// Soft children can only exist when surrounded by `Any` children. Not at the
/// start, end or next to hard children. This way, spaces at start and end of
/// paragraphs and next to `#h(..)` goes away.
enum Last<N> {
    /// Start state, nothing there.
    None,
    /// Text or a block node or something.
    Any,
    /// Hard children: Linebreaks and explicit spacing.
    Hard,
    /// Soft children: Word spaces and paragraph breaks. These are saved here
    /// temporarily and then applied once an `Any` child appears. The boolean
    /// says whether this soft child is "important" and preferrable to other soft
    /// nodes (that is the case for explicit paragraph breaks).
    Soft(N, bool),
}

impl<N> Last<N> {
    /// Transition into the `Any` state and return a soft child to really add
    /// now if currently in `Soft` state.
    fn any(&mut self) -> Option<N> {
        match mem::replace(self, Self::Any) {
            Self::Soft(soft, _) => Some(soft),
            _ => None,
        }
    }

    /// Transition into the `Soft` state, but only if in `Any`. Otherwise, the
    /// soft child is discarded.
    fn soft(&mut self, soft: N, important: bool) {
        if matches!(
            (&self, important),
            (Self::Any, _) | (Self::Soft(_, false), true)
        ) {
            *self = Self::Soft(soft, important);
        }
    }

    /// Transition into the `Hard` state, discarding a possibly existing soft
    /// child and preventing further soft nodes from being added.
    fn hard(&mut self) {
        *self = Self::Hard;
    }
}
