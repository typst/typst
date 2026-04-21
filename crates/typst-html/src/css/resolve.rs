//! This modules tries to resolve a [`Stylesheet`] from the CSS properties that
//! are specified for each element. Existing classes and tags are used where
//! possible, otherwise custom classes are generated and assigned to elements.

use std::cell::Cell;
use std::collections::VecDeque;
use std::fmt::{Debug, Display, Write as _};
use std::hash::Hash;

use bumpalo::Bump;
use bumpalo::collections::{CollectIn, String as BumpString, Vec as BumpVec};
use ecow::EcoString;
use ecow::string::ToEcoString;
use indexmap::IndexMap;
use rustc_hash::{FxBuildHasher, FxHashSet};
use typst_utils::{Id, IdMap, IdRange, IdVec, KeyFor, ListSet};

use crate::css::resolve::idqueue::IdQueue;
use crate::css::{Properties, Property};
use crate::{HtmlAttrs, HtmlElement, HtmlNode, HtmlTag, attr};

mod idqueue;

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

    let (mut elems, props, selectors) = collect_elems(&bump, root);
    let groups = collect_prop_groups(&bump, &mut temp, &elems, &props);
    let targetable = find_group_selectors(&bump, &mut temp, &elems, &props, &selectors);
    identify_groups(
        &bump,
        &mut temp,
        &mut elems,
        &props,
        &selectors,
        &groups,
        &targetable,
    )
}

type ElemId = Id<ElemKey>;
struct ElemKey;
impl KeyFor<Elem<'_, '_>> for ElemKey {}

/// The whole [`HtmlElement`] cannot be mutably borrowed, because that would
/// also include its children.
#[derive(Debug)]
struct Elem<'a, 'b> {
    parent: Option<ElemId>,
    children: Option<IdRange<ElemId>>,
    attrs: &'a mut HtmlAttrs,
    props: BumpVec<'b, PropId>,
    selectors: BumpVec<'b, SelectorId>,
}

impl<'a, 'b> Elem<'a, 'b> {
    fn new(
        parent: Option<ElemId>,
        props: BumpVec<'b, PropId>,
        selectors: BumpVec<'b, SelectorId>,
        attrs: &'a mut HtmlAttrs,
    ) -> Self {
        Self { parent, children: None, attrs, props, selectors }
    }

    fn push_child(&mut self, id: ElemId) {
        match &mut self.children {
            Some(range) => range.include(id),
            None => self.children = Some(IdRange::new(id)),
        }
    }
}

/// Collect all elements in the document into a bi-directional tree.
fn collect_elems<'a, 'b>(
    bump: &'b Bump,
    root: &'a mut HtmlElement,
) -> (IdVec<Elem<'a, 'b>>, IdMap<&'a Property, PropRefs>, IdMap<Selector<'b>, SelectorKey>)
{
    // TODO: Is a tree struct wrapper around the `IdVec` convenient?
    let mut elems = IdVec::new();
    let mut props = IdMap::new();
    let mut selectors = IdMap::new();

    let root_id = elems.next_id();
    elems.push(Elem::new(
        None,
        intern_props(bump, &mut props, root_id, &root.css),
        intern_scalar_selectors(bump, &mut selectors, root),
        &mut root.attrs,
    ));

    // Traverse the tree breadth first, so children are contiguous in memory and
    // can be indexed by a simple range.
    let mut work = VecDeque::from([(root_id, root.children.make_mut())]);
    while let Some((parent, children)) = work.pop_front() {
        for node in children {
            let HtmlNode::Element(element) = node else { continue };

            let id = elems.next_id();
            elems.push(Elem::new(
                Some(parent),
                intern_props(bump, &mut props, id, &element.css),
                intern_scalar_selectors(bump, &mut selectors, element),
                &mut element.attrs,
            ));

            // Add to parent.
            elems.get_mut(parent).push_child(id);

            // Queue children.
            work.push_back((id, element.children.make_mut()));
        }
    }

    (elems, props, selectors)
}

fn intern_props<'a, 'b>(
    bump: &'b Bump,
    interner: &mut IdMap<&'a Property, PropRefs>,
    elem_id: ElemId,
    props: &'a Properties,
) -> BumpVec<'b, PropId> {
    props
        .iter()
        .map(|prop| {
            let entry = interner.entry(prop);
            let prop_id = entry.id();
            entry.or_default().elems.push(elem_id);
            prop_id
        })
        .collect_in(bump)
}

fn intern_scalar_selectors<'a>(
    bump: &'a Bump,
    interner: &mut IdMap<Selector<'a>, SelectorKey>,
    element: &HtmlElement,
) -> BumpVec<'a, SelectorId> {
    let mut elem_selectors = BumpVec::new_in(bump);

    {
        let entry = interner.entry(Selector::ty(element.tag));
        elem_selectors.push(entry.id());
        entry.or_insert(SelectorKey);
    }

    if let Some(role) = element.attrs.get(attr::role) {
        let id = if let Some(id) = interner.lookup_id(&Selector::role(role)) {
            id
        } else {
            let id = interner.next_id();
            interner.insert(Selector::role(bump.alloc_str(role)), SelectorKey);
            id
        };
        elem_selectors.push(id);
    }

    if let Some(classes) = element.attrs.get(attr::class) {
        for class in classes.split_whitespace() {
            // Don't use the entry API so each selector class is only bump
            // allocated once.
            let id = if let Some(id) = interner.lookup_id(&Selector::class(class)) {
                id
            } else {
                let id = interner.next_id();
                interner.insert(Selector::class(bump.alloc_str(class)), SelectorKey);
                id
            };
            elem_selectors.push(id);
        }
    }

    elem_selectors.sort_unstable();
    elem_selectors.dedup();

    elem_selectors
}

struct Groups<'a, 'b, 'c> {
    props: &'c IdMap<&'a Property, PropRefs>,
    groups: IdVec<PropGroup<'b>>,
}

impl<'b> Groups<'_, 'b, '_> {
    /// Create a new group and update the [`Self::group_by_prop`] lookup table.
    fn create_group(&mut self, props: BumpVec<'b, PropId>) {
        let group_id = self.groups.next_id();
        for &prop in props.iter() {
            self.props.get_id(prop).set_group(group_id);
        }
        self.groups.push(PropGroup::new(props));
    }
}

type PropId = Id<PropRefs>;

#[derive(Default)]
struct PropRefs {
    /// The elements are inserted in iteration order and thus the [`ElemId`] is
    /// monotonically increasing.
    elems: Vec<ElemId>,

    /// This is late initialized and updated during property grouping, but is
    /// guaranteed to be set after [`collect_prop_groups`].
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
    /// have the exact same set of elements, and the list is neve empty.
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

fn collect_prop_groups<'a, 'b>(
    bump: &'b Bump,
    temp: &mut Bump,
    elems: &IdVec<Elem<'a, 'b>>,
    props: &IdMap<&'a Property, PropRefs>,
) -> IdVec<PropGroup<'b>> {
    let mut ctx = Groups { props, groups: IdVec::new() };

    for elem in elems.iter() {
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
            .collect_in::<BumpVec<_>>(&bump);

        let existing_props = ListSet::new(existing_props);

        existing_groups.sort();
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

            // Remove properties that aren't shared between the group
            // and the element.
            let split_off_props = group
                .props
                .drain_filter(|group_prop| !existing_props.contains(group_prop))
                .collect_in::<BumpVec<_>>(bump);

            // Create a new group with the split off properties.
            if !split_off_props.is_empty() {
                ctx.create_group(split_off_props);
            }
        }
    }

    ctx.groups
}

/// whether groups are taretable by selectors.
#[derive(Default, Clone)]
struct Targetable<'b> {
    /// Sparse sorted list of groups that can be targeted by the selectors.
    /// This is the intersection of all elements matching the selector.
    groups: Option<BumpVec<'b, GroupId>>,
}

impl<'b> Targetable<'b> {
    /// Whether a group is targetable by the selector.
    fn is_targetable(&self, group: GroupId) -> bool {
        let Some(groups) = &self.groups else { return false };
        groups.binary_search(&group).is_ok()
    }

    /// Computes the intersection between the targetable groups and the element
    /// groups. If there is no targetable list of groups yet, the element's
    /// groups are bump allocated and used as the initial targetable set.
    fn intersect(&mut self, bump: &'b Bump, elem_groups: &[GroupId]) {
        let Some(targetable) = &mut self.groups else {
            let mut groups = BumpVec::new_in(bump);
            groups.extend_from_slice_copy(elem_groups);
            self.groups = Some(groups);
            return;
        };

        // Iterate the two sorted lists in tandem.
        let mut elem_groups = elem_groups.iter().copied().peekable();
        targetable.retain(|&target| {
            // Skip all group IDs smaller than the target one.
            while elem_groups.next_if(|&g| g < target).is_some() {}

            // Retain the target if it is also inside the element groups.
            elem_groups.next_if(|&g| g == target).is_some()
        });
    }
}

/// A list of CSS selectors.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
struct SelectorList<'a>(&'a [Selector<'a>]);

impl Display for SelectorList<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (i, selector) in self.0.iter().enumerate() {
            if i > 0 {
                f.write_str(", ")?;
            }
            Display::fmt(selector, f)?;
        }
        Ok(())
    }
}

type SelectorId = Id<SelectorKey>;
struct SelectorKey;
impl KeyFor<Selector<'_>> for SelectorKey {}
impl KeyFor<Targetable<'_>> for SelectorKey {}

/// A CSS selector.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
enum Selector<'a> {
    /// E.g. `.parent .child`
    Descendant(ScalarSelector<'a>, ScalarSelector<'a>),
    Scalar(ScalarSelector<'a>),
}

impl<'a> Selector<'a> {
    fn ty(tag: HtmlTag) -> Self {
        ScalarSelector::Type(HtmlTagKey(tag)).into()
    }

    fn role(role: &'a str) -> Self {
        ScalarSelector::Role(role).into()
    }

    fn class(class: &'a str) -> Self {
        ScalarSelector::Class(class).into()
    }

    fn first_selector(&self) -> ScalarSelector<'a> {
        match self {
            Selector::Descendant(first, _) => *first,
            Selector::Scalar(scalar) => *scalar,
        }
    }
}

impl<'a> From<ScalarSelector<'a>> for Selector<'a> {
    fn from(v: ScalarSelector<'a>) -> Self {
        Self::Scalar(v)
    }
}

impl Display for Selector<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Selector::Descendant(parent, child) => write!(f, "{parent} {child}"),
            Selector::Scalar(selector) => write!(f, "{selector}"),
        }
    }
}

/// A CSS selector.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
enum ScalarSelector<'a> {
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

impl Display for ScalarSelector<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScalarSelector::Type(tag) => f.write_str(&tag.0.resolve()),
            ScalarSelector::Role(role) => write!(f, "[role={role}]"),
            ScalarSelector::Class(class) => write!(f, ".{class}"),
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

/// A [`PropGroup`] is targetable by a [`Selector`] $s$, if all [`Elem`]s that
/// are selected by $s$ are part of the group. By extension, if there is a
/// single element that isn't part of the property group, the selector can't be
/// used to target the group. And all selectors that select an element without
/// any properties, can be ruled out directly.
fn find_group_selectors<'a, 'b>(
    bump: &'b Bump,
    temp: &mut Bump,
    elems: &IdVec<Elem<'a, 'b>>,
    props: &IdMap<&Property, PropRefs>,
    selectors: &IdMap<Selector<'b>, SelectorKey>,
) -> IdVec<Targetable<'b>> {
    let mut targetable_by_selector =
        IdVec::from_iter(std::iter::repeat_n(Targetable::default(), selectors.len()));

    // Find the intersection of groups for each selector.
    for elem in elems.iter() {
        temp.reset();

        let mut groups = (elem.props.iter())
            .map(|prop_id| props.get_id(*prop_id).group())
            .collect_in::<BumpVec<_>>(temp);
        groups.sort();
        groups.dedup();

        for &selector in elem.selectors.iter() {
            let targetable = targetable_by_selector.get_mut(selector);
            targetable.intersect(bump, &groups);
        }
    }

    targetable_by_selector
}

fn identify_groups<'a, 'b>(
    bump: &'b Bump,
    temp: &mut Bump,
    elem_tree: &mut IdVec<Elem<'a, 'b>>,
    props: &IdMap<&Property, PropRefs>,
    selectors: &IdMap<Selector<'b>, SelectorKey>,
    groups: &IdVec<PropGroup<'b>>,
    targetable: &IdVec<Targetable>,
) -> Stylesheet {
    let mut styles = IdMap::new();

    let mut class_number: u32 = 1;

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
            elem_tree,
            props,
            selectors,
            targetable,
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
                selectors.contains_key(&Selector::class(name.as_str()))
            } {}

            // Add the generated class.
            for &elem_id in unidentified.iter() {
                let elem = elem_tree.get_mut(elem_id);
                if let Some(classes) = elem.attrs.get_mut(attr::class) {
                    classes.push(' ');
                    classes.push_str(&name);
                } else {
                    elem.attrs.push_front(attr::class, EcoString::from(name.as_str()));
                }
            }

            identifier.push(Selector::class(name.into_bump_str()));
        }

        identifier.sort_by(|a, b| match (a.first_selector(), b.first_selector()) {
            (ScalarSelector::Type(a), ScalarSelector::Type(b)) => {
                // The `Ord` implementation of `HtmlTagKey` doesn't sort by
                // string contents for performance reasons. For the CSS
                // generation it's preferable to sort alphabetically even though
                // it will be slower.
                a.0.resolve().as_str().cmp(b.0.resolve().as_str())
            }
            _ => a.cmp(b),
        });

        let selector_list = SelectorList(&identifier).to_eco_string();
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
type SelectorCandidateId = Id<SelectorCandidate>;

#[derive(Default)]
struct SelectorCandidate {
    // PERF: Consider using a container with inline storage and the
    // bump-allocator as backing storage.
    buckets: FxHashSet<BucketId>,
    /// Cached number of elements this selector would cover.
    /// Is updated incrementally.
    remaining_elems: Cell<u32>,
}

/// Index into the `buckets` array.
type BucketId = Id<Bucket>;

#[derive(Debug, Default)]
struct Bucket {
    eliminated: Cell<bool>,
    num_elems: u32,
}

/// Try to uniquely identify a group of elements that have the same CSS
/// properties, using a list of unique type and class selectors that are already
/// present on the elements. Unique selectors are those that only describe the
/// elements within the current group and no elements of other groups.
///
/// # Algorithm
///
/// ## Conceptual
/// In principle this solves the ["Set cover problem"] using greedy algorithm to
/// approximate a minimal set cover (selector list):
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
/// First the group of elements is partitioned into mutually exclusive
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
fn identify_group<'a, 'b>(
    bump: &'b Bump,
    temp: &Bump,
    elem_tree: &IdVec<Elem<'a, 'b>>,
    props: &IdMap<&Property, PropRefs>,
    selectors: &IdMap<Selector<'b>, SelectorKey>,
    targetable: &IdVec<Targetable>,
    group_id: GroupId,
    group: &PropGroup,
    unidentified: &mut Vec<ElemId>,
) -> BumpVec<'b, Selector<'b>> {
    // PERF: Consider adding some cutoff optimizations here.

    let elems = &props.get_id(group.first()).elems;

    // Buckets of elements are identified by an exact intersection of selectors.
    //   => All buckets are mutually exclusive (disjoint) sets of elements.
    let mut buckets = IdMap::<&[SelectorId], Bucket>::new();

    // Maps from each selector to a list of buckets with elements.
    let mut selector_candidates = IdMap::<SelectorId, SelectorCandidate>::new();

    // Find class or type selectors that identify buckets of elements within the
    // current group, but no elements from other groups.
    for &elem_id in elems.iter() {
        let elem = elem_tree.get(elem_id);

        // TODO: consider removing type and possibly role selectors, when more precise
        // selectors are available, or scoring scalar type and role selectors
        // lower than class and combinators.
        let mut selector_list = BumpVec::new_in(temp);

        for &selector in elem.selectors.iter() {
            if targetable.get(selector).is_targetable(group_id) {
                selector_list.push(selector);
            }
        }

        // TODO: generate selectors with descendant or parent combinators.

        if selector_list.is_empty() {
            unidentified.push(elem_id);
        } else {
            // PERF: Avoid unnecessary bump allocations, free the selector list, if
            // it's already present in the selector map. (Not that bad, since
            // it's in the temporary bump allocator)
            let selector_list = selector_list.into_bump_slice();
            let entry = buckets.entry(selector_list);
            let first_inserted = entry.is_vacant();
            let bucket_id = entry.id();
            entry.or_default().num_elems += 1;

            if first_inserted {
                for &selector in selector_list.iter() {
                    let candidate = selector_candidates.entry(selector).or_default();
                    candidate.buckets.insert(bucket_id);
                }
            }
        }
    }

    // PERF: We could find disjoint sets of selectors by doing a flood fill and
    // compute the identifiers for those.
    // Disjoint meaning that there is no path between two selectors when
    // recursively visiting all selectors that point to the same bucket.
    // Would that change results? Probably no, but not sure.
    // Would it be more efficient?

    let mut identifier = BumpVec::new_in(bump);

    // Compute the number of remaining elements per selector candidate.
    for candidate in selector_candidates.values() {
        let num_elems = (candidate.buckets.iter())
            .map(|id| buckets.get_id(*id).num_elems)
            .sum::<u32>();
        candidate.remaining_elems.set(num_elems);
    }

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
    // the next selector and update the priority of other selectors that pointed
    // to the same buckets.
    let selector_queue = selector_candidates
        .id_iter()
        .filter(|(_, _, candidate)| candidate.remaining_elems.get() > 0)
        .map(|(id, _, _)| id)
        .collect_in::<BumpVec<_>>(bump);
    let mut selector_queue = IdQueue::new(bump, selector_queue, |id| {
        selector_candidates.get_id(id).remaining_elems.get()
    });

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
fn choose_selector_candidate<'a, 'b, F>(
    selectors: &IdMap<Selector<'b>, SelectorKey>,
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

    selector_list.push(*selectors.get_id_key(selector));

    candidate.remaining_elems.set(0);

    // Eliminate the covered buckets.
    #[allow(clippy::iter_over_hash_type)]
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
