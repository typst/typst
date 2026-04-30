//! This modules tries to resolve a [`Stylesheet`] from the CSS properties that
//! are specified for each element. Existing classes, ARIA roles, and tags are
//! used where possible, otherwise custom classes are generated and assigned to
//! elements.
//!
//! # Steps
//! 1. Collect all elements
//!     - build a bidirectional traversable tree structure
//!     - intern all property key-value pairs and associate them with the elements
//!     - intern all simple selectors and associate them with the elements
//!
//! 2. Group properties such that:
//!     - all groups consist of properties that are specified on the exact same
//!       set of elements
//!     - if the two properties don't have the exact same set of elements, split
//!       up the properties into different groups
//!
//! 3. Find simple selectors that can precisely target groups
//!     - a group is targetable by a selector, when all targeted elements have a
//!       property in that group
//!
//! 4. Find selector lists to fully identify property groups. For each group:
//!     - filter out selectors that aren't able to precisely target a group
//!     - try to find a minimal selector list that covers all elements of the group
//!     - generate and assign a class for group elements that couldn't be targeted

use std::cell::{Cell, LazyCell};
use std::fmt::{Debug, Display, Write as _};
use std::hash::Hash;
use std::ops::{Deref, DerefMut};

use bumpalo::Bump;
use bumpalo::collections::{CollectIn, String as BumpString, Vec as BumpVec};
use ecow::{EcoString, eco_format};
use indexmap::{IndexMap, IndexSet};
use rustc_hash::FxBuildHasher;
use typst_utils::{Id, IdMap, IdQueue, IdRange, IdVec, KeyFor, ListSet};

use crate::css::{Properties, Property};
use crate::{HtmlAttrs, HtmlElement, HtmlNode, HtmlTag, attr};

#[derive(Default)]
pub struct Stylesheet {
    styles: IndexMap<EcoString, Properties, FxBuildHasher>,
}

impl Stylesheet {
    pub fn is_empty(&self) -> bool {
        self.styles.is_empty()
    }

    /// Format the CSS stylesheet.
    pub fn display(&self) -> impl Display {
        typst_utils::display(|f| {
            for (selector, props) in self.styles.iter() {
                writeln!(f, "{selector} {{")?;
                for Property { name, value } in props.iter() {
                    writeln!(f, "  {name}: {value};")?;
                }
                writeln!(f, "}}")?;
            }
            Ok(())
        })
    }
}

/// Resolve a stylesheet from the CSS styles specified for each element.
pub fn resolve_stylesheet(root: &mut HtmlElement) -> Stylesheet {
    let bump = Bump::new();
    let mut temp = Bump::new();

    let (mut elems, props, mut selectors) = collect_elems(&bump, root);
    let groups = collect_prop_groups(&bump, &mut temp, &elems, &props);
    find_simple_group_selectors(&bump, &mut temp, &mut elems, &props, &mut selectors);
    identify_groups(&bump, &mut temp, &mut elems, &props, &mut selectors, &groups)
}

/// A tree of HTML elements.
#[derive(Default)]
struct ElemTree<'a, 'b> {
    /// A depth-first list of elements.
    inner: IdVec<Elem<'a, 'b>>,
    /// A breadth-first list of the element IDs, which is useful to obtain direct
    /// children of an element, instead of all its descendants.
    ///
    /// # Example
    /// Consider the following tree:
    /// ```txt
    ///       A
    ///     /   \
    ///    B     C
    ///   /|\   / \
    ///  D E F G   H
    ///   / \
    ///   I J
    /// ```
    ///
    /// ## Depth-first
    /// In the depth-first list all descendants of a node are contiguous:
    /// ```txt
    /// A B D E I J F C G H
    ///     ^^^^^^^^^
    ///     descendants of B
    /// ```
    /// But the direct children may be separated:
    /// ```txt
    /// A B D E I J F C G H
    ///   ^-----------^
    ///   children of A
    /// ```
    ///
    /// ## Breadth-first
    /// In the breadth-first list the descendants may be separated:
    /// ```txt
    /// A B C D E F G H I J
    ///       ^^^^^-----^^^
    ///       descendants of B
    /// ```
    /// But the direct children are contiguous:
    /// ```txt
    /// A B C D E F G H I J
    ///   ^^^
    ///   children of A
    /// ```
    children: IdVec<ElemChild>,
}

impl<'a, 'b> ElemTree<'a, 'b> {
    fn new(num_elems: usize) -> Self {
        Self {
            inner: IdVec::with_capacity(num_elems),
            children: IdVec::from_iter(std::iter::repeat_n(
                ElemChild { id: Id::new(u32::MAX as usize - 1) },
                num_elems,
            )),
        }
    }

    /// Returns an iterator of the element's ancestors.
    fn ancestors(&self, elem: ElemId) -> impl Iterator<Item = &Elem<'a, 'b>> {
        let mut current = self.get(elem);
        std::iter::from_fn(move || {
            let parent = current.parent?;
            current = self.get(parent);
            Some(current)
        })
    }

    /// Returns an iterator over the direct children of an element.
    fn children(&self, elem_id: ElemId) -> impl Iterator<Item = &Elem<'a, 'b>> {
        let range = self.get(elem_id).children;
        let ids = self.children.get_range(range);
        ids.iter().map(|child| self.get(child.id))
    }
}

impl<'a, 'b> Deref for ElemTree<'a, 'b> {
    type Target = IdVec<Elem<'a, 'b>>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<'a, 'b> DerefMut for ElemTree<'a, 'b> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

type ElemChildId = Id<ElemChild>;

#[derive(Copy, Clone)]
struct ElemChild {
    id: ElemId,
}

type ElemId = Id<ElemKey>;
struct ElemKey;
impl KeyFor<Elem<'_, '_>> for ElemKey {}

/// The whole [`HtmlElement`] cannot be mutably borrowed, because that would
/// also include its children.
#[derive(Debug)]
struct Elem<'a, 'b> {
    parent: Option<ElemId>,
    /// The range of all descendants: the complete sub-tree of all children.
    ///
    /// NOTE: This is late initialized after all descendants have been collected.
    descendants: IdRange<ElemId>,
    /// The range of direct children.
    ///
    /// NOTE: This is late initialized after all descendants have been collected.
    children: IdRange<ElemChildId>,
    attrs: &'a mut HtmlAttrs,
    props: ListSet<&'b [PropId]>,
    /// All property groups this element is part of.
    ///
    /// NOTE: This is late initialized in [`find_simple_group_selectors`].
    groups: ListSet<&'b [GroupId]>,
    /// The simple selectors that this element matches.
    /// This list is deduplicated.
    /// The element is not necessarily [`Targetable`] by them.
    simple_selectors: ListSet<&'b [SimpleSelectorId]>,
}

struct Collector<'a, 'b> {
    bump: &'b Bump,
    elems: ElemTree<'a, 'b>,
    props: IdMap<Property, PropRefs>,
    selectors: Selectors<'b>,
    /// The cursors for each layers.
    layer_cursors: Vec<usize>,
}

impl<'a, 'b> Collector<'a, 'b> {
    fn new(
        bump: &'b Bump,
        props: IdMap<Property, PropRefs>,
        selectors: IdMap<SimpleSelector<'b>, SimpleSelectorRefs<'b>>,
        layers: Vec<usize>,
    ) -> Self {
        let mut cursor = 0;
        let layer_cursors = layers
            .into_iter()
            .map(|n| {
                let layer = cursor;
                cursor += n;
                layer
            })
            .collect();

        Self {
            bump,
            elems: ElemTree::new(cursor),
            props,
            selectors: Selectors::new(selectors),
            layer_cursors,
        }
    }

    fn new_elem(
        &mut self,
        id: ElemId,
        layer: usize,
        parent: Option<ElemId>,
        props: ListSet<&'b [PropId]>,
        simple_selectors: ListSet<&'b [SimpleSelectorId]>,
        attrs: &'a mut HtmlAttrs,
    ) {
        let first_child_id = self.next_child_id(layer + 1);
        let elem = Elem {
            parent,
            descendants: IdRange::after(id),
            children: IdRange::at(first_child_id),
            attrs,
            props,
            groups: ListSet::from_sorted(&[]),
            simple_selectors,
        };
        let elem_id = self.elems.push(elem);
        debug_assert_eq!(id, elem_id.downcast());

        // Push the child ref.
        let self_child_id = self.next_child_id(layer);
        self.elems.children.get_mut(self_child_id).id = id;
        self.layer_cursors[layer] += 1;
    }

    /// Returns the next ID inside the breadth-first children list.
    fn next_child_id(&mut self, layer: usize) -> ElemChildId {
        self.layer_cursors
            .get(layer)
            .map(|idx| Id::new(*idx))
            .unwrap_or(self.elems.children.next_id())
    }
}

/// Collect all elements, and intern their CSS properties and selectors.
fn collect_elems<'a, 'b>(
    bump: &'b Bump,
    root: &'a mut HtmlElement,
) -> (ElemTree<'a, 'b>, IdMap<Property, PropRefs>, Selectors<'b>) {
    let mut ctx = pre_collect_elems(bump, root);

    collect_elem(&mut ctx, None, 0, root);

    (ctx.elems, ctx.props, ctx.selectors)
}

/// Collect all properties and selectors in a pre-pass and sort them. This
/// guarantees that the order of property IDs matches the order of items in
/// `css::Properties`. And guarantees that sorting lists of selector and
/// property IDs by their numeric value, is the same as looking up the
/// corresponding values.
fn pre_collect_elems<'a, 'b>(bump: &'b Bump, root: &HtmlElement) -> Collector<'a, 'b> {
    let mut props = IndexMap::default();
    let mut selectors = IndexMap::default();
    let mut layers = Vec::new();

    pre_collect_elem(bump, &mut props, &mut selectors, &mut layers, root, 0);

    props.sort_by_key(|p, _| p.name);
    selectors.sort_by_key(|s, _| *s);

    Collector::new(bump, IdMap::from(props), IdMap::from(selectors), layers)
}

fn pre_collect_elem<'b>(
    bump: &'b Bump,
    props: &mut IndexMap<Property, PropRefs, FxBuildHasher>,
    selectors: &mut IndexMap<SimpleSelector<'b>, SimpleSelectorRefs<'b>, FxBuildHasher>,
    layers: &mut Vec<usize>,
    element: &HtmlElement,
    layer: usize,
) {
    // Intern properties.
    for prop in element.css.iter() {
        props.entry(prop.clone()).or_default();
    }

    // Intern selectors.
    selectors.entry(SimpleSelector::ty(element.tag)).or_default();
    if let Some(role) = element.attrs.get(attr::role) {
        intern_str_selector(bump, selectors, role, |str| SimpleSelector::Role(str));
    }
    if let Some(classes) = element.attrs.get(attr::class) {
        for class in classes.split_whitespace() {
            intern_str_selector(bump, selectors, class, |str| SimpleSelector::Class(str));
        }
    }

    // Count elements per layer.
    if layer >= layers.len() {
        layers.resize(layer + 1, 0);
    }
    layers[layer] += 1;

    // Traverse children.
    for child in element.children.iter() {
        if let HtmlNode::Element(child) = child {
            pre_collect_elem(bump, props, selectors, layers, child, layer + 1);
        }
    }
}

/// Intern the selector and lazily allocate the inner string.
fn intern_str_selector<'b>(
    bump: &'b Bump,
    selectors: &mut IndexMap<SimpleSelector<'b>, SimpleSelectorRefs<'b>, FxBuildHasher>,
    name: &str,
    kind: impl for<'a> Fn(&'a str) -> SimpleSelector<'a>,
) {
    if !selectors.contains_key(&kind(name)) {
        let selector = kind(bump.alloc_str(name));
        selectors.insert(selector, SimpleSelectorRefs::default());
    }
}

/// Collect an element, including its properties an selectors.
///
/// Simultaneously build both a depth-first and breadth-first list of elements,
/// so an element's direct children and all its descendants can be efficiently
/// queried.
fn collect_elem<'a, 'b>(
    ctx: &mut Collector<'a, 'b>,
    parent: Option<ElemId>,
    layer: usize,
    element: &'a mut HtmlElement,
) {
    let id = ctx.elems.next_id();
    let props = collect_props(ctx, id, &element.css);
    let selectors = collect_simple_selectors(ctx, id, element);
    ctx.new_elem(id, layer, parent, props, selectors, &mut element.attrs);

    // Collect children.
    let start = ctx.elems.next_id();
    let child_start = ctx.next_child_id(layer + 1);

    for node in element.children.make_mut() {
        if let HtmlNode::Element(element) = node {
            collect_elem(ctx, Some(id), layer + 1, element);
        }
    }

    let end = ctx.elems.next_id();
    let child_end = ctx.next_child_id(layer + 1);

    ctx.elems.get_mut(id).descendants = IdRange::new(start, end);
    ctx.elems.get_mut(id).children = IdRange::new(child_start, child_end);
}

fn collect_props<'a, 'b>(
    ctx: &mut Collector<'a, 'b>,
    elem_id: ElemId,
    props: &'a Properties,
) -> ListSet<&'b [PropId]> {
    let props = props
        .iter()
        .map(|prop| {
            let prop_id = (ctx.props.lookup_id(prop))
                .expect("all properties should have been interned in the pre-pass");
            ctx.props.get_id_mut(prop_id).elems.push(elem_id);
            prop_id
        })
        .collect_in::<BumpVec<_>>(ctx.bump)
        .into_bump_slice();

    // Properties are interned in a pre-pass and the properties inside the
    // `css::Properties` struct are also sorted. Thus, the IDs of the interned
    // properties are also sorted.
    ListSet::from_sorted(props)
}

fn collect_simple_selectors<'a, 'b>(
    ctx: &mut Collector<'a, 'b>,
    elem_id: ElemId,
    element: &HtmlElement,
) -> ListSet<&'b [SimpleSelectorId]> {
    let mut selectors = BumpVec::new_in(ctx.bump);

    let lookup = |s| {
        ctx.selectors
            .simple
            .lookup_id(&s)
            .expect("all selectors should have been interned in the pre-pass")
    };

    selectors.push(lookup(SimpleSelector::ty(element.tag)));

    if let Some(role) = element.attrs.get(attr::role) {
        selectors.push(lookup(SimpleSelector::Role(role)));
    }

    let classes_start = selectors.len();
    if let Some(classes) = element.attrs.get(attr::class) {
        for class in classes.split_whitespace() {
            selectors.push(lookup(SimpleSelector::Class(class)));
        }
    }

    // Deduplicate the class selectors, so the element is only added once to the
    // selector references.
    selectors[classes_start..].sort_unstable();
    selectors.dedup();

    for &selector in selectors.iter() {
        let refs = ctx.selectors.simple.get_id_mut(selector);
        refs.elems.push(elem_id);
    }

    // The list is sorted, because simple selectors are first sorted by kind, so
    // the type and role selectors will always be ordered before the class ones.
    ListSet::from_sorted(selectors.into_bump_slice())
}

type PropId = Id<PropRefs>;

#[derive(Default)]
struct PropRefs {
    /// The elements are inserted in iteration order and thus the [`ElemId`] is
    /// monotonically increasing.
    elems: Vec<ElemId>,

    /// NOTE: This is late initialized and updated during property grouping, but
    /// is guaranteed to be set after [`collect_prop_groups`].
    group: Cell<Option<GroupId>>,
}

impl PropRefs {
    fn set_group(&self, group: GroupId) {
        self.group.set(Some(group));
    }

    /// This must only be used after the properties have been grouped.
    #[cfg_attr(debug_assertions, track_caller)]
    fn group(&self) -> GroupId {
        self.group.get().expect("group should be set after property grouping")
    }
}

type GroupId = Id<GroupKey>;
struct GroupKey;
impl KeyFor<PropGroup<'_>> for GroupKey {}

/// A group of properties.
#[derive(Debug)]
struct PropGroup<'b> {
    /// The properties in this group.
    /// Due to the way this is constructed, all properties in the same group
    /// have the exact same set of elements, and the list is never empty.
    props: BumpVec<'b, PropId>,
}

impl<'a> PropGroup<'a> {
    fn new(props: BumpVec<'a, PropId>) -> Self {
        Self { props }
    }

    fn first(&self) -> PropId {
        *self.props.first().unwrap()
    }
}

struct Groups<'b, 'c> {
    props: &'c IdMap<Property, PropRefs>,
    groups: IdVec<PropGroup<'b>>,
}

impl<'b> Groups<'b, '_> {
    /// Create a new group and update the [`Self::group_by_prop`] lookup table.
    fn create_group(&mut self, props: BumpVec<'b, PropId>) {
        let group_id = self.groups.next_id();
        for &prop in props.iter() {
            self.props.get_id(prop).set_group(group_id);
        }
        self.groups.push(PropGroup::new(props));
    }
}

fn collect_prop_groups<'a, 'b>(
    bump: &'b Bump,
    temp: &mut Bump,
    elems_tree: &ElemTree<'a, 'b>,
    props: &IdMap<Property, PropRefs>,
) -> IdVec<PropGroup<'b>> {
    let mut ctx = Groups { props, groups: IdVec::new() };

    for elem in elems_tree.iter() {
        temp.reset();

        let mut existing_props = BumpVec::from_iter_in(elem.props.iter().copied(), temp);
        let mut existing_groups = BumpVec::new_in(temp);
        let new_props = existing_props
            .drain_filter(|prop_id| {
                let group = props.get_id(*prop_id).group.get();
                if let Some(group_id) = group {
                    existing_groups.push(group_id);
                }
                group.is_none()
            })
            .collect_in::<BumpVec<_>>(bump);
        existing_props.sort_unstable();

        existing_groups.sort_unstable();
        existing_groups.dedup();

        // Initially create a new group with all newly seen properties of a
        // group.
        if !new_props.is_empty() {
            ctx.create_group(new_props);
        }

        // Incrementally split up groups. If an element only has a subset of the
        // group's properties, split the group.
        for &group_id in existing_groups.iter() {
            // Found an existing group, check if it needs to be split up.
            let group = ctx.groups.get_mut(group_id);

            // Remove properties that aren't shared between the group and the
            // element. Iterate the two sorted lists in tandem.
            let mut existing_props = existing_props.iter().peekable();
            let split_off_props = group
                .props
                .drain_filter(|group_prop| {
                    // Skip all property IDs smaller than the target one.
                    while existing_props.next_if(|&g| g < group_prop).is_some() {}

                    // Filter the property if it doesn't already exist.
                    existing_props.next_if(|&g| g == group_prop).is_none()
                })
                .collect_in::<BumpVec<_>>(bump);

            // Create a new group with the split off properties.
            if !split_off_props.is_empty() {
                ctx.create_group(split_off_props);
            }
        }
    }

    ctx.groups
}

#[derive(Default)]
struct Selectors<'b> {
    simple: IdMap<SimpleSelector<'b>, SimpleSelectorRefs<'b>>,
    /// Complex selectors are computed ad-hoc during [`identify_group`].
    complex: IdMap<Selector<'b>, ComplexSelectorRefs<'b>>,
}

impl<'b> Selectors<'b> {
    fn new(simple: IdMap<SimpleSelector<'b>, SimpleSelectorRefs<'b>>) -> Self {
        Self { simple, complex: IdMap::new() }
    }

    fn get<U: KeyFor<SelectorKey>>(&self, id: Id<U>) -> Selector<'b> {
        let id = id.upcast();
        // Selectors are split between two lists. IDs below a certain range
        // refer to simple selectors, the ones above refer to complex ones.
        if id.idx() < self.simple.len() {
            // The downcast is valid, because we've checked the ID is in range.
            let id = id.downcast::<SimpleSelectorKey>();
            Selector::Simple(*self.simple.get_id_key(id))
        } else {
            let id = ComplexSelectorId::new(id.idx() - self.simple.len());
            *self.complex.get_id_key(id)
        }
    }

    /// A niceness metric for the selector.
    fn score(&self, id: SelectorId) -> u32 {
        let selector = self.get(id);

        match selector {
            Selector::Simple(simple) => {
                // Score simple selectors higher than complex ones.
                simple.score() << 8
            }
            Selector::Descendant(parent, child) | Selector::Child(parent, child) => {
                // Descendant and child selectors are score equally.
                let parent = self.simple.get_id_key(parent);
                let child = self.simple.get_id_key(child);
                (parent.score() << 4) + child.score()
            }
        }
    }

    fn alphanumeric_cmp(&self, a: &Selector<'b>, b: &Selector<'b>) -> std::cmp::Ordering {
        match (a, b) {
            (Selector::Simple(a), Selector::Simple(b)) => a.alphanumeric_cmp(b),
            (&Selector::Descendant(a1, a2), &Selector::Descendant(b1, b2))
            | (&Selector::Child(a1, a2), &Selector::Child(b1, b2)) => {
                let cmp = |a, b| {
                    let a = self.simple.get_id_key(a);
                    let b = self.simple.get_id_key(b);
                    a.alphanumeric_cmp(b)
                };
                cmp(a1, b1).then_with(|| cmp(a2, b2))
            }
            // For different selectors, order them by kind.
            _ => a.cmp(b),
        }
    }

    fn display_list(&self, list: &[Selector<'b>]) -> impl Display {
        typst_utils::display(move |f| {
            for (i, selector) in list.iter().enumerate() {
                if i > 0 {
                    f.write_str(", ")?;
                }
                self.display(*selector).fmt(f)?;
            }
            Ok(())
        })
    }

    fn display(&self, selector: Selector<'b>) -> impl Display {
        typst_utils::display(move |f| match selector {
            Selector::Simple(selector) => write!(f, "{selector}"),
            Selector::Descendant(parent, child) => {
                let parent = self.simple.get_id_key(parent);
                let child = self.simple.get_id_key(child);
                write!(f, "{parent} {child}")
            }
            Selector::Child(parent, child) => {
                let parent = self.simple.get_id_key(parent);
                let child = self.simple.get_id_key(child);
                write!(f, "{parent} > {child}")
            }
        })
    }
}

type SelectorId = Id<SelectorKey>;
struct SelectorKey;

type SimpleSelectorId = Id<SimpleSelectorKey>;
struct SimpleSelectorKey;
impl KeyFor<SimpleSelectorRefs<'_>> for SimpleSelectorKey {}
impl KeyFor<SelectorKey> for SimpleSelectorKey {}

type ComplexSelectorId = Id<ComplexSelectorKey>;
struct ComplexSelectorKey;
impl KeyFor<ComplexSelectorRefs<'_>> for ComplexSelectorKey {}

#[derive(Default)]
struct SimpleSelectorRefs<'b> {
    elems: Vec<ElemId>,
    targetable: Targetable<'b>,
}

#[derive(Default)]
struct ComplexSelectorRefs<'b> {
    targetable: Targetable<'b>,
}

/// Stores a list of groups that are targetable by a selectors.
///
/// A [`PropGroup`] is targetable by a [`Selector`] `s`, if all [`Elem`]s that
/// are targeted by `s` are part of the group. By extension, if there is a
/// single element that isn't part of the property group, the selector can't be
/// used to target the group. And all selectors that select an element without
/// any properties, can be ruled out directly.
#[derive(Default, Clone)]
struct Targetable<'b> {
    /// Sparse sorted list of groups that can be targeted by the selectors.
    /// This is the intersection of all elements matching the selector.
    groups: Option<BumpVec<'b, GroupId>>,
}

impl<'b> Targetable<'b> {
    /// Whether a group is targetable by the selector.
    fn is(&self, group: GroupId) -> bool {
        let Some(groups) = &self.groups else { return false };
        groups.binary_search(&group).is_ok()
    }

    /// Computes the intersection between the targetable groups and the element
    /// groups. If there is no targetable list of groups yet, the element's
    /// groups are bump allocated and used as the initial targetable set.
    fn intersect(&mut self, bump: &'b Bump, elem_groups: &[GroupId]) {
        let Some(targetable) = &mut self.groups else {
            let groups = BumpVec::from_iter_in(elem_groups.iter().copied(), bump);
            self.groups = Some(groups);
            return;
        };

        // Iterate the two sorted lists in tandem.
        let mut elem_groups = elem_groups.iter().copied().peekable();
        targetable.retain(|&target| {
            // Skip all group IDs smaller than the target one.
            while elem_groups.next_if(|&g| g < target).is_some() {}

            // Retain the target if it is also inside the element's groups.
            elem_groups.next_if(|&g| g == target).is_some()
        });
    }
}

/// A CSS selector.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
enum Selector<'a> {
    /// A simple selector.
    Simple(SimpleSelector<'a>),
    /// E.g. `.ancestor .child`
    Descendant(SimpleSelectorId, SimpleSelectorId),
    /// E.g. `.parent > .child`
    Child(SimpleSelectorId, SimpleSelectorId),
}

impl<'a> From<SimpleSelector<'a>> for Selector<'a> {
    fn from(v: SimpleSelector<'a>) -> Self {
        Self::Simple(v)
    }
}

/// A CSS selector.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
enum SimpleSelector<'a> {
    /// A HTML tag.
    /// E.g. `li`
    Type(HtmlTagKey),
    /// An ARIA role.
    /// E.g. `[role=hidden]`
    Role(&'a str),
    /// A CSS class.
    /// E.g. `.class`
    Class(&'a str),
}

impl SimpleSelector<'_> {
    fn ty(tag: HtmlTag) -> Self {
        Self::Type(HtmlTagKey(tag))
    }

    /// A niceness metric for the selector.
    fn score(&self) -> u32 {
        match self {
            // TODO: Consider having categories of tags that might score higher.
            Self::Type(_) => 1,
            Self::Role(_) => 2,
            Self::Class(_) => 3,
        }
    }

    fn alphanumeric_cmp(&self, other: &Self) -> std::cmp::Ordering {
        match (self, other) {
            (Self::Type(a), Self::Type(b)) => {
                // The `Ord` implementation of `HtmlTagKey` doesn't sort by
                // string contents for performance reasons. For the CSS
                // generation it's preferable to sort alphabetically even though
                // it will be slower.
                a.0.resolve().as_str().cmp(b.0.resolve().as_str())
            }
            _ => self.cmp(other),
        }
    }
}

impl Display for SimpleSelector<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SimpleSelector::Type(tag) => f.write_str(&tag.0.resolve()),
            SimpleSelector::Role(role) => write!(f, "[role={role}]"),
            SimpleSelector::Class(class) => write!(f, ".{class}"),
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
struct HtmlTagKey(HtmlTag);

impl Ord for HtmlTagKey {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // We don't really care about what ordering, just that there is *some*
        // consistent ordering of selectors.
        self.0.opaque_key().cmp(&other.0.opaque_key())
    }
}

impl PartialOrd for HtmlTagKey {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

fn find_simple_group_selectors<'a, 'b>(
    bump: &'b Bump,
    temp: &mut Bump,
    elem_tree: &mut ElemTree<'a, 'b>,
    props: &IdMap<Property, PropRefs>,
    selectors: &mut Selectors<'b>,
) {
    // Find the intersection of groups for each selector.
    for elem in elem_tree.iter_mut() {
        temp.reset();

        let mut groups = (elem.props.iter())
            .map(|prop_id| props.get_id(*prop_id).group())
            .collect_in::<BumpVec<_>>(bump);
        groups.sort_unstable();
        groups.dedup();

        // Late initialize the groups here.
        elem.groups = ListSet::from_sorted(groups.into_bump_slice());

        for &selector in elem.simple_selectors.iter() {
            let refs = selectors.simple.get_id_mut(selector);
            refs.targetable.intersect(bump, &elem.groups);
        }
    }
}

#[derive(Default)]
struct ScratchSet<T>(IndexSet<T, FxBuildHasher>);

impl<T> ScratchSet<T> {
    fn new() -> Self {
        Self(IndexSet::default())
    }

    fn reuse(&mut self) -> &mut IndexSet<T, FxBuildHasher> {
        self.0.clear();
        &mut self.0
    }
}

#[allow(clippy::too_many_arguments)]
fn find_complex_group_selectors<'a, 'b>(
    bump: &'b Bump,
    temp: &Bump,
    scratch: &mut ScratchSet<SimpleSelectorId>,
    elem_tree: &ElemTree<'a, 'b>,
    selectors: &mut Selectors<'b>,
    group_id: GroupId,
    elem_id: ElemId,
    selector_list: &mut BumpVec<SelectorId>,
) {
    let child = elem_tree.get(elem_id);

    // Deduplicate parent selectors.
    let ancestor_selectors = scratch.reuse();
    for ancestor in elem_tree.ancestors(elem_id) {
        for &selector in ancestor.simple_selectors.iter() {
            ancestor_selectors.insert(selector);
        }
    }

    // TODO: Mix in some sort averaged of locality score for descendant selectors.
    for &ancestor_selector in ancestor_selectors.iter() {
        let ancestor_selector_refs = selectors.simple.get_id(ancestor_selector);

        // Only collect the descendant ranges if a selector hasn't been cached.
        let descendant_ranges = LazyCell::new(|| {
            let mut ranges = BumpVec::new_in(temp);
            for &elem_id in ancestor_selector_refs.elems.iter() {
                let elem = elem_tree.get(elem_id);
                ranges.push(elem.descendants);
            }

            combine_ranges(&mut ranges);

            ranges
        });

        for &child_selector in child.simple_selectors.iter() {
            let complex = Selector::Descendant(ancestor_selector, child_selector);

            let entry = selectors.complex.entry(complex);
            let id = complex_to_selector_id(&selectors.simple, entry.id());
            let refs = entry.or_insert_with(|| {
                compute_descendant_selector_refs(
                    bump,
                    elem_tree,
                    &selectors.simple,
                    child_selector,
                    &descendant_ranges,
                )
            });

            if refs.targetable.is(group_id) {
                selector_list.push(id);
            }
        }
    }

    if let Some(parent) = elem_tree.ancestors(elem_id).next() {
        for &parent_selector in parent.simple_selectors.iter() {
            let parent_selector_refs = selectors.simple.get_id(parent_selector);

            for &child_selector in child.simple_selectors.iter() {
                let complex = Selector::Child(parent_selector, child_selector);

                let entry = selectors.complex.entry(complex);
                let id = complex_to_selector_id(&selectors.simple, entry.id());
                let refs = entry.or_insert_with(|| {
                    compute_child_selector_refs(
                        bump,
                        elem_tree,
                        child_selector,
                        &parent_selector_refs.elems,
                    )
                });

                if refs.targetable.is(group_id) {
                    selector_list.push(id);
                }
            }
        }
    }
}

fn complex_to_selector_id(
    simple_selectors: &IdMap<SimpleSelector<'_>, SimpleSelectorRefs<'_>>,
    complex_id: ComplexSelectorId,
) -> SelectorId {
    SelectorId::new(simple_selectors.len() + complex_id.idx())
}

fn compute_descendant_selector_refs<'a, 'b>(
    bump: &'b Bump,
    elem_tree: &ElemTree<'a, 'b>,
    simple_selectors: &IdMap<SimpleSelector<'b>, SimpleSelectorRefs<'b>>,
    descendant_selector: SimpleSelectorId,
    descendant_ranges: &[IdRange<ElemId>],
) -> ComplexSelectorRefs<'b> {
    let mut complex_refs = ComplexSelectorRefs::default();

    let child_selector_refs = simple_selectors.get_id(descendant_selector);

    let num_descendants =
        descendant_ranges.iter().map(|range| range.len()).sum::<usize>();

    // Check the list that has less items in it. This is just an optimization,
    // both branches will produce the same result.
    if num_descendants < child_selector_refs.elems.len() {
        for descendant in
            descendant_ranges.iter().flat_map(|ids| elem_tree.get_range(*ids))
        {
            if !descendant.simple_selectors.contains(&descendant_selector) {
                // Descendant doesn't have the selector.
                continue;
            }

            complex_refs.targetable.intersect(bump, &descendant.groups);
        }
    } else {
        for &elem_id in child_selector_refs.elems.iter() {
            if !ranges_contain(descendant_ranges, elem_id) {
                // Element is no descendant.
                continue;
            }

            let descendant = elem_tree.get(elem_id);
            complex_refs.targetable.intersect(bump, &descendant.groups);
        }
    }

    complex_refs
}

fn compute_child_selector_refs<'a, 'b>(
    bump: &'b Bump,
    elem_tree: &ElemTree<'a, 'b>,
    child_selector: SimpleSelectorId,
    parents: &[ElemId],
) -> ComplexSelectorRefs<'b> {
    let mut complex_refs = ComplexSelectorRefs::default();

    for child in parents.iter().flat_map(|id| elem_tree.children(*id)) {
        if !child.simple_selectors.contains(&child_selector) {
            // Descendant doesn't have the selector.
            continue;
        }

        complex_refs.targetable.intersect(bump, &child.groups);
    }

    complex_refs
}

/// Combine overlapping ranges.
fn combine_ranges<T>(ranges: &mut BumpVec<'_, IdRange<Id<T>>>) {
    ranges.sort_by_key(|range| range.start);

    let mut i = 0;
    for j in 1..ranges.len() {
        let [a, b] = ranges.get_disjoint_mut([i, j]).expect("i < j");

        if a.end >= b.start {
            a.end = a.end.max(b.end);
        } else {
            i += 1;
            ranges[i] = *b;
        }
    }
    ranges.truncate(i + 1);
}

/// Uses binary search to check if any range in the sorted non-overlapping slice
/// of ranges contains the [`Id`].
fn ranges_contain<T>(ranges: &[IdRange<Id<T>>], id: Id<T>) -> bool {
    let res = ranges.binary_search_by(|range| {
        if range.start > id {
            std::cmp::Ordering::Greater
        } else if range.end <= id {
            std::cmp::Ordering::Less
        } else {
            std::cmp::Ordering::Equal
        }
    });
    res.is_ok()
}

fn identify_groups<'a, 'b>(
    bump: &'b Bump,
    temp: &mut Bump,
    elem_tree: &mut ElemTree<'a, 'b>,
    props: &IdMap<Property, PropRefs>,
    selectors: &mut Selectors<'b>,
    groups: &IdVec<PropGroup<'b>>,
) -> Stylesheet {
    let mut styles = IdMap::new();

    let mut class_number: u32 = 1;

    let mut scratch = ScratchSet::new();
    let mut unidentified = Vec::new();
    for (group_id, group) in groups.id_iter() {
        unidentified.clear();
        temp.reset();

        // The group with an empty set of properties is only included to check
        // for uniqueness of selectors.
        if props.is_empty() {
            continue;
        }

        let mut identifier = identify_group(
            bump,
            temp,
            &mut scratch,
            elem_tree,
            props,
            selectors,
            group_id.downcast(),
            group,
            &mut unidentified,
        );

        // Only add a custom class to `unidentified` elements, and reuse
        // existing selectors for other ones. Otherwise there might be
        // unexpected cases where creating an unrelated element with the same
        // properties will force custom class assignment for an otherwise well
        // identified subset of the group.
        if !unidentified.is_empty() {
            // TODO: Derive better names if possible.
            let mut name = BumpString::new_in(temp);
            // Naively generate a custom class name.
            while {
                name.clear();
                write!(name, "typst-{class_number}").ok();
                class_number += 1;
                selectors.simple.contains_key(&SimpleSelector::Class(name.as_str()))
            } {}

            // Add the generated class.
            for &elem_id in unidentified.iter() {
                let elem = elem_tree.get_mut(elem_id);
                if let Some(classes) = elem.attrs.get_mut(attr::class) {
                    classes.push(' ');
                    classes.push_str(&name);
                } else {
                    elem.attrs.push(attr::class, EcoString::from(name.as_str()));
                }
            }

            identifier.push(SimpleSelector::Class(name.into_bump_str()).into());
        }

        identifier.sort_by(|a, b| selectors.alphanumeric_cmp(a, b));

        let selector_list = eco_format!("{}", selectors.display_list(&identifier));
        let group_props = (group.props.iter())
            .map(|&prop| (*props.get_id_key(prop)).clone())
            .collect();
        styles.insert(selector_list, group_props);
    }

    // Sort the stylesheet
    let mut styles = styles.into_inner();
    styles.sort_by(|a, _, b, _| a.cmp(b));
    Stylesheet { styles }
}

/// Index into `selectcor_candidates`.
type SelectorCandidateId = Id<SelectorCandidateKey>;
struct SelectorCandidateKey;
impl KeyFor<SelectorCandidate<'_>> for SelectorCandidateKey {}

struct SelectorCandidate<'t> {
    buckets: BumpVec<'t, BucketId>,
    /// Cached number of elements this selector would cover.
    /// Is updated incrementally.
    ///
    /// NOTE: This is late initialized after all elements have been collected
    /// into disjoint buckets.
    remaining_elems: Cell<u32>,
    selector_score: u32,
}

impl<'t> SelectorCandidate<'t> {
    fn new(bump: &'t Bump, selector_score: u32) -> Self {
        Self {
            buckets: BumpVec::new_in(bump),
            remaining_elems: Cell::new(0),
            selector_score,
        }
    }

    /// The combined priority of this selector candidate.
    fn priority(&self) -> [u32; 2] {
        [self.remaining_elems.get(), self.selector_score]
    }
}

/// Index into the `buckets` array.
type BucketId = Id<Bucket>;

#[derive(Debug, Default)]
struct Bucket {
    eliminated: Cell<bool>,
    num_elems: u32,
}

/// Try to identify a group of elements that have the same CSS properties, by
/// generating a selector list. The selectors make use of the element's type,
/// role, and classes that are already present.
/// Elements that can't be targeted by any selector will be added to the
/// `unidentified` list.
///
/// # Algorithm
///
/// ## Conceptual
/// In principle this approaches the ["Set cover problem"] using greedy
/// algorithm to approximate a minimal set cover (selector list):
/// ```txt
/// // U is universe: the set of all uniquely identifiable elements in the group
/// // S is the family of subsets: unique selectors covering a set subset of U
/// greedy-set-cover(U, S) {
///     X = U  // stores the uncovered elements
///     C = {} // stores the sets of the cover
///     while (X is not empty) {
///         select s[i] in S that covers the most elements of X
///         add i to C
///         remove the elements of s[i] from X
///     }
///     return C
/// }
/// ```
///
/// ## In practice
///
/// ### Partitioning
/// First the element list of this group is partitioned into mutually exclusive
/// (disjoint) buckets of elements that have the same non-empty set of unique
/// selectors.
///
/// Elements that can't be uniquely described by any of their selectors are
/// stored in the `unidentified` list and will have a custom class assigned.
///
/// ### Filter
/// In a pre-pass, all selectors that have a bucket of elements which can only
/// be identified them are selected and the buckets are eliminated.
///
/// ### Weighted cover
/// After partitioning the elements into buckets, this is essentially the
/// weighted ["Set cover problem"], where the number of elements in each bucket
/// is the bucket's weight. The algorithm incrementally selects the selector
/// which has the highest number of [`SelectorCandidate::remaining_elems`] adds
/// it to the selector list and updates all other selector candidates that would
/// also cover the same bucket. Selectors that have no more remaining elements
/// are eliminated. This is done until there are no more candidates, and thus
/// all buckets of elements have been covered.
///
/// ["Set cover problem"]: https://en.wikipedia.org/wiki/Set_cover_problem
#[allow(clippy::too_many_arguments)]
fn identify_group<'a, 'b>(
    bump: &'b Bump,
    temp: &mut Bump,
    scratch: &mut ScratchSet<SimpleSelectorId>,
    elem_tree: &ElemTree<'a, 'b>,
    props: &IdMap<Property, PropRefs>,
    selectors: &mut Selectors<'b>,
    group_id: GroupId,
    group: &PropGroup,
    unidentified: &mut Vec<ElemId>,
) -> BumpVec<'b, Selector<'b>> {
    // PERF: Consider adding some cutoff optimizations here.

    let group_elems = &props.get_id(group.first()).elems;

    // Buckets of elements are identified by an exact intersection of selectors.
    //   => All buckets are mutually exclusive (disjoint) sets of elements.
    let mut buckets = IdMap::<&[SelectorId], Bucket>::new();

    // Maps from each selector to a list of buckets with elements.
    let mut selector_candidates = IdMap::<SelectorId, SelectorCandidate>::new();

    // Find class or type selectors that identify buckets of elements within the
    // current group, but no elements from other groups.
    for &elem_id in group_elems.iter() {
        let elem = elem_tree.get(elem_id);

        // Collect all targetable simple selectors.
        let mut selector_list = BumpVec::new_in(temp);
        for &selector in elem.simple_selectors.iter() {
            if selectors.simple.get_id(selector).targetable.is(group_id) {
                selector_list.push(selector.upcast());
            }
        }

        // PERF: Try to avoid computing complex selectors.
        // Don't do this if there is a simple selector that covers all elements.
        find_complex_group_selectors(
            bump,
            temp,
            scratch,
            elem_tree,
            selectors,
            group_id,
            elem_id,
            &mut selector_list,
        );

        if selector_list.is_empty() {
            unidentified.push(elem_id);
        } else {
            let selector_list = selector_list.into_bump_slice();
            let entry = buckets.entry(selector_list);
            let bucket_first_created = entry.is_vacant();
            let bucket_id = entry.id();
            entry.or_default().num_elems += 1;

            if bucket_first_created {
                for &selector in selector_list.iter() {
                    let candidate =
                        selector_candidates.entry(selector).or_insert_with(|| {
                            let score = selectors.score(selector);
                            SelectorCandidate::new(temp, score)
                        });
                    // The bucked ID is only added to the candidate when the
                    // bucket is first created, so there won't be duplicate IDs.
                    candidate.buckets.push(bucket_id);
                }
            }
        }
    }

    // Compute the number of remaining elements per selector candidate.
    for candidate in selector_candidates.values() {
        let num_elems = (candidate.buckets.iter())
            .map(|id| buckets.get_id(*id).num_elems)
            .sum::<u32>();
        candidate.remaining_elems.set(num_elems);
    }

    let mut identifier = BumpVec::new_in(bump);

    // Eliminiate all buckets that only have a single possible selector.
    for bucket_id in buckets.ids() {
        if let [selector] = buckets.get_id_key(bucket_id) {
            let candidate_id = selector_candidates.lookup_id(selector).unwrap();
            choose_selector_candidate(
                selectors,
                &mut identifier,
                &selector_candidates,
                &buckets,
                candidate_id,
                |_, _| (),
            );
        }
    }

    // Build a priority queue backed by a binary heap to incrementally select
    // the next selector and update the other selector's number of remaining
    // elements if they point to the same buckets.
    let selector_queue = selector_candidates
        .id_iter()
        .filter(|(_, _, candidate)| candidate.remaining_elems.get() > 0)
        .map(|(id, _, _)| id.downcast())
        .collect();
    let mut selector_queue =
        IdQueue::new(selector_queue, |id| selector_candidates.get_id(id).priority());

    // Incrementally build the identifier for this group by choosing the
    // selector that will cover the most remaining elements.
    while let Some(candidate_id) = selector_queue.pop() {
        choose_selector_candidate(
            selectors,
            &mut identifier,
            &selector_candidates,
            &buckets,
            candidate_id,
            |candidate_id, num_elems| {
                if num_elems == 0 {
                    selector_queue.remove(candidate_id);
                } else {
                    selector_queue.update(candidate_id);
                }
            },
        );
    }

    identifier
}

/// Choose a selector and recursively update the
/// [`SelectorCandidate::remaining_elems`] count of all affected selectors.
fn choose_selector_candidate<'b, F>(
    selectors: &Selectors<'b>,
    selector_list: &mut BumpVec<Selector<'b>>,
    selector_candidates: &IdMap<SelectorId, SelectorCandidate>,
    buckets: &IdMap<&[SelectorId], Bucket>,
    candidate_id: SelectorCandidateId,
    mut update_queue: F,
) where
    F: FnMut(SelectorCandidateId, u32),
{
    let (&selector, candidate) = selector_candidates.get_id_full(candidate_id);

    // There should be no candidates with 0 `remaining_elems` in the queue.
    // Either they should have been filtered out when creating the queue, or
    // they should have been removed when their `remaining_elems` count was
    // updated.
    debug_assert_ne!(candidate.remaining_elems.get(), 0);

    selector_list.push(selectors.get(selector));

    candidate.remaining_elems.set(0);

    // Eliminate the covered buckets.
    for &bucket_id in candidate.buckets.iter() {
        let (list, bucket) = buckets.get_id_full(bucket_id);
        if bucket.eliminated.get() {
            continue;
        }
        bucket.eliminated.set(true);

        // Update the candidate element counts.
        for selector in list.iter() {
            let candidate_id = selector_candidates.lookup_id(selector).unwrap();
            let candidate = selector_candidates.get_id(candidate_id);
            let remaining_elems = candidate.remaining_elems.get();

            // This should only happen for the candidate that was just selected,
            // otherwise the bucket shouldn't have been eliminated.
            if remaining_elems == 0 {
                continue;
            }

            candidate.remaining_elems.set(remaining_elems - bucket.num_elems);

            update_queue(candidate_id, candidate.remaining_elems.get());
        }
    }
}
