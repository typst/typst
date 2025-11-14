use std::collections::BTreeSet;
use std::fmt::{self, Debug, Formatter};
use std::hash::Hash;
use std::num::NonZeroUsize;
use std::sync::RwLock;

use ecow::{EcoString, EcoVec};
use rustc_hash::{FxHashMap, FxHashSet};
use smallvec::SmallVec;
use typst_utils::NonZeroExt;

use crate::diag::{StrResult, bail};
use crate::foundations::{Content, Label, Repr, Selector};
use crate::introspection::{Location, Tag};
use crate::layout::{Frame, FrameItem, Point, Position, Transform};
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
    locations: FxHashMap<Location, usize>,
    /// Accelerates lookup of elements by label.
    labels: MultiMap<Label, usize>,

    /// Maps from element locations to assigned HTML IDs. This used to support
    /// intra-doc links in HTML export. In paged export, is is simply left
    /// empty and [`Self::html_id`] is not used.
    html_ids: FxHashMap<Location, EcoString>,

    /// Caches queries done on the introspector. This is important because
    /// even if all top-level queries are distinct, they often have shared
    /// subqueries. Example: Individual counter queries with `before` that
    /// all depend on a global counter query.
    queries: QueryCache,
}

/// A pair of content and its position.
type Pair = (Content, DocumentPosition);

impl Introspector {
    /// Iterates over all locatable elements.
    pub fn all(&self) -> impl Iterator<Item = &Content> + '_ {
        self.elems.iter().map(|(c, _)| c)
    }

    /// Checks how many times a label exists.
    pub fn label_count(&self, label: Label) -> usize {
        self.labels.get(&label).len()
    }

    /// Enriches an existing introspector with HTML IDs, which were assigned
    /// to the DOM in a post-processing step.
    pub fn set_html_ids(&mut self, html_ids: FxHashMap<Location, EcoString>) {
        self.html_ids = html_ids;
    }

    /// Retrieves the element with the given index.
    #[track_caller]
    fn get_by_idx(&self, idx: usize) -> &Content {
        &self.elems[idx].0
    }

    /// Retrieves the position of the element with the given index.
    #[track_caller]
    fn get_pos_by_idx(&self, idx: usize) -> DocumentPosition {
        self.elems[idx].1.clone()
    }

    /// Retrieves an element by its location.
    fn get_by_loc(&self, location: &Location) -> Option<&Content> {
        self.locations.get(location).map(|&idx| self.get_by_idx(idx))
    }

    /// Retrieves the position of the element with the given index.
    fn get_pos_by_loc(&self, location: &Location) -> Option<DocumentPosition> {
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
        match self.position(location) {
            DocumentPosition::Paged(position) => position.page,
            _ => NonZeroUsize::ONE,
        }
    }

    /// Find the position for the given location.
    pub fn position(&self, location: Location) -> DocumentPosition {
        self.get_pos_by_loc(&location)
            .unwrap_or(DocumentPosition::Paged(Position {
                page: NonZeroUsize::ONE,
                point: Point::zero(),
            }))
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

    /// Retrieves the ID to link to for this location in HTML export.
    pub fn html_id(&self, location: Location) -> Option<&EcoString> {
        self.html_ids.get(&location)
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
struct MultiMap<K, V>(FxHashMap<K, SmallVec<[V; 1]>>);

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

    fn take(&mut self, key: &K) -> Option<impl Iterator<Item = V> + use<K, V>> {
        self.0.remove(key).map(|vec| vec.into_iter())
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

/// Builds the introspector.
#[derive(Default)]
pub struct IntrospectorBuilder {
    pub pages: usize,
    pub page_numberings: Vec<Option<Numbering>>,
    pub page_supplements: Vec<Content>,
    pub html_ids: FxHashMap<Location, EcoString>,
    seen: FxHashSet<Location>,
    insertions: MultiMap<Location, Vec<Pair>>,
    keys: MultiMap<u128, Location>,
    locations: FxHashMap<Location, usize>,
    labels: MultiMap<Label, usize>,
}

impl IntrospectorBuilder {
    /// Create an empty builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Processes the tags in the frame.
    pub fn discover_in_frame(
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
                        self.register_insertion(parent.location, nested);
                    } else {
                        self.discover_in_frame(sink, &group.frame, page, ts);
                    }
                }
                FrameItem::Tag(tag) => {
                    self.discover_in_tag(
                        sink,
                        tag,
                        DocumentPosition::Paged(Position {
                            page,
                            point: pos.transform(ts),
                        }),
                    );
                }
                _ => {}
            }
        }
    }

    /// Handle a tag.
    pub fn discover_in_tag(
        &mut self,
        sink: &mut Vec<Pair>,
        tag: &Tag,
        position: DocumentPosition,
    ) {
        match tag {
            Tag::Start(elem, flags) => {
                if flags.introspectable {
                    let loc = elem.location().unwrap();
                    if self.seen.insert(loc) {
                        sink.push((elem.clone(), position));
                    }
                }
            }
            Tag::End(loc, key, flags) => {
                if flags.introspectable {
                    self.keys.insert(*key, *loc);
                }
            }
        }
    }

    /// Saves nested pairs as logically belonging to the `parent`.
    pub fn register_insertion(&mut self, parent: Location, nested: Vec<Pair>) {
        self.insertions.insert(parent, nested);
    }

    /// Build a complete introspector with all acceleration structures from a
    /// list of top-level pairs.
    pub fn finalize(mut self, root: Vec<Pair>) -> Introspector {
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
            html_ids: self.html_ids,
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

/// A position in an HTML-tree.
#[derive(Clone, Debug, Hash)]
pub struct HtmlPosition {
    /// Indices that can be used to traverse the tree from the root.
    element: EcoVec<usize>,
    /// Precise position inside of the specified element.
    inner: Option<InnerHtmlPosition>,
}

impl HtmlPosition {
    /// A position in an HTML pointing to a specific node as a whole.
    ///
    /// The items of the vector corresponds to indices that can be used to
    /// traverse the DOM tree from the root to reach the node. In practice, this
    /// means that the first item of the vector will often be `1` for the `<body>` tag
    /// (`0` being the `<head>` tag in a typical HTML document).
    pub fn new(element: EcoVec<usize>) -> Self {
        Self { element, inner: None }
    }

    /// Specify a character offset inside of the node, to build a position
    /// pointing to a specific point in text.
    ///
    /// This only makes sense if the node is a text node, not an element nor a
    /// frame.
    ///
    /// The offset is expressed in codepoints, not in bytes, to be
    /// encoding-independent.
    pub fn at_char(self, offset: usize) -> Self {
        Self {
            element: self.element,
            inner: Some(InnerHtmlPosition::Character(offset)),
        }
    }

    /// Specify a point in a frame, to build a more precise position.
    ///
    /// This only makes sense if the node is a frame.
    pub fn in_frame(self, point: Point) -> Self {
        Self {
            element: self.element,
            inner: Some(InnerHtmlPosition::Frame(point)),
        }
    }

    /// Extra-information for more a precise location inside of the node
    /// designated by [`HtmlPosition::element`].
    pub fn details(&self) -> Option<&InnerHtmlPosition> {
        self.inner.as_ref()
    }

    /// Indices to traverse an HTML tree to reach the node corresponding to this position.
    ///
    /// See [`HtmlPosition::new`] for more details.
    pub fn element(&self) -> impl Iterator<Item = &usize> {
        self.element.iter()
    }
}

/// Precise position inside of an HTML node.
#[derive(Clone, Debug, Hash)]
pub enum InnerHtmlPosition {
    /// If the node is a frame, the coordinates of the position.
    Frame(Point),
    /// If the node is a text node, the index of the codepoint at the position.
    Character(usize),
}

/// Physical position in a document, be it paged or HTML.
///
/// Only one variant should be used for all positions in a same document. This
/// type exists to make it possible to write functions that are generic over the
/// document target.
#[derive(Clone, Debug, Hash)]
pub enum DocumentPosition {
    /// If the document is paged, the position is expressed as coordinates
    /// inside of a page.
    Paged(Position),
    /// If the document is an HTML document, the position points to a specific
    /// node in the DOM tree.
    Html(HtmlPosition),
}

impl DocumentPosition {
    /// Returns the paged [`Position`] if available.
    pub fn as_paged(self) -> Option<Position> {
        match self {
            DocumentPosition::Paged(position) => Some(position),
            _ => None,
        }
    }

    pub fn as_paged_or_default(self) -> Position {
        self.as_paged()
            .unwrap_or(Position { page: NonZeroUsize::ONE, point: Point::zero() })
    }

    /// Returns the [`HtmlPosition`] if available.
    pub fn as_html(self) -> Option<HtmlPosition> {
        match self {
            DocumentPosition::Html(position) => Some(position),
            _ => None,
        }
    }
}

impl From<Position> for DocumentPosition {
    fn from(value: Position) -> Self {
        Self::Paged(value)
    }
}

impl From<HtmlPosition> for DocumentPosition {
    fn from(value: HtmlPosition) -> Self {
        Self::Html(value)
    }
}
