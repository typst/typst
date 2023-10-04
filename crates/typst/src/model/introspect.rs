use std::cell::RefCell;
use std::collections::{BTreeSet, HashMap};
use std::fmt::Debug;
use std::hash::Hash;
use std::num::NonZeroUsize;

use comemo::{Prehashed, Track, Tracked, Validate};
use ecow::{EcoString, EcoVec};
use indexmap::IndexMap;

use super::{Content, Selector};
use crate::diag::{bail, StrResult};
use crate::doc::{Frame, FrameItem, Meta, Position};
use crate::eval::{cast, func, scope, ty, Dict, Repr, Value, Vm};
use crate::geom::{Point, Transform};
use crate::model::Label;
use crate::util::NonZeroExt;

/// Identifies an element in the document.
///
/// A location uniquely identifies an element in the document and lets you
/// access its absolute position on the pages. You can retrieve the current
/// location with the [`locate`]($locate) function and the location of a queried
/// or shown element with the [`location()`]($content.location) method on
/// content.
#[ty(scope)]
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct Location {
    /// The hash of the element.
    hash: u128,
    /// An unique number among elements with the same hash. This is the reason
    /// we need a `Locator` everywhere.
    disambiguator: usize,
    /// A synthetic location created from another one. This is used for example
    /// in bibliography management to create individual linkable locations for
    /// reference entries from the bibliography's location.
    variant: usize,
}

impl Location {
    /// Produce a variant of this location.
    pub fn variant(mut self, n: usize) -> Self {
        self.variant = n;
        self
    }
}

#[scope]
impl Location {
    /// Return the page number for this location.
    ///
    /// Note that this does not return the value of the [page counter]($counter)
    /// at this location, but the true page number (starting from one).
    ///
    /// If you want to know the value of the page counter, use
    /// `{counter(page).at(loc)}` instead.
    #[func]
    pub fn page(self, vm: &mut Vm) -> NonZeroUsize {
        vm.vt.introspector.page(self)
    }

    /// Return a dictionary with the page number and the x, y position for this
    /// location. The page number starts at one and the coordinates are measured
    /// from the top-left of the page.
    ///
    /// If you only need the page number, use `page()` instead as it allows
    /// Typst to skip unnecessary work.
    #[func]
    pub fn position(self, vm: &mut Vm) -> Dict {
        vm.vt.introspector.position(self).into()
    }

    /// Returns the page numbering pattern of the page at this location. This
    /// can be used when displaying the page counter in order to obtain the
    /// local numbering. This is useful if you are building custom indices or
    /// outlines.
    ///
    /// If the page numbering is set to `none` at that location, this function
    /// returns `none`.
    #[func]
    pub fn page_numbering(self, vm: &mut Vm) -> Value {
        vm.vt.introspector.page_numbering(self)
    }
}

impl Repr for Location {
    fn repr(&self) -> EcoString {
        "..".into()
    }
}

cast! {
    type Location,
}

/// Provides locations for elements in the document.
///
/// A [`Location`] consists of an element's hash plus a disambiguator. Just the
/// hash is not enough because we can have multiple equal elements with the same
/// hash (not a hash collision, just equal elements!). Between these, we
/// disambiguate with an increasing number. In principle, the disambiguator
/// could just be counted up. However, counting is an impure operation and as
/// such we can't count across a memoization boundary. [^1]
///
/// Instead, we only mutate within a single "layout run" and combine the results
/// with disambiguators from an outer tracked locator. Thus, the locators form a
/// "tracked chain". When a layout run ends, its mutations are discarded and, on
/// the other side of the memoization boundary, we
/// [reconstruct](Self::visit_frame) them from the resulting [frames](Frame).
///
/// [^1]: Well, we could with [`TrackedMut`](comemo::TrackedMut), but the
/// overhead is quite high, especially since we need to save & undo the counting
/// when only measuring.
#[derive(Default, Clone)]
pub struct Locator<'a> {
    /// Maps from a hash to the maximum number we've seen for this hash. This
    /// number becomes the `disambiguator`.
    hashes: RefCell<HashMap<u128, usize>>,
    /// An outer `Locator`, from which we can get disambiguator for hashes
    /// outside of the current "layout run".
    ///
    /// We need to override the constraint's lifetime here so that `Tracked` is
    /// covariant over the constraint. If it becomes invariant, we're in for a
    /// world of lifetime pain.
    outer: Option<Tracked<'a, Self, <Locator<'static> as Validate>::Constraint>>,
}

impl<'a> Locator<'a> {
    /// Create a new locator.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new chained locator.
    pub fn chained(outer: Tracked<'a, Self>) -> Self {
        Self { outer: Some(outer), ..Default::default() }
    }

    /// Start tracking this locator.
    ///
    /// In comparison to [`Track::track`], this method skips this chain link
    /// if it does not contribute anything.
    pub fn track(&self) -> Tracked<'_, Self> {
        match self.outer {
            Some(outer) if self.hashes.borrow().is_empty() => outer,
            _ => Track::track(self),
        }
    }

    /// Produce a stable identifier for this call site.
    pub fn locate(&mut self, hash: u128) -> Location {
        // Get the current disambiguator for this hash.
        let disambiguator = self.disambiguator_impl(hash);

        // Bump the next disambiguator up by one.
        self.hashes.borrow_mut().insert(hash, disambiguator + 1);

        // Create the location in its default variant.
        Location { hash, disambiguator, variant: 0 }
    }

    /// Advance past a frame.
    pub fn visit_frame(&mut self, frame: &Frame) {
        for (_, item) in frame.items() {
            match item {
                FrameItem::Group(group) => self.visit_frame(&group.frame),
                FrameItem::Meta(Meta::Elem(elem), _) => {
                    let mut hashes = self.hashes.borrow_mut();
                    let loc = elem.location().unwrap();
                    let entry = hashes.entry(loc.hash).or_default();

                    // Next disambiguator needs to be at least one larger than
                    // the maximum we've seen so far.
                    *entry = (*entry).max(loc.disambiguator + 1);
                }
                _ => {}
            }
        }
    }

    /// Advance past a number of frames.
    pub fn visit_frames<'b>(&mut self, frames: impl IntoIterator<Item = &'b Frame>) {
        for frame in frames {
            self.visit_frame(frame);
        }
    }

    /// The current disambiguator for the given hash.
    fn disambiguator_impl(&self, hash: u128) -> usize {
        *self
            .hashes
            .borrow_mut()
            .entry(hash)
            .or_insert_with(|| self.outer.map_or(0, |outer| outer.disambiguator(hash)))
    }
}

#[comemo::track]
impl<'a> Locator<'a> {
    /// The current disambiguator for the hash.
    fn disambiguator(&self, hash: u128) -> usize {
        self.disambiguator_impl(hash)
    }
}

/// Can be queried for elements and their positions.
pub struct Introspector {
    /// The number of pages in the document.
    pages: usize,
    /// All introspectable elements.
    elems: IndexMap<Location, (Prehashed<Content>, Position)>,
    /// The page numberings, indexed by page number minus 1.
    page_numberings: Vec<Value>,
    /// Caches queries done on the introspector. This is important because
    /// even if all top-level queries are distinct, they often have shared
    /// subqueries. Example: Individual counter queries with `before` that
    /// all depend on a global counter query.
    queries: RefCell<HashMap<u128, EcoVec<Prehashed<Content>>>>,
}

impl Introspector {
    /// Create a new introspector.
    #[tracing::instrument(skip(frames))]
    pub fn new(frames: &[Frame]) -> Self {
        let mut introspector = Self {
            pages: frames.len(),
            elems: IndexMap::new(),
            page_numberings: vec![],
            queries: RefCell::default(),
        };
        for (i, frame) in frames.iter().enumerate() {
            let page = NonZeroUsize::new(1 + i).unwrap();
            introspector.extract(frame, page, Transform::identity());
        }
        introspector
    }

    /// Extract metadata from a frame.
    #[tracing::instrument(skip_all)]
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
                        (Prehashed::new(content.clone()), Position { page, point: pos }),
                    );
                    assert!(ret.is_none(), "duplicate locations");
                }
                FrameItem::Meta(Meta::PageNumbering(numbering), _) => {
                    self.page_numberings.push(numbering.clone());
                }
                _ => {}
            }
        }
    }

    /// Iterate over all locatable elements.
    pub fn all(&self) -> impl Iterator<Item = &Prehashed<Content>> + '_ {
        self.elems.values().map(|(c, _)| c)
    }

    /// Get an element by its location.
    fn get(&self, location: &Location) -> Option<&Prehashed<Content>> {
        self.elems.get(location).map(|(elem, _)| elem)
    }

    /// Get the index of this element among all.
    fn index(&self, elem: &Content) -> usize {
        self.elems
            .get_index_of(&elem.location().unwrap())
            .unwrap_or(usize::MAX)
    }

    /// Perform a binary search for `elem` among the `list`.
    fn binary_search(
        &self,
        list: &[Prehashed<Content>],
        elem: &Content,
    ) -> Result<usize, usize> {
        list.binary_search_by_key(&self.index(elem), |elem| self.index(elem))
    }
}

#[comemo::track]
impl Introspector {
    /// Query for all matching elements.
    pub fn query(&self, selector: &Selector) -> EcoVec<Prehashed<Content>> {
        let hash = crate::util::hash128(selector);
        if let Some(output) = self.queries.borrow().get(&hash) {
            return output.clone();
        }

        let output = match selector {
            Selector::Elem(..)
            | Selector::Label(_)
            | Selector::Regex(_)
            | Selector::Can(_) => {
                self.all().filter(|elem| selector.matches(elem)).cloned().collect()
            }
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

        self.queries.borrow_mut().insert(hash, output.clone());
        output
    }

    /// Query for the first element that matches the selector.
    pub fn query_first(&self, selector: &Selector) -> Option<Prehashed<Content>> {
        match selector {
            Selector::Location(location) => self.get(location).cloned(),
            _ => self.query(selector).first().cloned(),
        }
    }

    /// Query for a unique element with the label.
    pub fn query_label(&self, label: &Label) -> StrResult<Prehashed<Content>> {
        let mut found = None;
        for elem in self.all().filter(|elem| elem.label() == Some(label)) {
            if found.is_some() {
                bail!("label occurs multiple times in the document");
            }
            found = Some(elem.clone());
        }
        found.ok_or_else(|| "label does not exist in the document".into())
    }

    /// The total number pages.
    pub fn pages(&self) -> NonZeroUsize {
        NonZeroUsize::new(self.pages).unwrap_or(NonZeroUsize::ONE)
    }

    /// Gets the page numbering for the given location, if any.
    pub fn page_numbering(&self, location: Location) -> Value {
        let page = self.page(location);
        self.page_numberings.get(page.get() - 1).cloned().unwrap_or_default()
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
        Self::new(&[])
    }
}
