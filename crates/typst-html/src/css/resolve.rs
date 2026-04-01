//! This modules tries to resolve a [`Stylesheet`] from the CSS properties that
//! are specified for each element. Existing classes and tags are used where
//! possible, otherwise custom classes are generated and assigned to elements.

use std::borrow::Borrow;
use std::cell::Cell;
use std::fmt::{Display, Write as _};
use std::hash::Hash;
use std::ops::Deref;

use bumpalo::Bump;
use bumpalo::collections::{CollectIn, String as BumpString, Vec as BumpVec};
use ecow::EcoString;
use ecow::string::ToEcoString;
use indexmap::IndexMap;
use rustc_hash::{FxBuildHasher, FxHashMap, FxHashSet};
use typst_utils::{Id, IdMap, IdVec, KeyFor};

use crate::css::resolve::idqueue::IdQueue;
use crate::css::{Properties, Property};
use crate::{HtmlAttrs, HtmlNode, HtmlTag, attr};

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

/// TODO: Should the hash for [`Properties`] be cached, similar to [`LazyHash`]?
struct Resolver<'a> {
    bump: &'a Bump,
    /// Elements grouped by their CSS properties.
    groups: IdMap<&'a Properties, Group<'a>>,
    /// Lookup table for groups that contain at least one element with a tag.
    by_tag: FxHashMap<HtmlTag, FxHashSet<GroupId>>,
    /// Lookup table for groups that contain at least one element with a class.
    ///
    /// Simultaneously acts as a string interner for bump allocated class names.
    by_class: FxHashMap<&'a str, FxHashSet<GroupId>>,
}

impl<'a> Resolver<'a> {
    fn new(bump: &'a Bump) -> Self {
        Self {
            bump,
            groups: IdMap::new(),
            by_tag: FxHashMap::default(),
            by_class: FxHashMap::default(),
        }
    }
}

/// Index into [`Resolver::groups`].
type GroupId = Id<GroupKey>;
struct GroupKey;
impl<'a> KeyFor<Group<'a>> for GroupKey {}

#[derive(Debug, Default)]
struct Group<'a> {
    /// The elements in this group.
    elems: IdVec<Elem<'a>>,
}

/// Index into [`Group::elems`].
type ElemId = Id<ElemKey>;
struct ElemKey;
impl<'a> KeyFor<Elem<'a>> for ElemKey {}

/// The whole [`HtmlElement`] cannot be borrowed, because that would also
/// include its children.
#[derive(Debug)]
struct Elem<'a> {
    tag: HtmlTag,
    attrs: &'a mut HtmlAttrs,
}

impl<'a> Elem<'a> {
    fn new(tag: HtmlTag, attrs: &'a mut HtmlAttrs) -> Self {
        Self { tag, attrs }
    }
}

/// Resolve a stylesheet from the CSS styles specified for each element.
pub fn resolve_stylesheet(nodes: &mut [HtmlNode]) -> Stylesheet {
    let bump = Bump::new();
    let mut rs = Resolver::new(&bump);

    for node in nodes.iter_mut() {
        visit_node(&mut rs, node);
    }

    // TODO: Consider preprocessing groups. Maybe have categories of attributes
    // that belong together and split the properties based on that.

    identify_groups(&mut rs)
}

/// Build lookup tables to efficiently identify groups of elements sharing the
/// same properties.
fn visit_node<'a>(rs: &mut Resolver<'a>, node: &'a mut HtmlNode) {
    match node {
        HtmlNode::Element(element) => {
            let entry = rs.groups.entry(&element.css);
            let id = entry.id();
            let group = entry.or_default();

            // Tags
            rs.by_tag.entry(element.tag).or_default().insert(id);

            // Classes
            if let Some(class) = element.attrs.get(attr::class) {
                for class in class.split_whitespace() {
                    if let Some(class_groups) = rs.by_class.get_mut(class) {
                        class_groups.insert(id);
                    } else {
                        // Lazily bump allocate the class strings.
                        let class = rs.bump.alloc_str(class);
                        rs.by_class.entry(class).or_default().insert(id);
                    }
                }
            }

            group.elems.push(Elem::new(element.tag, &mut element.attrs));

            for child in element.children.make_mut() {
                visit_node(rs, child);
            }
        }
        HtmlNode::Tag(..) | HtmlNode::Text(..) | HtmlNode::Frame(..) => (),
    }
}

fn identify_groups(rs: &mut Resolver) -> Stylesheet {
    let mut styles = IdMap::new();

    let mut class_number: u32 = 1;

    let mut bump = Bump::new();
    let mut unidentified = Vec::new();
    for (&props, group) in rs.groups.iter_mut() {
        bump.reset();
        unidentified.clear();

        // The group with an empty set of properties is only included to check
        // for uniqueness of selectors.
        if props.is_empty() {
            continue;
        }

        // TODO: Maybe have some sort of niceness metric at which point we
        // generate our own classes instead of using existing tags and classes.
        // Possibly mixing both. The problem with adding a heuristic like this
        // is that the output may switch unexpectedly.
        let mut identifier =
            indentify_group(&bump, &rs.by_tag, &rs.by_class, group, &mut unidentified);

        // Only add a custom class to `unidentified` elements, and reuse
        // existing selectors for other ones. Otherwise there might be
        // unexpected cases where creating an unrelated element with the same
        // properties will force custom class assignment for an otherwise well
        // identified subset of the group.
        if !unidentified.is_empty() {
            // TODO: Derive better names if possible.
            let mut name = BumpString::new_in(&bump);
            // Naively generate a custom class name.
            while {
                name.clear();
                write!(name, "typst-{class_number}").ok();
                class_number += 1;
                rs.by_class.contains_key(name.as_str())
            } {}

            // Add the class attribute.
            for &elem_id in unidentified.iter() {
                let elem = group.elems.get_mut(elem_id);
                if let Some(classes) = elem.attrs.get_mut(attr::class) {
                    classes.push(' ');
                    classes.push_str(&name);
                } else {
                    elem.attrs.push_front(attr::class, EcoString::from(name.as_str()));
                }
            }

            identifier.push(Selector::Class(name.into_bump_str()));
        }

        identifier.sort_by(|a, b| match (a, b) {
            (Selector::Type(a), Selector::Type(b)) => {
                // The `Ord` implementation of `HtmlTagKey` doesn't sort by
                // string contents, for performance reasons. For the CSS
                // generation it's preferable to sort alphabetically even though
                // it will be slower.
                a.0.resolve().as_str().cmp(b.0.resolve().as_str())
            }
            _ => a.cmp(b),
        });

        let selector_list = SelectorList(&identifier).to_eco_string();
        styles.insert(selector_list, props.clone());
    }

    // Sort the stylesheet
    let mut styles = styles.into_inner();
    styles.sort_by(|a, _, b, _| a.cmp(b));
    Stylesheet { styles }
}

/// A list of CSS selectors.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
struct SelectorList<'a>(&'a [Selector<&'a str>]);

impl<'a> Borrow<[Selector<&'a str>]> for SelectorList<'a> {
    fn borrow(&self) -> &'a [Selector<&'a str>] {
        self.0
    }
}

impl<'a> Deref for SelectorList<'a> {
    type Target = [Selector<&'a str>];

    fn deref(&self) -> &'a Self::Target {
        self.0
    }
}

impl Display for SelectorList<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (i, selector) in self.iter().enumerate() {
            if i > 0 {
                f.write_str(", ")?;
            }
            match selector {
                Selector::Type(tag) => f.write_str(&tag.0.resolve())?,
                Selector::Class(class) => write!(f, ".{class}")?,
            }
        }
        Ok(())
    }
}

/// A CSS selector.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
enum Selector<S> {
    Type(HtmlTagKey),
    Class(S),
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
fn indentify_group<'a>(
    bump: &'a Bump,
    by_tag: &FxHashMap<HtmlTag, FxHashSet<GroupId>>,
    by_class: &FxHashMap<&'a str, FxHashSet<GroupId>>,
    group: &Group<'a>,
    unidentified: &mut Vec<ElemId>,
) -> BumpVec<'a, Selector<&'a str>> {
    // PERF: Consider adding some cutoff optimizations here.

    // PERF: Somehow make use of the bump allocator, reuse allocations, or use
    // some other more efficient allocation strategy.

    // Buckets of elements are identified by an exact intersection of selectors.
    //   => All buckets are mutually exclusive (disjoint) sets of elements.
    let mut buckets = IdMap::<SelectorList, Bucket>::new();

    // Maps from each selector to a list of buckets with elements.
    let mut selector_candidates = IdMap::<Selector<&'a str>, SelectorCandidate>::new();

    // Find class or type selectors that identify buckets of elements within the
    // current group, but no elements from other groups.
    for (i, elem) in group.elems.iter().enumerate() {
        let mut selector_list = BumpVec::new_in(bump);

        if let Some(classes) = elem.attrs.get(attr::class) {
            for class in classes.split_whitespace() {
                let (class, groups) = by_class.get_key_value(class).unwrap();
                if groups.len() == 1 {
                    selector_list.push(Selector::Class(*class));
                }
            }
        }

        // TODO: also use aria roles here?

        // Only use tag if there is no class that can uniquely identify this
        // group.
        if selector_list.is_empty() {
            let groups = by_tag.get(&elem.tag).unwrap();
            if groups.len() == 1 {
                selector_list.push(Selector::Type(HtmlTagKey(elem.tag)));
            }
        }

        if selector_list.is_empty() {
            unidentified.push(ElemId::new(i));
        } else {
            // Make sure the selector lists are consistently sorted, so we can
            // use them as unique keys.
            selector_list.sort_unstable();
            selector_list.dedup();

            // PERF: Avoid unnecessary bump allocations, free the selector list, if
            // it's already present in the selector map.
            let selector_list = SelectorList(selector_list.into_bump_slice());
            let entry = buckets.entry(selector_list);
            let first_inserted = entry.is_vacant();
            let bucket_id = entry.id();
            entry.or_default().num_elems += 1;

            if first_inserted {
                for selector in selector_list.iter() {
                    let candidate = selector_candidates.entry(*selector).or_default();
                    candidate.buckets.insert(bucket_id);
                }
            }
        }
    }

    // PERF: We could find disjoint sets of selectors by doing a flood fill and
    // compute the identifiers for those.
    // Disjoint meaning that there is no path between two selectors when
    // recursively visiting all selectors that point to the same bucket.

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
        let (list, _) = buckets.get_id_full(bucket_id);
        if let [selector] = list.0 {
            let candidate_id = selector_candidates.lookup_id(selector).unwrap();
            choose_selector_candidate(
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
fn choose_selector_candidate<'a, F>(
    selector_list: &mut BumpVec<Selector<&'a str>>,
    selector_candidates: &IdMap<Selector<&'a str>, SelectorCandidate>,
    buckets: &IdMap<SelectorList<'a>, Bucket>,
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

    selector_list.push(selector);

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
