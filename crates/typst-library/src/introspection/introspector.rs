use std::collections::BTreeSet;
use std::hash::Hash;
use std::num::NonZeroUsize;
use std::ops::Range;
use std::sync::RwLock;

use comemo::{Track, Tracked};
use ecow::{EcoString, EcoVec};
use rustc_hash::{FxHashMap, FxHashSet};
use smallvec::SmallVec;
use typst_syntax::VirtualPath;

use crate::diag::{StrResult, bail};
use crate::foundations::{Content, Label, Repr, Selector};
use crate::introspection::{DocumentPosition, Location, Tag};
use crate::model::Numbering;

/// Serves inquiries for pieces of information from the compilation output.
///
/// See [`Introspect`](crate::introspection::Introspect) for general information
/// about introspection.
///
/// This trait is implemented by [target-specific](crate::foundations::Target)
/// introspectors. These must implement all methods to provide a unified
/// interface to the Typst standard library, but may return `None` or error in
/// some methods, depending on the specifics of the target. The HTML target, for
/// instance, will return `None` for [`page`](Self::page) requests.
#[comemo::track]
pub trait Introspector: Send + Sync {
    /// Queries for all matching elements.
    fn query(&self, selector: &Selector) -> EcoVec<Content>;

    /// Queries for the first element that matches the selector.
    fn query_first(&self, selector: &Selector) -> Option<Content>;

    /// Queries for the first element that matches the selector.
    fn query_unique(&self, selector: &Selector) -> StrResult<Content>;

    /// Queries for a unique element with the label.
    fn query_label(&self, label: Label) -> StrResult<&Content>;

    /// Queries for all elements with a label.
    fn query_labelled(&self) -> EcoVec<Content>;

    /// An optimized version of `query(selector.before(end, true).len()` used by
    /// counters and state.
    fn query_count_before(&self, selector: &Selector, end: Location) -> usize;

    /// Checks how many times a label exists.
    fn label_count(&self, label: Label) -> usize;

    /// Tries to find a location for an element with the given `key` hash
    /// that is closest after the `base`.
    ///
    /// This is used for introspector-assisted location assignment during
    /// measurement. See the "Dealing with Measurement" section of the
    /// [`Locator`](crate::introspection::Locator) docs for more details.
    fn locator(&self, key: u128, base: Location) -> Option<Location>;

    /// Returns the total number of pages in the document that contains the
    /// given location.
    fn pages(&self, location: Location) -> Option<NonZeroUsize>;

    /// Returns the page number for the given location.
    fn page(&self, location: Location) -> Option<NonZeroUsize>;

    /// Returns the position for the given location.
    fn position(&self, location: Location) -> Option<DocumentPosition>;

    /// Returns the page numbering for the given location, if any.
    fn page_numbering(&self, location: Location) -> Option<&Numbering>;

    /// Returns the page supplement for the given location, if any.
    fn page_supplement(&self, location: Location) -> Option<&Content>;

    /// Retrieves the anchor to link to for this location in HTML export.
    fn anchor(&self, location: Location) -> Option<&EcoString>;

    /// Returns the location of the document which has/contains the given
    /// location.
    fn document(&self, location: Location) -> Option<Location>;

    /// Returns the file path of the document/asset which has or contains the
    /// given location.
    ///
    /// Returns `None` in a single document (not a bundle) or if the location is
    /// not associated with a document or asset (top-level in a bundle).
    fn path(&self, location: Location) -> Option<&VirtualPath>;
}

/// An introspector that returns empty results for all inquiries.
pub struct EmptyIntrospector;

impl EmptyIntrospector {
    pub fn track(&self) -> Tracked<'_, dyn Introspector + '_> {
        (self as &dyn Introspector).track()
    }
}

impl Introspector for EmptyIntrospector {
    fn query(&self, _: &Selector) -> EcoVec<Content> {
        EcoVec::new()
    }

    fn query_first(&self, _: &Selector) -> Option<Content> {
        None
    }

    fn query_unique(&self, _: &Selector) -> StrResult<Content> {
        bail!("selector does not match any element");
    }

    fn query_label(&self, label: Label) -> StrResult<&Content> {
        bail!("label `{}` does not exist in the document", label.repr());
    }

    fn query_labelled(&self) -> EcoVec<Content> {
        EcoVec::new()
    }

    fn query_count_before(&self, _: &Selector, _: Location) -> usize {
        0
    }

    fn label_count(&self, _: Label) -> usize {
        0
    }

    fn locator(&self, _: u128, _: Location) -> Option<Location> {
        None
    }

    fn pages(&self, _: Location) -> Option<NonZeroUsize> {
        None
    }

    fn page(&self, _: Location) -> Option<NonZeroUsize> {
        None
    }

    fn position(&self, _: Location) -> Option<DocumentPosition> {
        None
    }

    fn page_numbering(&self, _: Location) -> Option<&Numbering> {
        None
    }

    fn page_supplement(&self, _: Location) -> Option<&Content> {
        None
    }

    fn anchor(&self, _: Location) -> Option<&EcoString> {
        None
    }

    fn document(&self, _: Location) -> Option<Location> {
        None
    }

    fn path(&self, _: Location) -> Option<&VirtualPath> {
        None
    }
}

/// An underlying target-agnostic introspector used for most queries.
///
/// The parameter `P` represents a position type for the relevant target.
#[derive(Clone)]
pub struct ElementIntrospector<P> {
    /// All introspectable elements.
    elems: Vec<(Content, P)>,
    /// Lists all elements with a specific hash key. This is used for
    /// introspector-assisted location assignment during measurement.
    keys: MultiMap<u128, Location>,
    /// Accelerates lookup of elements by location.
    ///
    /// Holds a range pointing into `elements` that covers the element and all
    /// its conceptual descendants. The first element in the range (i.e.
    /// `elems[range.start]` is the element with the location itself while the
    /// last element is its right-most descendants).
    locations: FxHashMap<Location, Range<usize>>,
    /// Accelerates lookup of elements by label.
    labels: MultiMap<Label, usize>,
    /// Caches queries done on the introspector. This is important because
    /// even if all top-level queries are distinct, they often have shared
    /// subqueries. Example: Individual counter queries with `before` that
    /// all depend on a global counter query.
    queries: QueryCache,
}

impl<P> ElementIntrospector<P> {
    /// Queries for all matching elements.
    pub fn query(&self, selector: &Selector) -> EcoVec<Content> {
        let hash = typst_utils::hash128(selector);
        if let Some(output) = self.queries.get(hash) {
            return output;
        }

        let output = match selector {
            Selector::Elem(..) => self
                .all()
                .filter(|elem| selector.matches(elem, None))
                .cloned()
                .collect(),
            Selector::Location(location) => {
                self.get_by_loc(location).cloned().into_iter().collect()
            }
            Selector::Label(label) => self
                .labels
                .get(label)
                .iter()
                .map(|&idx| self.get_by_idx(idx).clone())
                .collect(),
            Selector::Or(selectors) => selectors
                .iter()
                .flat_map(|sel| self.query(sel))
                .map(|elem| self.elem_index(&elem))
                .collect::<BTreeSet<usize>>()
                .into_iter()
                .map(|idx| self.get_by_idx(idx).clone())
                .collect(),
            Selector::And(selectors) => {
                let mut results: Vec<_> =
                    selectors.iter().map(|sel| self.query(sel)).collect();

                // Extract the smallest result list and then keep only those
                // elements in the smallest list that are also in all other
                // lists.
                results
                    .iter()
                    .enumerate()
                    .min_by_key(|(_, vec)| vec.len())
                    .map(|(i, _)| i)
                    .map(|i| results.swap_remove(i))
                    .iter()
                    .flatten()
                    .filter(|candidate| {
                        results
                            .iter()
                            .all(|other| self.binary_search(other, candidate).is_ok())
                    })
                    .cloned()
                    .collect()
            }
            Selector::Before { selector, end, inclusive } => {
                let mut list = self.query(selector);
                if let Some(end) = self.query_first(end) {
                    // Determine which elements are before `end`.
                    let split = match self.binary_search(&list, &end) {
                        // Element itself is contained.
                        Ok(i) => i + *inclusive as usize,
                        // Element itself is not contained.
                        Err(i) => i,
                    };
                    list = list[..split].into();
                }
                list
            }
            Selector::After { selector, start, inclusive } => {
                let mut list = self.query(selector);
                if let Some(start) = self.query_first(start) {
                    // Determine which elements are after `start`.
                    let split = match self.binary_search(&list, &start) {
                        // Element itself is contained.
                        Ok(i) => i + !*inclusive as usize,
                        // Element itself is not contained.
                        Err(i) => i,
                    };
                    list = list[split..].into();
                }
                list
            }
            Selector::Within { selector, ancestor } => {
                let list = self.query(selector);
                let ancestors = self.query(ancestor);

                let mut out = EcoVec::new();
                let mut visited = 0;

                // Walk the ancestors in order, collecting all elements in
                // `list` that are descendants of them. Elements in the list
                // that are descendants of multiple ancestors are yielded only
                // once by virtue of `visited`.
                for ancestor in &ancestors {
                    let loc = ancestor.location().unwrap();
                    let Range { start, end } = self.loc_range(&loc);

                    let start_in_list = match list
                        .binary_search_by_key(&start, |elem| self.elem_index(elem))
                    {
                        // The element and the ancestor start at the same index.
                        // This means they are one and the same. The within
                        // selector is not inclusive, so we exclude it.
                        Ok(i) => i + 1,
                        // The ancestor's insertion index would be at `i`, so
                        // the element currently at `i` is later than it and
                        // should be included.
                        Err(i) => i,
                    };

                    let end_in_list = match list.binary_search_by_key(&end, |elem| {
                        self.loc_range(&elem.location().unwrap()).end
                    }) {
                        // The element and the ancestor end in the same place.
                        // They might be the same, but it's equally possible for
                        // the element to be a rightmost leaf of the ancestor.
                        // If it's the same, we already exclude it via `start`
                        // above and if it's a rightmost leaf, we want to
                        // include it.
                        Ok(i) => i + 1,
                        // The ancestor's end index would be at `i`, so the
                        // element right before `i` is earlier than it and
                        // should be included.
                        Err(i) => i,
                    };

                    // Clamp at `visited` to ensure we don't yield elements
                    // twice.
                    let start_in_list = start_in_list.max(visited);
                    let end_in_list = end_in_list.max(visited);
                    out.extend(list[start_in_list..end_in_list].iter().cloned());
                    visited = end_in_list;
                }

                out
            }
            // Not supported here.
            Selector::Can(_) | Selector::Regex(_) => EcoVec::new(),
        };

        self.queries.insert(hash, output.clone());
        output
    }

    /// Queries for the first element that matches the selector.
    pub fn query_first(&self, selector: &Selector) -> Option<Content> {
        match selector {
            Selector::Location(location) => self.get_by_loc(location).cloned(),
            Selector::Label(label) => self
                .labels
                .get(label)
                .first()
                .map(|&idx| self.get_by_idx(idx).clone()),
            _ => self.query(selector).first().cloned(),
        }
    }

    /// Queries for the first element that matches the selector.
    pub fn query_unique(&self, selector: &Selector) -> StrResult<Content> {
        match selector {
            Selector::Location(location) => self
                .get_by_loc(location)
                .cloned()
                .ok_or_else(|| "element does not exist in the document".into()),
            Selector::Label(label) => self.query_label(*label).cloned(),
            _ => {
                let elems = self.query(selector);
                if elems.len() > 1 {
                    bail!("selector matches multiple elements",);
                }
                elems
                    .into_iter()
                    .next()
                    .ok_or_else(|| "selector does not match any element".into())
            }
        }
    }

    /// Queries for a unique element with the label.
    pub fn query_label(&self, label: Label) -> StrResult<&Content> {
        match *self.labels.get(&label) {
            [idx] => Ok(self.get_by_idx(idx)),
            [] => bail!("label `{}` does not exist in the document", label.repr()),
            _ => bail!("label `{}` occurs multiple times in the document", label.repr()),
        }
    }

    /// Queries for all elements with a label.
    pub fn query_labelled(&self) -> EcoVec<Content> {
        self.all().filter(|c| c.label().is_some()).cloned().collect()
    }

    /// An optimized version of `query(selector.before(end, true).len()` used by
    /// counters and state.
    pub fn query_count_before(&self, selector: &Selector, end: Location) -> usize {
        // See `query()` for details.
        let list = self.query(selector);
        if let Some(end) = self.get_by_loc(&end) {
            match self.binary_search(&list, end) {
                Ok(i) => i + 1,
                Err(i) => i,
            }
        } else {
            list.len()
        }
    }

    /// Checks how many times a label exists.
    pub fn label_count(&self, label: Label) -> usize {
        self.labels.get(&label).len()
    }

    /// Tries to find a location for an element with the given `key` hash
    /// that is closest after the `base`.
    pub fn locator(&self, key: u128, base: Location) -> Option<Location> {
        let base = self.loc_index(&base);
        self.keys
            .get(&key)
            .iter()
            .copied()
            .min_by_key(|loc| self.loc_index(loc).wrapping_sub(base))
    }

    /// Returns the target-specific position of the element at the given
    /// location.
    pub fn position(&self, location: Location) -> Option<&P> {
        self.locations.get(&location).map(|r| self.get_pos_by_idx(r.start))
    }

    /// Iterates over all locatable elements.
    pub fn all(&self) -> impl Iterator<Item = &Content> + '_ {
        self.elems.iter().map(|(c, _)| c)
    }

    /// Retrieves the element with the given index.
    #[track_caller]
    pub fn get_by_idx(&self, idx: usize) -> &Content {
        &self.elems[idx].0
    }

    /// Retrieves the position of the element with the given index.
    #[track_caller]
    pub fn get_pos_by_idx(&self, idx: usize) -> &P {
        &self.elems[idx].1
    }

    /// Retrieves an element by its location.
    pub fn get_by_loc(&self, location: &Location) -> Option<&Content> {
        self.locations.get(location).map(|r| self.get_by_idx(r.start))
    }

    /// Performs a binary search for `elem` among the `list`.
    pub fn binary_search(
        &self,
        list: &[Content],
        elem: &Content,
    ) -> Result<usize, usize> {
        list.binary_search_by_key(&self.elem_index(elem), |elem| self.elem_index(elem))
    }

    /// Gets the index of this element.
    pub fn elem_index(&self, elem: &Content) -> usize {
        self.loc_index(&elem.location().unwrap())
    }

    /// Gets the index of the element with this location among all.
    pub fn loc_index(&self, location: &Location) -> usize {
        self.locations.get(location).map(|r| r.start).unwrap_or(usize::MAX)
    }

    /// Gets the range of the element with this location among all.
    pub fn loc_range(&self, location: &Location) -> Range<usize> {
        self.locations
            .get(location)
            .cloned()
            .unwrap_or(usize::MAX..usize::MAX)
    }
}

/// Constructs the [`ElementIntrospector`].
pub struct ElementIntrospectorBuilder<P> {
    stack: Vec<Vec<BuilderItem<P>>>,
    sink: Vec<BuilderItem<P>>,
    seen: FxHashSet<Location>,
    insertions: MultiMap<Location, Vec<BuilderItem<P>>>,
    keys: MultiMap<u128, Location>,
    locations: FxHashMap<Location, Range<usize>>,
    labels: MultiMap<Label, usize>,
}

/// An item in the builder's sink.
enum BuilderItem<P> {
    /// Indicates the start of the given element. Also holds its location and
    /// position.
    Start(Content, Location, P),
    /// Indicates the end of the element with the given location.
    End(Location),
}

impl<P> ElementIntrospectorBuilder<P> {
    /// Creates an empty builder.
    pub fn new() -> Self {
        Self::with_capacity(0)
    }

    /// Creates a builder with pre-allocated capacity for the estimated
    /// number of introspectable elements. Reduces memory waste from
    /// HashMap/HashSet/Vec growth doubling.
    pub fn with_capacity(hint: usize) -> Self {
        let mut seen = FxHashSet::default();
        seen.reserve(hint);
        let mut locations = FxHashMap::default();
        locations.reserve(hint);
        Self {
            stack: Vec::new(),
            sink: Vec::with_capacity(hint.saturating_mul(2)),
            seen,
            insertions: MultiMap::default(),
            keys: MultiMap::default(),
            locations,
            labels: MultiMap::default(),
        }
    }

    /// Discovers an introspectible in a tag.
    pub fn discover_tag(&mut self, tag: &Tag, position: P) {
        match tag {
            Tag::Start(elem, loc, flags) => {
                if flags.introspectable {
                    let loc = *loc;
                    if self.seen.insert(loc) {
                        self.sink.push(BuilderItem::Start(elem.clone(), loc, position));
                    }
                }
            }
            Tag::End(loc, key, flags) => {
                if flags.introspectable {
                    self.keys.insert(*key, *loc);
                    self.sink.push(BuilderItem::End(*loc));
                }
            }
        }
    }

    /// Discovers elements from another already built introspector.
    pub fn discover_elements<Q>(
        &mut self,
        elements: &ElementIntrospector<Q>,
        map_position: impl Fn(&Q) -> P,
    ) {
        // Because `elements` is already fully built, we need to basically
        // reverse the already built location ranges back to start/end events.
        // We do this by queueing end events for positions as we visit elements
        // and dequeueing them at the end of the relevant element.
        self.sink.reserve(2 * elements.elems.len());
        let mut queued = MultiMap::default();
        for (i, (elem, q)) in elements.elems.iter().enumerate() {
            let loc = elem.location().unwrap();
            if self.seen.insert(loc) {
                let range = elements.locations.get(&loc).unwrap();
                let position = map_position(q);
                self.sink.push(BuilderItem::Start(elem.clone(), loc, position));
                debug_assert_eq!(range.start, i);
                queued.insert(range.end, loc);
            }
            for &end in queued.get(&(i + 1)).iter().rev() {
                self.sink.push(BuilderItem::End(end));
            }
        }
        self.keys.extend(&elements.keys);
    }

    /// Future content until a matching `end_insertion` will ordering-wise be
    /// treated as belonging to the `parent` passed to `end_insertion`.
    pub fn start_insertion(&mut self) {
        self.stack.push(std::mem::take(&mut self.sink));
    }

    /// Closes an insertion group started by a matching `start_insertion`.
    #[track_caller]
    pub fn end_insertion(&mut self, parent: Location) {
        let elems = std::mem::replace(
            &mut self.sink,
            self.stack.pop().expect("insertion to have been started"),
        );
        self.insertions.insert(parent, elems);
    }

    /// Builds a complete introspector with all acceleration structures from a
    /// list of top-level pairs.
    pub fn finalize(mut self) -> ElementIntrospector<P> {
        self.locations.reserve(self.seen.len());

        // Save all pairs and their descendants in the correct order.
        let mut elems = Vec::with_capacity(self.seen.len());
        for item in std::mem::take(&mut self.sink) {
            self.visit(&mut elems, item);
        }

        ElementIntrospector {
            elems,
            keys: self.keys,
            locations: self.locations,
            labels: self.labels,
            queries: QueryCache::default(),
        }
    }

    /// Saves a pair and all its descendants into `elems` and populates the
    /// acceleration structures.
    fn visit(&mut self, elems: &mut Vec<(Content, P)>, item: BuilderItem<P>) {
        match item {
            BuilderItem::Start(elem, loc, pos) => {
                let idx = elems.len();

                // Populate the location acceleration map. Initially, we insert
                // with a range covering just the element itself. Once we visit
                // the end tag, we update this information.
                self.locations.insert(loc, idx..idx + 1);

                // Populate the label acceleration map.
                if let Some(label) = elem.label() {
                    self.labels.insert(label, idx);
                }

                // Save the element.
                elems.push((elem, pos));

                // Process potential descendants.
                if let Some(insertions) = self.insertions.take(&loc) {
                    for pair in insertions.flatten() {
                        self.visit(elems, pair);
                    }
                }
            }
            BuilderItem::End(loc) => {
                // Update the end of the element's range.
                if let Some(entry) = self.locations.get_mut(&loc) {
                    entry.end = elems.len();
                }
            }
        }
    }
}

impl<P> Default for ElementIntrospectorBuilder<P> {
    fn default() -> Self {
        Self::new()
    }
}

/// A map from one keys to multiple elements.
#[derive(Clone)]
struct MultiMap<K, V>(FxHashMap<K, SmallVec<[V; 1]>>);

impl<K, V> MultiMap<K, V>
where
    K: Hash + Eq,
{
    fn get(&self, key: &K) -> &[V] {
        self.0.get(key).map_or(&[], |vec| vec.as_slice())
    }

    fn iter<'a>(&'a self) -> impl Iterator<Item = (&'a K, &'a [V])> + use<'a, K, V> {
        self.0.iter().map(|(k, v)| (k, v.as_slice()))
    }

    fn insert(&mut self, key: K, value: V) {
        self.0.entry(key).or_default().push(value);
    }

    fn insert_iter(&mut self, key: K, values: impl IntoIterator<Item = V>) {
        self.0.entry(key).or_default().extend(values);
    }

    fn take(&mut self, key: &K) -> Option<impl Iterator<Item = V> + use<K, V>> {
        self.0.remove(key).map(|vec| vec.into_iter())
    }

    fn extend(&mut self, other: &Self)
    where
        K: Clone,
        V: Clone,
    {
        for (key, locs) in other.iter() {
            self.insert_iter(key.clone(), locs.iter().cloned());
        }
    }
}

impl<K, V> Default for MultiMap<K, V> {
    fn default() -> Self {
        Self(FxHashMap::default())
    }
}

/// Caches queries.
#[derive(Default)]
struct QueryCache(RwLock<FxHashMap<u128, EcoVec<Content>>>);

impl QueryCache {
    fn get(&self, hash: u128) -> Option<EcoVec<Content>> {
        self.0.read().unwrap().get(&hash).cloned()
    }

    fn insert(&self, hash: u128, output: EcoVec<Content>) {
        self.0.write().unwrap().insert(hash, output);
    }
}

impl Clone for QueryCache {
    fn clone(&self) -> Self {
        Self(RwLock::new(self.0.read().unwrap().clone()))
    }
}
