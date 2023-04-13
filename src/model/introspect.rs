use std::fmt::{self, Debug, Formatter};
use std::hash::Hash;
use std::num::NonZeroUsize;

use indexmap::IndexMap;

use super::{Content, Selector};
use crate::diag::StrResult;
use crate::doc::{Frame, FrameItem, Meta, Position};
use crate::eval::{cast_from_value, Value};
use crate::geom::{Point, Transform};
use crate::model::Label;
use crate::util::NonZeroExt;

/// Stably identifies an element in the document across multiple layout passes.
///
/// This struct is created by [`StabilityProvider::locate`].
#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub struct Location {
    /// The hash of the element.
    hash: u128,
    /// An unique number among elements with the same hash. This is the reason
    /// we need a mutable `StabilityProvider` everywhere.
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

impl Debug for Location {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.pad("..")
    }
}

cast_from_value! {
    Location: "location",
}

/// Provides stable identities to elements.
#[derive(Clone, Default)]
pub struct StabilityProvider {
    hashes: Vec<u128>,
    checkpoints: Vec<usize>,
}

impl StabilityProvider {
    /// Create a new stability provider.
    pub fn new() -> Self {
        Self::default()
    }
}

#[comemo::track]
impl StabilityProvider {
    /// Produce a stable identifier for this call site.
    pub fn locate(&mut self, hash: u128) -> Location {
        let disambiguator = self.hashes.iter().filter(|&&prev| prev == hash).count();
        self.hashes.push(hash);
        Location { hash, disambiguator, variant: 0 }
    }

    /// Create a checkpoint of the state that can be restored.
    pub fn save(&mut self) {
        self.checkpoints.push(self.hashes.len());
    }

    /// Restore the last checkpoint.
    pub fn restore(&mut self) {
        if let Some(checkpoint) = self.checkpoints.pop() {
            self.hashes.truncate(checkpoint);
        }
    }
}

/// Can be queried for elements and their positions.
pub struct Introspector {
    /// The number of pages in the document.
    pages: usize,
    /// All introspectable elements.
    elems: IndexMap<Location, (Content, Position)>,
    /// The page numberings, indexed by page number minus 1.
    page_numberings: Vec<Value>,
}

impl Introspector {
    /// Create a new introspector.
    pub fn new(frames: &[Frame]) -> Self {
        let mut introspector = Self {
            pages: frames.len(),
            elems: IndexMap::new(),
            page_numberings: vec![],
        };
        for (i, frame) in frames.iter().enumerate() {
            let page = NonZeroUsize::new(1 + i).unwrap();
            introspector.extract(frame, page, Transform::identity());
        }
        introspector
    }

    /// Iterate over all elements.
    pub fn all(&self) -> impl Iterator<Item = Content> + '_ {
        self.elems.values().map(|(c, _)| c).cloned()
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
                }
                FrameItem::Meta(Meta::PageNumbering(numbering), _) => {
                    self.page_numberings.push(numbering.clone());
                }
                _ => {}
            }
        }
    }
}

#[comemo::track]
impl Introspector {
    /// Whether this introspector is not yet initialized.
    pub fn init(&self) -> bool {
        self.pages > 0
    }

    /// Get an element from the position cache.
    pub fn location(&self, location: &Location) -> Option<Content> {
        self.elems.get(location).map(|(c, _)| c).cloned()
    }

    /// Query for all matching elements.
    pub fn query<'a>(&'a self, selector: &'a Selector) -> Vec<Content> {
        match selector {
            Selector::Location(location) => self
                .elems
                .get(location)
                .map(|(content, _)| content)
                .cloned()
                .into_iter()
                .collect(),
            _ => selector.match_iter(self).collect(),
        }
    }

    /// Query for the first matching element.
    pub fn query_first<'a>(&'a self, selector: &'a Selector) -> Option<Content> {
        match selector {
            Selector::Location(location) => {
                self.elems.get(location).map(|(content, _)| content).cloned()
            }
            _ => selector.match_iter(self).next(),
        }
    }

    /// Query for a unique element with the label.
    pub fn query_label(&self, label: &Label) -> StrResult<Content> {
        let mut found = None;
        for elem in self.all().filter(|elem| elem.label() == Some(label)) {
            if found.is_some() {
                return Err("label occurs multiple times in the document".into());
            }
            found = Some(elem.clone());
        }
        found.ok_or_else(|| "label does not exist in the document".into())
    }

    /// The total number pages.
    pub fn pages(&self) -> NonZeroUsize {
        NonZeroUsize::new(self.pages).unwrap_or(NonZeroUsize::ONE)
    }

    /// Find the page number for the given location.
    pub fn page(&self, location: Location) -> NonZeroUsize {
        self.position(location).page
    }

    /// Gets the page numbering for the given location, if any.
    pub fn page_numbering(&self, location: Location) -> Value {
        let page = self.page(location);
        self.page_numberings.get(page.get() - 1).cloned().unwrap_or_default()
    }

    /// Find the position for the given location.
    pub fn position(&self, location: Location) -> Position {
        self.elems
            .get(&location)
            .map(|(_, loc)| *loc)
            .unwrap_or(Position { page: NonZeroUsize::ONE, point: Point::zero() })
    }

    /// Checks whether `a` is before `b` in the document.
    pub fn is_before(&self, a: Location, b: Location, inclusive: bool) -> bool {
        let a = self.elems.get_index_of(&a).unwrap();
        let b = self.elems.get_index_of(&b).unwrap();
        if inclusive {
            a <= b
        } else {
            a < b
        }
    }

    /// Checks whether `a` is after `b` in the document.
    pub fn is_after(&self, a: Location, b: Location, inclusive: bool) -> bool {
        !self.is_before(a, b, !inclusive)
    }
}
