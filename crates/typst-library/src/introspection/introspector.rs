use std::collections::{BTreeSet, HashMap, HashSet};
use std::fmt::{self, Debug, Formatter};
use std::hash::Hash;
use std::num::NonZeroUsize;
use std::sync::RwLock;

use ecow::EcoVec;
use smallvec::SmallVec;
use typst_utils::NonZeroExt;

use crate::diag::{bail, StrResult};
use crate::foundations::{Content, Label, Repr, Selector};
use crate::html::{HtmlElement, HtmlNode};
use crate::introspection::{Location, Tag};
use crate::layout::{Frame, FrameItem, Page, Point, Position, Transform};
use crate::model::Numbering;

/// Can be queried for elements and their positions.
#[derive(Default, Clone)]
pub struct Introspector {
    /// The number of pages in the document.
    pages: usize,
    /// The page numberings, indexed by page number minus 1.
    page_numberings: Vec<Option<Numbering>>,
    /// The page supplements, indexed by page number minus 1.
    page_supplements: Vec<Content>,

    /// All introspectable elements.
    elems: Vec<Pair>,
    /// Lists all elements with a specific hash key. This is used for
    /// introspector-assisted location assignment during measurement.
    keys: MultiMap<u128, Location>,

    /// Accelerates lookup of elements by location.
    locations: HashMap<Location, usize>,
    /// Accelerates lookup of elements by label.
    labels: MultiMap<Label, usize>,

    /// Caches queries done on the introspector. This is important because
    /// even if all top-level queries are distinct, they often have shared
    /// subqueries. Example: Individual counter queries with `before` that
    /// all depend on a global counter query.
    queries: QueryCache,
}

/// A pair of content and its position.
type Pair = (Content, Position);

impl Introspector {
    /// Creates an introspector for a page list.
    #[typst_macros::time(name = "introspect pages")]
    pub fn paged(pages: &[Page]) -> Self {
        IntrospectorBuilder::new().build_paged(pages)
    }

    /// Creates an introspector for HTML.
    #[typst_macros::time(name = "introspect html")]
    pub fn html(root: &HtmlElement) -> Self {
        IntrospectorBuilder::new().build_html(root)
    }

    /// Iterates over all locatable elements.
    pub fn all(&self) -> impl Iterator<Item = &Content> + '_ {
        self.elems.iter().map(|(c, _)| c)
    }

    /// Retrieves the element with the given index.
    #[track_caller]
    fn get_by_idx(&self, idx: usize) -> &Content {
        &self.elems[idx].0
    }

    /// Retrieves the position of the element with the given index.
    #[track_caller]
    fn get_pos_by_idx(&self, idx: usize) -> Position {
        self.elems[idx].1
    }

    /// Retrieves an element by its location.
    fn get_by_loc(&self, location: &Location) -> Option<&Content> {
        self.locations.get(location).map(|&idx| self.get_by_idx(idx))
    }

    /// Retrieves the position of the element with the given index.
    fn get_pos_by_loc(&self, location: &Location) -> Option<Position> {
        self.locations.get(location).map(|&idx| self.get_pos_by_idx(idx))
    }

    /// Performs a binary search for `elem` among the `list`.
    fn binary_search(&self, list: &[Content], elem: &Content) -> Result<usize, usize> {
        list.binary_search_by_key(&self.elem_index(elem), |elem| self.elem_index(elem))
    }

    /// Gets the index of this element.
    fn elem_index(&self, elem: &Content) -> usize {
        self.loc_index(&elem.location().unwrap())
    }

    /// Gets the index of the element with this location among all.
    fn loc_index(&self, location: &Location) -> usize {
        self.locations.get(location).copied().unwrap_or(usize::MAX)
    }
}

#[comemo::track]
impl Introspector {
    /// Query for all matching elements.
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
            Selector::Within { selector, ancestor } => self
                .query(ancestor)
                .iter()
                .flat_map(|children| children.query(selector))
                .collect(),
            // Not supported here.
            Selector::Can(_) | Selector::Regex(_) => EcoVec::new(),
        };

        self.queries.insert(hash, output.clone());
        output
    }

    /// Query for the first element that matches the selector.
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

    /// Query for the first element that matches the selector.
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

    /// Query for a unique element with the label.
    pub fn query_label(&self, label: Label) -> StrResult<&Content> {
        match *self.labels.get(&label) {
            [idx] => Ok(self.get_by_idx(idx)),
            [] => bail!("label `{}` does not exist in the document", label.repr()),
            _ => bail!("label `{}` occurs multiple times in the document", label.repr()),
        }
    }

    /// This is an optimized version of
    /// `query(selector.before(end, true).len()` used by counters and state.
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

    /// The total number pages.
    pub fn pages(&self) -> NonZeroUsize {
        NonZeroUsize::new(self.pages).unwrap_or(NonZeroUsize::ONE)
    }

    /// Find the page number for the given location.
    pub fn page(&self, location: Location) -> NonZeroUsize {
        self.position(location).page
    }

    /// Find the position for the given location.
    pub fn position(&self, location: Location) -> Position {
        self.get_pos_by_loc(&location)
            .unwrap_or(Position { page: NonZeroUsize::ONE, point: Point::zero() })
    }

    /// Gets the page numbering for the given location, if any.
    pub fn page_numbering(&self, location: Location) -> Option<&Numbering> {
        let page = self.page(location);
        self.page_numberings
            .get(page.get() - 1)
            .and_then(|slot| slot.as_ref())
    }

    /// Gets the page supplement for the given location, if any.
    pub fn page_supplement(&self, location: Location) -> Content {
        let page = self.page(location);
        self.page_supplements.get(page.get() - 1).cloned().unwrap_or_default()
    }

    /// Try to find a location for an element with the given `key` hash
    /// that is closest after the `anchor`.
    ///
    /// This is used for introspector-assisted location assignment during
    /// measurement. See the "Dealing with Measurement" section of the
    /// [`Locator`](crate::introspection::Locator) docs for more details.
    pub fn locator(&self, key: u128, anchor: Location) -> Option<Location> {
        let anchor = self.loc_index(&anchor);
        self.keys
            .get(&key)
            .iter()
            .copied()
            .min_by_key(|loc| self.loc_index(loc).wrapping_sub(anchor))
    }
}

impl Debug for Introspector {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.pad("Introspector(..)")
    }
}

/// A map from one keys to multiple elements.
#[derive(Clone)]
struct MultiMap<K, V>(HashMap<K, SmallVec<[V; 1]>>);

impl<K, V> MultiMap<K, V>
where
    K: Hash + Eq,
{
    fn get(&self, key: &K) -> &[V] {
        self.0.get(key).map_or(&[], |vec| vec.as_slice())
    }

    fn insert(&mut self, key: K, value: V) {
        self.0.entry(key).or_default().push(value);
    }

    fn take(&mut self, key: &K) -> Option<impl Iterator<Item = V>> {
        self.0.remove(key).map(|vec| vec.into_iter())
    }
}

impl<K, V> Default for MultiMap<K, V> {
    fn default() -> Self {
        Self(HashMap::new())
    }
}

/// Caches queries.
#[derive(Default)]
struct QueryCache(RwLock<HashMap<u128, EcoVec<Content>>>);

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

/// Builds the introspector.
#[derive(Default)]
struct IntrospectorBuilder {
    pages: usize,
    page_numberings: Vec<Option<Numbering>>,
    page_supplements: Vec<Content>,
    seen: HashSet<Location>,
    insertions: MultiMap<Location, Vec<Pair>>,
    keys: MultiMap<u128, Location>,
    locations: HashMap<Location, usize>,
    labels: MultiMap<Label, usize>,
}

impl IntrospectorBuilder {
    /// Create an empty builder.
    fn new() -> Self {
        Self::default()
    }

    /// Build an introspector for a page list.
    fn build_paged(mut self, pages: &[Page]) -> Introspector {
        self.pages = pages.len();
        self.page_numberings.reserve(pages.len());
        self.page_supplements.reserve(pages.len());

        // Discover all elements.
        let mut elems = Vec::new();
        for (i, page) in pages.iter().enumerate() {
            self.page_numberings.push(page.numbering.clone());
            self.page_supplements.push(page.supplement.clone());
            self.discover_in_frame(
                &mut elems,
                &page.frame,
                NonZeroUsize::new(1 + i).unwrap(),
                Transform::identity(),
            );
        }

        self.finalize(elems)
    }

    /// Build an introspector for an HTML document.
    fn build_html(mut self, root: &HtmlElement) -> Introspector {
        let mut elems = Vec::new();
        self.discover_in_html(&mut elems, root);
        self.finalize(elems)
    }

    /// Processes the tags in the frame.
    fn discover_in_frame(
        &mut self,
        sink: &mut Vec<Pair>,
        frame: &Frame,
        page: NonZeroUsize,
        ts: Transform,
    ) {
        for (pos, item) in frame.items() {
            match item {
                FrameItem::Group(group) => {
                    let ts = ts
                        .pre_concat(Transform::translate(pos.x, pos.y))
                        .pre_concat(group.transform);

                    if let Some(parent) = group.parent {
                        let mut nested = vec![];
                        self.discover_in_frame(&mut nested, &group.frame, page, ts);
                        self.insertions.insert(parent, nested);
                    } else {
                        self.discover_in_frame(sink, &group.frame, page, ts);
                    }
                }
                FrameItem::Tag(tag) => {
                    self.discover_in_tag(
                        sink,
                        tag,
                        Position { page, point: pos.transform(ts) },
                    );
                }
                _ => {}
            }
        }
    }

    /// Processes the tags in the HTML element.
    fn discover_in_html(&mut self, sink: &mut Vec<Pair>, elem: &HtmlElement) {
        for child in &elem.children {
            match child {
                HtmlNode::Tag(tag) => self.discover_in_tag(
                    sink,
                    tag,
                    Position { page: NonZeroUsize::ONE, point: Point::zero() },
                ),
                HtmlNode::Text(_, _) => {}
                HtmlNode::Element(elem) => self.discover_in_html(sink, elem),
                HtmlNode::Frame(frame) => self.discover_in_frame(
                    sink,
                    frame,
                    NonZeroUsize::ONE,
                    Transform::identity(),
                ),
            }
        }
    }

    /// Handle a tag.
    fn discover_in_tag(&mut self, sink: &mut Vec<Pair>, tag: &Tag, position: Position) {
        match tag {
            Tag::Start(elem) => {
                let loc = elem.location().unwrap();
                if self.seen.insert(loc) {
                    sink.push((elem.clone(), position));
                }
            }
            Tag::End(loc, key) => {
                self.keys.insert(*key, *loc);
            }
        }
    }

    /// Build a complete introspector with all acceleration structures from a
    /// list of top-level pairs.
    fn finalize(mut self, root: Vec<Pair>) -> Introspector {
        self.locations.reserve(self.seen.len());

        // Save all pairs and their descendants in the correct order.
        let mut elems = Vec::with_capacity(self.seen.len());
        for pair in root {
            self.visit(&mut elems, pair);
        }

        Introspector {
            pages: self.pages,
            page_numberings: self.page_numberings,
            page_supplements: self.page_supplements,
            elems,
            keys: self.keys,
            locations: self.locations,
            labels: self.labels,
            queries: QueryCache::default(),
        }
    }

    /// Saves a pair and all its descendants into `elems` and populates the
    /// acceleration structures.
    fn visit(&mut self, elems: &mut Vec<Pair>, pair: Pair) {
        let elem = &pair.0;
        let loc = elem.location().unwrap();
        let idx = elems.len();

        // Populate the location acceleration map.
        self.locations.insert(loc, idx);

        // Populate the label acceleration map.
        if let Some(label) = elem.label() {
            self.labels.insert(label, idx);
        }

        // Save the element.
        elems.push(pair);

        // Process potential descendants.
        if let Some(insertions) = self.insertions.take(&loc) {
            for pair in insertions.flatten() {
                self.visit(elems, pair);
            }
        }
    }
}
