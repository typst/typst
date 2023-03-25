use std::fmt::{self, Debug, Formatter};
use std::hash::Hash;
use std::num::NonZeroUsize;

use super::{Content, Selector};
use crate::doc::{Frame, FrameItem, Meta, Position};
use crate::eval::cast_from_value;
use crate::geom::{Point, Transform};
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
#[derive(Clone)]
pub struct StabilityProvider {
    hashes: Vec<u128>,
    checkpoints: Vec<usize>,
}

impl StabilityProvider {
    /// Create a new stability provider.
    pub fn new() -> Self {
        Self { hashes: vec![], checkpoints: vec![] }
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
    pages: usize,
    elems: Vec<(Content, Position)>,
}

impl Introspector {
    /// Create a new introspector.
    pub fn new(frames: &[Frame]) -> Self {
        let mut introspector = Self { pages: frames.len(), elems: vec![] };
        for (i, frame) in frames.iter().enumerate() {
            let page = NonZeroUsize::new(1 + i).unwrap();
            introspector.extract(frame, page, Transform::identity());
        }
        introspector
    }

    /// Iterate over all elements.
    pub fn all(&self) -> impl Iterator<Item = &Content> {
        self.elems.iter().map(|(elem, _)| elem)
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
                    if !self
                        .elems
                        .iter()
                        .any(|(prev, _)| prev.location() == content.location()) =>
                {
                    let pos = pos.transform(ts);
                    self.elems.push((content.clone(), Position { page, point: pos }));
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

    /// Query for all matching elements.
    pub fn query(&self, selector: Selector) -> Vec<Content> {
        self.all().filter(|elem| selector.matches(elem)).cloned().collect()
    }

    /// Query for all matching element up to the given location.
    pub fn query_before(&self, selector: Selector, location: Location) -> Vec<Content> {
        let mut matches = vec![];
        for elem in self.all() {
            if selector.matches(elem) {
                matches.push(elem.clone());
            }
            if elem.location() == Some(location) {
                break;
            }
        }
        matches
    }

    /// Query for all matching elements starting from the given location.
    pub fn query_after(&self, selector: Selector, location: Location) -> Vec<Content> {
        self.all()
            .skip_while(|elem| elem.location() != Some(location))
            .filter(|elem| selector.matches(elem))
            .cloned()
            .collect()
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
        self.elems
            .iter()
            .find(|(elem, _)| elem.location() == Some(location))
            .map_or(
                Position { page: NonZeroUsize::ONE, point: Point::zero() },
                |(_, loc)| *loc,
            )
    }
}
