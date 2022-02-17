use std::fmt::Debug;
use std::hash::Hash;
use std::iter::Sum;
use std::mem;
use std::ops::{Add, AddAssign};

use typed_arena::Arena;

use super::{
    CollapsingBuilder, Interruption, Property, Show, ShowNode, StyleMap, StyleVecBuilder,
};
use crate::diag::StrResult;
use crate::layout::{Layout, LayoutNode};
use crate::library::prelude::*;
use crate::library::{
    DecoNode, FlowChild, FlowNode, Labelling, ListItem, ListNode, PageNode, ParChild,
    ParNode, PlaceNode, SpacingKind, TextNode, ORDERED, UNDERLINE, UNORDERED,
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
/// 2. A `Sequence` template combines multiple arbitrary templates and is the
///    representation of a "flow" of content. So, when you write `[Hi] + [you]`
///    in Typst, this type's [`Add`] implementation is invoked and the two
///    [`Text`](Self::Text) templates are combined into a single
///    [`Sequence`](Self::Sequence) template.
///
///    A sequence may contain nested sequences (meaning this variant effectively
///    allows templates to form trees). All nested sequences can equivalently be
///    represented as a single flat sequence, but allowing nesting doesn't hurt
///    since we can just recurse into the nested sequences. Also, in theory,
///    this allows better complexity when adding large sequence nodes just like
///    for something like a text rope.
#[derive(PartialEq, Clone, Hash)]
pub enum Template {
    /// A word space.
    Space,
    /// A line break.
    Linebreak,
    /// Horizontal spacing.
    Horizontal(SpacingKind),
    /// Plain text.
    Text(EcoString),
    /// An inline-level node.
    Inline(LayoutNode),
    /// A paragraph break.
    Parbreak,
    /// A column break.
    Colbreak,
    /// Vertical spacing.
    Vertical(SpacingKind),
    /// A block-level node.
    Block(LayoutNode),
    /// An item in an unordered list.
    List(ListItem),
    /// An item in an ordered list.
    Enum(ListItem),
    /// A page break.
    Pagebreak,
    /// A page node.
    Page(PageNode),
    /// A node that can be realized with styles.
    Show(ShowNode),
    /// A template with attached styles.
    Styled(Arc<(Self, StyleMap)>),
    /// A sequence of multiple subtemplates.
    Sequence(Arc<Vec<Self>>),
}

impl Template {
    /// Create an empty template.
    pub fn new() -> Self {
        Self::sequence(vec![])
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

    /// Create a template from a showable node.
    pub fn show<T>(node: T) -> Self
    where
        T: Show + Debug + Hash + Sync + Send + 'static,
    {
        Self::Show(node.pack())
    }

    /// Style this template with a single property.
    pub fn styled<P: Property>(mut self, key: P, value: P::Value) -> Self {
        if let Self::Styled(styled) = &mut self {
            if let Some((_, map)) = Arc::get_mut(styled) {
                if !map.has_scoped() {
                    map.set(key, value);
                }
                return self;
            }
        }

        self.styled_with_map(StyleMap::with(key, value))
    }

    /// Style this template with a full style map.
    pub fn styled_with_map(mut self, styles: StyleMap) -> Self {
        if styles.is_empty() {
            return self;
        }

        if let Self::Styled(styled) = &mut self {
            if let Some((_, map)) = Arc::get_mut(styled) {
                if !styles.has_scoped() && !map.has_scoped() {
                    map.apply(&styles);
                    return self;
                }
            }
        }

        Self::Styled(Arc::new((self, styles)))
    }

    /// Style this template in monospace.
    pub fn monospaced(self) -> Self {
        self.styled(TextNode::MONOSPACED, true)
    }

    /// Underline this template.
    pub fn underlined(self) -> Self {
        Self::show(DecoNode::<UNDERLINE>(self))
    }

    /// Create a new sequence template.
    pub fn sequence(seq: Vec<Self>) -> Self {
        if seq.len() == 1 {
            seq.into_iter().next().unwrap()
        } else {
            Self::Sequence(Arc::new(seq))
        }
    }

    /// Repeat this template `n` times.
    pub fn repeat(&self, n: i64) -> StrResult<Self> {
        let count = usize::try_from(n)
            .map_err(|_| format!("cannot repeat this template {} times", n))?;

        Ok(Self::sequence(vec![self.clone(); count]))
    }

    /// Layout this template into a collection of pages.
    pub fn layout(&self, vm: &mut Vm) -> TypResult<Vec<Arc<Frame>>> {
        let sya = Arena::new();
        let tpa = Arena::new();

        let mut builder = Builder::new(&sya, &tpa, true);
        let styles = StyleChain::new(vm.styles);
        builder.process(vm, self, styles)?;
        builder.finish(vm, styles)?;

        let mut frames = vec![];
        let (pages, shared) = builder.pages.unwrap().finish();
        for (page, map) in pages.iter() {
            frames.extend(page.layout(vm, map.chain(&shared))?);
        }

        Ok(frames)
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
            (Self::Sequence(mut lhs), Self::Sequence(rhs)) => {
                let mutable = Arc::make_mut(&mut lhs);
                match Arc::try_unwrap(rhs) {
                    Ok(vec) => mutable.extend(vec),
                    Err(rc) => mutable.extend(rc.iter().cloned()),
                }
                lhs
            }
            (Self::Sequence(mut lhs), rhs) => {
                Arc::make_mut(&mut lhs).push(rhs);
                lhs
            }
            (lhs, Self::Sequence(mut rhs)) => {
                Arc::make_mut(&mut rhs).insert(0, lhs);
                rhs
            }
            (lhs, rhs) => Arc::new(vec![lhs, rhs]),
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
        Self::sequence(iter.collect())
    }
}

impl Layout for Template {
    fn layout(
        &self,
        vm: &mut Vm,
        regions: &Regions,
        styles: StyleChain,
    ) -> TypResult<Vec<Constrained<Arc<Frame>>>> {
        let sya = Arena::new();
        let tpa = Arena::new();

        let mut builder = Builder::new(&sya, &tpa, false);
        builder.process(vm, self, styles)?;
        builder.finish(vm, styles)?;

        let (flow, shared) = builder.flow.finish();
        FlowNode(flow).layout(vm, regions, shared)
    }

    fn pack(self) -> LayoutNode {
        match self {
            Template::Block(node) => node,
            other => LayoutNode::new(other),
        }
    }
}

/// Builds a flow or page nodes from a template.
struct Builder<'a> {
    /// An arena where intermediate style chains are stored.
    sya: &'a Arena<StyleChain<'a>>,
    /// An arena where intermediate templates are stored.
    tpa: &'a Arena<Template>,
    /// The already built page runs.
    pages: Option<StyleVecBuilder<'a, PageNode>>,
    /// The currently built list.
    list: Option<ListBuilder<'a>>,
    /// The currently built flow.
    flow: CollapsingBuilder<'a, FlowChild>,
    /// The currently built paragraph.
    par: CollapsingBuilder<'a, ParChild>,
    /// Whether to keep the next page even if it is empty.
    keep_next: bool,
}

impl<'a> Builder<'a> {
    /// Prepare the builder.
    fn new(sya: &'a Arena<StyleChain<'a>>, tpa: &'a Arena<Template>, top: bool) -> Self {
        Self {
            sya,
            tpa,
            pages: top.then(|| StyleVecBuilder::new()),
            flow: CollapsingBuilder::new(),
            list: None,
            par: CollapsingBuilder::new(),
            keep_next: true,
        }
    }

    /// Process a template.
    fn process(
        &mut self,
        vm: &mut Vm,
        template: &'a Template,
        styles: StyleChain<'a>,
    ) -> TypResult<()> {
        if let Some(builder) = &mut self.list {
            match template {
                Template::Space => {
                    builder.staged.push((template, styles));
                    return Ok(());
                }
                Template::Parbreak => {
                    builder.staged.push((template, styles));
                    return Ok(());
                }
                Template::List(item) if builder.labelling == UNORDERED => {
                    builder.wide |=
                        builder.staged.iter().any(|&(t, _)| *t == Template::Parbreak);
                    builder.staged.clear();
                    builder.items.push(item.clone());
                    return Ok(());
                }
                Template::Enum(item) if builder.labelling == ORDERED => {
                    builder.wide |=
                        builder.staged.iter().any(|&(t, _)| *t == Template::Parbreak);
                    builder.staged.clear();
                    builder.items.push(item.clone());
                    return Ok(());
                }
                _ => self.finish_list(vm)?,
            }
        }

        match template {
            Template::Space => {
                self.par.weak(ParChild::Text(' '.into()), 0, styles);
            }
            Template::Linebreak => {
                self.par.destructive(ParChild::Text('\n'.into()), styles);
            }
            Template::Horizontal(kind) => {
                let child = ParChild::Spacing(*kind);
                if kind.is_fractional() {
                    self.par.destructive(child, styles);
                } else {
                    self.par.ignorant(child, styles);
                }
            }
            Template::Text(text) => {
                self.par.supportive(ParChild::Text(text.clone()), styles);
            }
            Template::Inline(node) => {
                self.par.supportive(ParChild::Node(node.clone()), styles);
            }
            Template::Parbreak => {
                self.finish_par(styles);
                self.flow.weak(FlowChild::Parbreak, 1, styles);
            }
            Template::Colbreak => {
                self.finish_par(styles);
                self.flow.destructive(FlowChild::Colbreak, styles);
            }
            Template::Vertical(kind) => {
                self.finish_par(styles);
                let child = FlowChild::Spacing(*kind);
                if kind.is_fractional() {
                    self.flow.destructive(child, styles);
                } else {
                    self.flow.ignorant(child, styles);
                }
            }
            Template::Block(node) => {
                self.finish_par(styles);
                let child = FlowChild::Node(node.clone());
                if node.is::<PlaceNode>() {
                    self.flow.ignorant(child, styles);
                } else {
                    self.flow.supportive(child, styles);
                }
                self.finish_par(styles);
            }
            Template::List(item) => {
                self.list = Some(ListBuilder {
                    styles,
                    labelling: UNORDERED,
                    items: vec![item.clone()],
                    wide: false,
                    staged: vec![],
                });
            }
            Template::Enum(item) => {
                self.list = Some(ListBuilder {
                    styles,
                    labelling: ORDERED,
                    items: vec![item.clone()],
                    wide: false,
                    staged: vec![],
                });
            }
            Template::Pagebreak => {
                self.finish_page(true, true, styles);
            }
            Template::Page(page) => {
                self.finish_page(false, false, styles);
                if let Some(pages) = &mut self.pages {
                    pages.push(page.clone(), styles);
                }
            }
            Template::Show(node) => {
                let template = node.show(vm, styles)?;
                let stored = self.tpa.alloc(template);
                self.process(vm, stored, styles.unscoped(node.id()))?;
            }
            Template::Styled(styled) => {
                let (sub, map) = styled.as_ref();
                let stored = self.sya.alloc(styles);
                let styles = map.chain(stored);

                let interruption = map.interruption();
                match interruption {
                    Some(Interruption::Page) => self.finish_page(false, true, styles),
                    Some(Interruption::Par) => self.finish_par(styles),
                    None => {}
                }

                self.process(vm, sub, styles)?;

                match interruption {
                    Some(Interruption::Page) => self.finish_page(true, false, styles),
                    Some(Interruption::Par) => self.finish_par(styles),
                    None => {}
                }
            }
            Template::Sequence(seq) => {
                for sub in seq.iter() {
                    self.process(vm, sub, styles)?;
                }
            }
        }

        Ok(())
    }

    /// Finish the currently built paragraph.
    fn finish_par(&mut self, styles: StyleChain<'a>) {
        let (par, shared) = mem::take(&mut self.par).finish();
        if !par.is_empty() {
            let node = ParNode(par).pack();
            self.flow.supportive(FlowChild::Node(node), shared);
        }
        self.flow.weak(FlowChild::Leading, 0, styles);
    }

    /// Finish the currently built list.
    fn finish_list(&mut self, vm: &mut Vm) -> TypResult<()> {
        let ListBuilder { styles, labelling, items, wide, staged } =
            match self.list.take() {
                Some(list) => list,
                None => return Ok(()),
            };

        let template = match labelling {
            UNORDERED => Template::show(ListNode::<UNORDERED> { items, wide, start: 1 }),
            ORDERED | _ => Template::show(ListNode::<ORDERED> { items, wide, start: 1 }),
        };

        let stored = self.tpa.alloc(template);
        self.process(vm, stored, styles)?;
        for (template, styles) in staged {
            self.process(vm, template, styles)?;
        }

        Ok(())
    }

    /// Finish the currently built page run.
    fn finish_page(&mut self, keep_last: bool, keep_next: bool, styles: StyleChain<'a>) {
        self.finish_par(styles);
        if let Some(pages) = &mut self.pages {
            let (flow, shared) = mem::take(&mut self.flow).finish();
            if !flow.is_empty() || (keep_last && self.keep_next) {
                let styles = if flow.is_empty() { styles } else { shared };
                let node = PageNode(FlowNode(flow).pack());
                pages.push(node, styles);
            }
        }
        self.keep_next = keep_next;
    }

    /// Finish everything.
    fn finish(&mut self, vm: &mut Vm, styles: StyleChain<'a>) -> TypResult<()> {
        self.finish_list(vm)?;
        self.finish_page(true, false, styles);
        Ok(())
    }
}

/// Builds an unordered or ordered list from items.
struct ListBuilder<'a> {
    styles: StyleChain<'a>,
    labelling: Labelling,
    items: Vec<ListItem>,
    wide: bool,
    staged: Vec<(&'a Template, StyleChain<'a>)>,
}

impl Debug for Template {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::Space => f.pad("Space"),
            Self::Linebreak => f.pad("Linebreak"),
            Self::Horizontal(kind) => write!(f, "Horizontal({kind:?})"),
            Self::Text(text) => write!(f, "Text({text:?})"),
            Self::Inline(node) => {
                f.write_str("Inline(")?;
                node.fmt(f)?;
                f.write_str(")")
            }
            Self::Parbreak => f.pad("Parbreak"),
            Self::Colbreak => f.pad("Colbreak"),
            Self::Vertical(kind) => write!(f, "Vertical({kind:?})"),
            Self::Block(node) => {
                f.write_str("Block(")?;
                node.fmt(f)?;
                f.write_str(")")
            }
            Self::List(item) => {
                f.write_str("- ")?;
                item.body.fmt(f)
            }
            Self::Enum(item) => {
                if let Some(number) = item.number {
                    write!(f, "{}", number)?;
                }
                f.write_str(". ")?;
                item.body.fmt(f)
            }
            Self::Pagebreak => f.pad("Pagebreak"),
            Self::Page(page) => page.fmt(f),
            Self::Show(node) => {
                f.write_str("Show(")?;
                node.fmt(f)?;
                f.write_str(")")
            }
            Self::Styled(styled) => {
                let (sub, map) = styled.as_ref();
                map.fmt(f)?;
                sub.fmt(f)
            }
            Self::Sequence(seq) => f.debug_list().entries(seq.iter()).finish(),
        }
    }
}
