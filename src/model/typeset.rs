use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::Hash;
use std::num::NonZeroUsize;

use comemo::{Track, Tracked, TrackedMut};

use super::{Content, Node, Selector, StyleChain};
use crate::diag::SourceResult;
use crate::doc::{Document, Element, Frame, Location, Meta};
use crate::geom::Transform;
use crate::util::hash128;
use crate::World;

/// Typeset content into a fully layouted document.
#[comemo::memoize]
pub fn typeset(world: Tracked<dyn World>, content: &Content) -> SourceResult<Document> {
    let library = world.library();
    let styles = StyleChain::new(&library.styles);

    let mut document;
    let mut iter = 0;
    let mut introspector = Introspector::new(&[]);

    // Relayout until all introspections stabilize.
    // If that doesn't happen within five attempts, we give up.
    loop {
        let mut provider = StabilityProvider::new();
        let mut vt = Vt {
            world,
            provider: provider.track_mut(),
            introspector: introspector.track(),
        };

        document = (library.items.layout)(&mut vt, content, styles)?;
        iter += 1;

        if iter >= 5 || introspector.update(&document.pages) {
            break;
        }
    }

    Ok(document)
}

/// A virtual typesetter.
///
/// Holds the state needed to [typeset] content. This is the equivalent to the
/// [Vm](crate::eval::Vm) for typesetting.
pub struct Vt<'a> {
    /// The compilation environment.
    pub world: Tracked<'a, dyn World>,
    /// Provides stable identities to nodes.
    pub provider: TrackedMut<'a, StabilityProvider>,
    /// Provides access to information about the document.
    pub introspector: Tracked<'a, Introspector>,
}

impl<'a> Vt<'a> {
    /// Access the underlying world.
    pub fn world(&self) -> Tracked<'a, dyn World> {
        self.world
    }

    /// Produce a stable identifier for this call site.
    ///
    /// The key should be something that identifies the call site, but is not
    /// necessarily unique. The stable marker incorporates the key's hash plus
    /// additional disambiguation from other call sites with the same key.
    ///
    /// The returned id can be attached to content as metadata is the then
    /// locatable through [`locate`](Self::locate).
    pub fn identify<T: Hash>(&mut self, key: &T) -> StableId {
        self.provider.identify(hash128(key))
    }

    /// Whether things are locatable already.
    pub fn locatable(&self) -> bool {
        self.introspector.init()
    }

    /// Locate all metadata matches for the given node.
    pub fn query_node<T: Node>(&self) -> impl Iterator<Item = &T> {
        self.introspector
            .query(Selector::node::<T>())
            .into_iter()
            .map(|content| content.to::<T>().unwrap())
    }
}

/// Stably identifies a call site across multiple layout passes.
///
/// This struct is created by [`Vt::identify`].
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct StableId(u128, u64, u64);

impl StableId {
    /// Produce a variant of this id.
    pub fn variant(self, n: u64) -> Self {
        Self(self.0, self.1, n)
    }
}

/// Provides stable identities to nodes.
#[derive(Clone)]
pub struct StabilityProvider(HashMap<u128, u64>);

impl StabilityProvider {
    /// Create a new stability provider.
    fn new() -> Self {
        Self(HashMap::new())
    }
}

#[comemo::track]
impl StabilityProvider {
    /// Produce a stable identifier for this call site.
    fn identify(&mut self, hash: u128) -> StableId {
        let slot = self.0.entry(hash).or_default();
        let id = StableId(hash, *slot, 0);
        *slot += 1;
        id
    }
}

/// Provides access to information about the document.
pub struct Introspector {
    init: bool,
    nodes: Vec<(Content, Location)>,
    queries: RefCell<Vec<(Selector, u128)>>,
}

impl Introspector {
    /// Create a new introspector.
    pub fn new(frames: &[Frame]) -> Self {
        let mut introspector = Self {
            init: false,
            nodes: vec![],
            queries: RefCell::new(vec![]),
        };
        introspector.extract_from_frames(frames);
        introspector
    }

    /// Update the information given new frames and return whether we can stop
    /// layouting.
    pub fn update(&mut self, frames: &[Frame]) -> bool {
        self.nodes.clear();
        self.extract_from_frames(frames);

        let was_init = std::mem::replace(&mut self.init, true);
        let queries = std::mem::take(&mut self.queries).into_inner();

        for (selector, hash) in &queries {
            let nodes = self.query_impl(selector);
            if hash128(&nodes) != *hash {
                return false;
            }
        }

        if !was_init && !queries.is_empty() {
            return false;
        }

        true
    }

    /// Iterate over all nodes.
    pub fn nodes(&self) -> impl Iterator<Item = &Content> {
        self.nodes.iter().map(|(node, _)| node)
    }

    /// Extract metadata from frames.
    fn extract_from_frames(&mut self, frames: &[Frame]) {
        for (i, frame) in frames.iter().enumerate() {
            let page = NonZeroUsize::new(1 + i).unwrap();
            self.extract_from_frame(frame, page, Transform::identity());
        }
    }

    /// Extract metadata from a frame.
    fn extract_from_frame(&mut self, frame: &Frame, page: NonZeroUsize, ts: Transform) {
        for (pos, element) in frame.elements() {
            match element {
                Element::Group(group) => {
                    let ts = ts
                        .pre_concat(Transform::translate(pos.x, pos.y))
                        .pre_concat(group.transform);
                    self.extract_from_frame(&group.frame, page, ts);
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

    /// Query for all metadata matches for the given selector.
    pub fn query(&self, selector: Selector) -> Vec<&Content> {
        let nodes = self.query_impl(&selector);
        let mut queries = self.queries.borrow_mut();
        if !queries.iter().any(|(prev, _)| prev == &selector) {
            queries.push((selector, hash128(&nodes)));
        }
        nodes
    }

    /// Find the page number for the given stable id.
    pub fn page(&self, id: StableId) -> Option<NonZeroUsize> {
        Some(self.location(id)?.page)
    }

    /// Find the location for the given stable id.
    pub fn location(&self, id: StableId) -> Option<Location> {
        Some(self.nodes.iter().find(|(node, _)| node.stable_id() == Some(id))?.1)
    }
}

impl Introspector {
    fn query_impl(&self, selector: &Selector) -> Vec<&Content> {
        self.nodes().filter(|node| selector.matches(node)).collect()
    }
}
