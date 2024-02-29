use std::collections::{BTreeSet, HashMap};
use std::fmt::{self, Debug, Formatter};
use std::hash::Hash;
use std::num::NonZeroUsize;
use std::sync::RwLock;

use ecow::{eco_format, EcoVec};
use indexmap::IndexMap;
use smallvec::SmallVec;

use crate::diag::{bail, StrResult};
use crate::foundations::{Content, Label, Repr, Selector};
use crate::introspection::{Location, Meta};
use crate::layout::{Frame, FrameItem, Page, Point, Position, Transform};
use crate::model::Numbering;
use crate::util::NonZeroExt;

/// Can be queried for elements and their positions.
#[derive(Clone)]
pub struct Introspector {
    /// The number of pages in the document.
    pages: usize,
    /// All introspectable elements.
    elems: IndexMap<Location, (Content, Position)>,
    /// Maps labels to their indices in the element list. We use a smallvec such
    /// that if the label is unique, we don't need to allocate.
    labels: HashMap<Label, SmallVec<[usize; 1]>>,
    /// The page numberings, indexed by page number minus 1.
    page_numberings: Vec<Option<Numbering>>,
    /// Caches queries done on the introspector. This is important because
    /// even if all top-level queries are distinct, they often have shared
    /// subqueries. Example: Individual counter queries with `before` that
    /// all depend on a global counter query.
    queries: QueryCache,
}

impl Introspector {
    /// Applies new frames in-place, reusing the existing allocations.
    #[typst_macros::time(name = "introspect")]
    pub fn rebuild(&mut self, pages: &[Page]) {
        self.pages = pages.len();
        self.elems.clear();
        self.labels.clear();
        self.page_numberings.clear();
        self.queries.clear();

        for (i, page) in pages.iter().enumerate() {
            let page_nr = NonZeroUsize::new(1 + i).unwrap();
            self.extract(&page.frame, page_nr, Transform::identity());
            self.page_numberings.push(page.numbering.clone());
        }
    }

    /// Extract metadata from a frame.
    fn extract(&mut self, frame: &Frame, page: NonZeroUsize, ts: Transform) {
        for (pos, item) in frame.items() {
            match item {
                FrameItem::Group(group) => {
                    let ts = ts
                        .pre_concat(Transform::translate(pos.x, pos.y))
                        .pre_concat(group.transform);
                    self.extract(&group.frame, page, ts);
                }
                FrameItem::Meta(Meta::Elem(content), _)
                    if !self.elems.contains_key(&content.location().unwrap()) =>
                {
                    let pos = pos.transform(ts);
                    let ret = self.elems.insert(
                        content.location().unwrap(),
                        (content.clone(), Position { page, point: pos }),
                    );
                    assert!(ret.is_none(), "duplicate locations");

                    // Build the label cache.
                    if let Some(label) = content.label() {
                        self.labels.entry(label).or_default().push(self.elems.len() - 1);
                    }
                }
                _ => {}
            }
        }
    }

    /// Iterate over all locatable elements.
    pub fn all(&self) -> impl Iterator<Item = &Content> + '_ {
        self.elems.values().map(|(c, _)| c)
    }

    /// Get an element by its location.
    fn get(&self, location: &Location) -> Option<&Content> {
        self.elems.get(location).map(|(elem, _)| elem)
    }

    /// Get the index of this element among all.
    fn index(&self, elem: &Content) -> usize {
        self.elems
            .get_index_of(&elem.location().unwrap())
            .unwrap_or(usize::MAX)
    }

    /// Perform a binary search for `elem` among the `list`.
    fn binary_search(&self, list: &[Content], elem: &Content) -> Result<usize, usize> {
        list.binary_search_by_key(&self.index(elem), |elem| self.index(elem))
    }
}

#[comemo::track]
impl Introspector {
    /// Query for all matching elements.
    pub fn query(&self, selector: &Selector) -> EcoVec<Content> {
        let hash = crate::util::hash128(selector);
        if let Some(output) = self.queries.get(hash) {
            return output;
        }

        let output = match selector {
            Selector::Label(label) => self
                .labels
                .get(label)
                .map(|indices| {
                    indices.iter().map(|&index| self.elems[index].0.clone()).collect()
                })
                .unwrap_or_default(),
            Selector::Elem(..) | Selector::Regex(_) | Selector::Can(_) => self
                .all()
                .filter(|elem| selector.matches(elem, None))
                .cloned()
                .collect(),
            Selector::Location(location) => {
                self.get(location).cloned().into_iter().collect()
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
            Selector::Or(selectors) => selectors
                .iter()
                .flat_map(|sel| self.query(sel))
                .map(|elem| self.index(&elem))
                .collect::<BTreeSet<usize>>()
                .into_iter()
                .map(|index| self.elems[index].0.clone())
                .collect(),
        };

        self.queries.insert(hash, output.clone());
        output
    }

    /// Query for the first element that matches the selector.
    pub fn query_first(&self, selector: &Selector) -> Option<Content> {
        match selector {
            Selector::Location(location) => self.get(location).cloned(),
            _ => self.query(selector).first().cloned(),
        }
    }

    /// Query for the first element that matches the selector.
    pub fn query_unique(&self, selector: &Selector) -> StrResult<Content> {
        match selector {
            Selector::Location(location) => self
                .get(location)
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
        let indices = self.labels.get(&label).ok_or_else(|| {
            eco_format!("label `{}` does not exist in the document", label.repr())
        })?;

        if indices.len() > 1 {
            bail!("label `{}` occurs multiple times in the document", label.repr());
        }

        Ok(&self.elems[indices[0]].0)
    }

    /// The total number pages.
    pub fn pages(&self) -> NonZeroUsize {
        NonZeroUsize::new(self.pages).unwrap_or(NonZeroUsize::ONE)
    }

    /// Gets the page numbering for the given location, if any.
    pub fn page_numbering(&self, location: Location) -> Option<&Numbering> {
        let page = self.page(location);
        self.page_numberings
            .get(page.get() - 1)
            .and_then(|slot| slot.as_ref())
    }

    /// Find the page number for the given location.
    pub fn page(&self, location: Location) -> NonZeroUsize {
        self.position(location).page
    }

    /// Find the position for the given location.
    pub fn position(&self, location: Location) -> Position {
        self.elems
            .get(&location)
            .map(|(_, loc)| *loc)
            .unwrap_or(Position { page: NonZeroUsize::ONE, point: Point::zero() })
    }
}

impl Default for Introspector {
    fn default() -> Self {
        Self {
            pages: 0,
            elems: IndexMap::new(),
            labels: HashMap::new(),
            page_numberings: vec![],
            queries: QueryCache::default(),
        }
    }
}

impl Debug for Introspector {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.pad("Introspector(..)")
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

    fn clear(&mut self) {
        self.0.get_mut().unwrap().clear();
    }
}

impl Clone for QueryCache {
    fn clone(&self) -> Self {
        Self(RwLock::new(self.0.read().unwrap().clone()))
    }
}
