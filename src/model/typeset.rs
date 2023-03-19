use std::fmt::{self, Debug, Formatter};
use std::hash::Hash;
use std::num::NonZeroUsize;

use comemo::{Constraint, Track, Tracked, TrackedMut};

use super::{Content, Selector, StyleChain};
use crate::diag::SourceResult;
use crate::doc::{Document, Element, Frame, Location, Meta};
use crate::eval::{cast_from_value, Tracer};
use crate::geom::{Point, Transform};
use crate::util::NonZeroExt;
use crate::World;

/// Typeset content into a fully layouted document.
#[comemo::memoize]
pub fn typeset(
    world: Tracked<dyn World>,
    mut tracer: TrackedMut<Tracer>,
    content: &Content,
) -> SourceResult<Document> {
    let library = world.library();
    let styles = StyleChain::new(&library.styles);

    let mut document;
    let mut iter = 0;
    let mut introspector = Introspector::new(&[]);

    // Relayout until all introspections stabilize.
    // If that doesn't happen within five attempts, we give up.
    loop {
        let constraint = Constraint::new();
        let mut provider = StabilityProvider::new();
        let mut vt = Vt {
            world,
            tracer: TrackedMut::reborrow_mut(&mut tracer),
            provider: provider.track_mut(),
            introspector: introspector.track_with(&constraint),
        };

        document = (library.items.layout)(&mut vt, content, styles)?;
        iter += 1;

        introspector = Introspector::new(&document.pages);
        introspector.init = true;

        if iter >= 5 || introspector.valid(&constraint) {
            break;
        }
    }

    Ok(document)
}

/// A virtual typesetter.
///
/// Holds the state needed to [typeset] content.
pub struct Vt<'a> {
    /// The compilation environment.
    pub world: Tracked<'a, dyn World>,
    /// The tracer for inspection of the values an expression produces.
    pub tracer: TrackedMut<'a, Tracer>,
    /// Provides stable identities to nodes.
    pub provider: TrackedMut<'a, StabilityProvider>,
    /// Provides access to information about the document.
    pub introspector: Tracked<'a, Introspector>,
}

impl Vt<'_> {
    /// Mutably reborrow with a shorter lifetime.
    pub fn reborrow_mut(&mut self) -> Vt<'_> {
        Vt {
            world: self.world,
            tracer: TrackedMut::reborrow_mut(&mut self.tracer),
            provider: TrackedMut::reborrow_mut(&mut self.provider),
            introspector: self.introspector,
        }
    }
}

/// Provides stable identities to nodes.
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
    pub fn identify(&mut self, hash: u128) -> StableId {
        let count = self.hashes.iter().filter(|&&prev| prev == hash).count();
        self.hashes.push(hash);
        StableId(hash, count, 0)
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

/// Stably identifies a call site across multiple layout passes.
///
/// This struct is created by [`StabilityProvider::identify`].
#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub struct StableId(u128, usize, usize);

impl StableId {
    /// Produce a variant of this id.
    pub fn variant(self, n: usize) -> Self {
        Self(self.0, self.1, n)
    }
}

impl Debug for StableId {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.pad("..")
    }
}

cast_from_value! {
    StableId: "stable id",
}

/// Provides access to information about the document.
pub struct Introspector {
    init: bool,
    pages: usize,
    nodes: Vec<(Content, Location)>,
}

impl Introspector {
    /// Create a new introspector.
    pub fn new(frames: &[Frame]) -> Self {
        let mut introspector = Self { init: false, pages: frames.len(), nodes: vec![] };
        for (i, frame) in frames.iter().enumerate() {
            let page = NonZeroUsize::new(1 + i).unwrap();
            introspector.extract(frame, page, Transform::identity());
        }
        introspector
    }

    /// Iterate over all nodes.
    pub fn all(&self) -> impl Iterator<Item = &Content> {
        self.nodes.iter().map(|(node, _)| node)
    }

    /// Extract metadata from a frame.
    fn extract(&mut self, frame: &Frame, page: NonZeroUsize, ts: Transform) {
        for (pos, element) in frame.elements() {
            match element {
                Element::Group(group) => {
                    let ts = ts
                        .pre_concat(Transform::translate(pos.x, pos.y))
                        .pre_concat(group.transform);
                    self.extract(&group.frame, page, ts);
                }
                Element::Meta(Meta::Node(content), _)
                    if !self
                        .nodes
                        .iter()
                        .any(|(prev, _)| prev.stable_id() == content.stable_id()) =>
                {
                    let pos = pos.transform(ts);
                    self.nodes.push((content.clone(), Location { page, pos }));
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
        self.init
    }

    /// Query for all nodes for the given selector.
    pub fn query(&self, selector: Selector) -> Vec<Content> {
        self.all().filter(|node| selector.matches(node)).cloned().collect()
    }

    /// Query for all nodes up to the given id.
    pub fn query_before(&self, selector: Selector, id: StableId) -> Vec<Content> {
        let mut matches = vec![];
        for node in self.all() {
            if selector.matches(node) {
                matches.push(node.clone());
            }
            if node.stable_id() == Some(id) {
                break;
            }
        }
        matches
    }

    /// Query for all nodes starting from the given id.
    pub fn query_after(&self, selector: Selector, id: StableId) -> Vec<Content> {
        self.all()
            .skip_while(|node| node.stable_id() != Some(id))
            .filter(|node| selector.matches(node))
            .cloned()
            .collect()
    }

    /// The total number pages.
    pub fn pages(&self) -> NonZeroUsize {
        NonZeroUsize::new(self.pages).unwrap_or(NonZeroUsize::ONE)
    }

    /// Find the page number for the given stable id.
    pub fn page(&self, id: StableId) -> NonZeroUsize {
        self.location(id).page
    }

    /// Find the location for the given stable id.
    pub fn location(&self, id: StableId) -> Location {
        self.nodes
            .iter()
            .find(|(node, _)| node.stable_id() == Some(id))
            .map(|(_, loc)| *loc)
            .unwrap_or(Location { page: NonZeroUsize::ONE, pos: Point::zero() })
    }
}
