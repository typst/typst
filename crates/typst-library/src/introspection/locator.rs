use std::collections::HashMap;
use std::fmt::{self, Debug, Formatter};
use std::hash::Hash;
use std::sync::OnceLock;

use comemo::{Tracked, Validate};

use crate::introspection::{Introspector, Location};

/// Provides locations for elements in the document.
///
/// A [`Location`] is a unique ID for an element generated during realization.
///
/// # How to use this
/// The same content may yield different results when laid out in different
/// parts of the document. To reflect this, every layout operation receives a
/// locator and every layout operation requires a locator. In code:
///
/// - all layouters receive an owned `Locator`
/// - all layout functions take an owned `Locator`
///
/// When a layouter only requires a single sublayout call, it can simply pass on
/// its locator. When a layouter needs to call multiple sublayouters, we need to
/// make an explicit decision:
///
/// - Split: When we're layouting multiple distinct children (or other pieces of
///   content), we need to split up the locator with [`Locator::split`]. This
///   allows us to produce multiple new `Locator`s for the sublayouts. When we
///   split the locator, each sublocator will be a distinct entity and using it
///   to e.g. layout the same piece of figure content will yield distinctly
///   numbered figures.
///
/// - Relayout: When we're layouting the same content multiple times (e.g. when
///   measuring something), we can call [`Locator::relayout`] to use the same
///   locator multiple times. This indicates to the compiler that it's actually
///   the same content. Using it to e.g. layout the same piece of figure content
///   will yield the same figure number both times. Typically, when we layout
///   something multiple times using `relayout`, only one of the outputs
///   actually ends up in the document, while the other outputs are only used
///   for measurement and then discarded.
///
/// The `Locator` intentionally does not implement `Copy` and `Clone` so that it
/// can only be used once. This ensures that whenever we are layouting multiple
/// things, we make an explicit decision whether we want to split or relayout.
///
/// # How it works
/// There are two primary considerations for the assignment of locations:
///
/// 1. Locations should match up over multiple layout iterations, so that
///    elements can be identified as being the same: That's the whole point of
///    them.
///
/// 2. Locations should be as stable as possible across document edits, so that
///    incremental compilation is effective.
///
/// 3. We want to assign them with as little long-lived state as possible to
///    enable parallelization of the layout process.
///
/// Let's look at a few different assignment strategies to get a feeling for
/// these requirements:
///
/// - A very simple way to generate unique IDs would be to just increase a
///   counter for each element. In this setup, (1) is somewhat satisfied: In
///   principle, the counter will line up across iterations, but things start to
///   break down once we generate content dependent on introspection since the
///   IDs generated for that new content will shift the IDs for all following
///   elements in the document. (2) is not satisfied since an edit in the middle
///   of the document shifts all later IDs. (3) is obviously not satisfied.
///   Conclusion: Not great.
///
/// - To make things more robust, we can incorporate some stable knowledge about
///   the element into the ID. For this, we can use the element's span since it
///   is already mostly unique: Elements resulting from different source code
///   locations are guaranteed to have different spans. However, we can also
///   have multiple distinct elements generated from the same source location:
///   e.g. `#for _ in range(5) { figure(..) }`. To handle this case, we can then
///   disambiguate elements with the same span with an increasing counter. In
///   this setup, (1) is mostly satisfied: Unless we do stuff like generating
///   colliding counter updates dependent on introspection, things will line up.
///   (2) is also reasonably well satisfied, as typical edits will only affect
///   the single element at the currently edited span. Only if we edit inside of
///   a function, loop, or similar construct, we will affect multiple elements.
///   (3) is still a problem though, since we count up.
///
/// - What's left is to get rid of the mutable state. Note that layout is a
///   recursive process and has a tree-shaped execution graph. Thus, we can try
///   to determine an element's ID based on the path of execution taken in this
///   graph. Something like "3rd element in layer 1, 7th element in layer 2,
///   ..". This is basically the first approach, but on a per-layer basis. Thus,
///   we can again apply our trick from the second approach, and use the span +
///   disambiguation strategy on a per-layer basis: "1st element with span X in
///   layer 1, 3rd element with span Y in layer 2". The chance for a collision
///   is now pretty low and our state is wholly local to each level. So, if we
///   want to parallelize layout within a layer, we can generate the IDs for
///   that layer upfront and then start forking out. The final remaining
///   question is how we can compactly encode this information: For this, as
///   always, we use hashing! We incorporate the ID information from each layer
///   into a single hash and thanks to the collision resistance of 128-bit
///   SipHash, we get almost guaranteed unique locations. We don't even store
///   the full layer information at all, but rather hash _hierarchically:_ Let
///   `k_x` be our local per-layer ID for layer `x` and `h_x` be the full
///   combined hash for layer `x`. We compute `h_n = hash(h_(n-1), k_n)`.
///
/// So that's what's going on conceptually in this type. For efficient
/// memoization, we do all of this in a tracked fashion, such that we only
/// observe the hash for all the layers above us, if we actually need to
/// generate a [`Location`]. Thus, if we have a piece of content that does not
/// contain any locatable elements, we can cache its layout even if it occurs in
/// different places.
///
/// # Dealing with measurement
/// As explained above, any kind of measurement the compiler performs requires a
/// locator that matches the one used during real layout. This ensures that the
/// locations assigned during measurement match up exactly with the locations of
/// real document elements. Without this guarantee, many introspection-driven
/// features (like counters, state, and citations) don't work correctly (since
/// they perform queries dependent on concrete locations).
///
/// This is all fine and good, but things get really tricky when the _user_
/// measures such introspecting content since the user isn't kindly managing
/// locators for us. Our standard `Locator` workflow assigns locations that
/// depend a lot on the exact placement in the hierarchy of elements. For this
/// reason, something that is measured, but then placed into something like a
/// grid will get a location influenced by the grid. Without a locator, we can't
/// make the connection between the measured content and the real content, so we
/// can't ensure that the locations match up.
///
/// One possible way to deal with this is to force the user to uniquely identify
/// content before being measured after all. This would mean that the user needs
/// to come up with an identifier that is unique within the surrounding context
/// block and attach it to the content in some way. However, after careful
/// consideration, I have concluded that this is simply too big of an ask from
/// users: Understanding why this is even necessary is pretty complicated and
/// how to best come up with a unique ID is even more so.
///
/// For this reason, I chose an alternative best-effort approach: The locator
/// has a custom "measurement mode" (entered through [`LocatorLink::measure`]),
/// in which it does its best to assign locations that match up. Specifically,
/// it uses the key hashes of the individual locatable elements in the measured
/// content (which may not be unique if content is reused) and combines them
/// with the context's location to find the most likely matching real element.
/// This approach works correctly almost all of the time (especially for
/// "normal" hand-written content where the key hashes rarely collide, as
/// opposed to code-heavy things where they do).
///
/// Support for enhancing this with user-provided uniqueness can still be added
/// in the future. It will most likely anyway be added simply because it's
/// automatically included when we add a way to "freeze" content for things like
/// slidehows. But it will be opt-in because it's just too much complication.
pub struct Locator<'a> {
    /// A local hash that incorporates all layers since the last memoization
    /// boundary.
    local: u128,
    /// A pointer to an outer cached locator, which contributes the information
    /// for all the layers beyond the memoization boundary on-demand.
    outer: Option<&'a LocatorLink<'a>>,
}

impl<'a> Locator<'a> {
    /// Create a new root-level locator.
    ///
    /// Should typically only be created at the document level, though there
    /// are a few places where we use it as well that just don't support
    /// introspection (e.g. tilings).
    pub fn root() -> Self {
        Self { local: 0, outer: None }
    }

    /// Creates a new synthetic locator.
    ///
    /// This can be used to create a new dependent layout based on an element.
    /// This is used for layouting footnote entries based on the location
    /// of the associated footnote.
    pub fn synthesize(location: Location) -> Self {
        Self { local: location.hash(), outer: None }
    }

    /// Creates a new locator that points to the given link.
    pub fn link(link: &'a LocatorLink<'a>) -> Self {
        Self { local: 0, outer: Some(link) }
    }
}

impl<'a> Locator<'a> {
    /// Returns a type that can be used to generate `Locator`s for multiple
    /// child elements. See the type-level docs for more details.
    pub fn split(self) -> SplitLocator<'a> {
        SplitLocator {
            local: self.local,
            outer: self.outer,
            disambiguators: HashMap::new(),
        }
    }

    /// Creates a copy of this locator for measurement or relayout of the same
    /// content. See the type-level docs for more details.
    ///
    /// This is effectively just `Clone`, but the `Locator` doesn't implement
    /// `Clone` to make this operation explicit.
    pub fn relayout(&self) -> Self {
        Self { local: self.local, outer: self.outer }
    }
}

#[comemo::track]
#[allow(clippy::needless_lifetimes)]
impl<'a> Locator<'a> {
    /// Resolves the locator based on its local and the outer information.
    fn resolve(&self) -> Resolved {
        match self.outer {
            None => Resolved::Hash(self.local),
            Some(outer) => match outer.resolve() {
                Resolved::Hash(outer) => {
                    Resolved::Hash(typst_utils::hash128(&(self.local, outer)))
                }
                Resolved::Measure(anchor) => Resolved::Measure(anchor),
            },
        }
    }
}

impl Debug for Locator<'_> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "Locator({:?})", self.resolve())
    }
}

/// The fully resolved value of a locator.
#[derive(Debug, Copy, Clone, Hash)]
enum Resolved {
    /// The full hash, incorporating the local and all outer information.
    Hash(u128),
    /// Indicates that the locator is in measurement mode, with the given anchor
    /// location.
    Measure(Location),
}

/// A type that generates unique sublocators.
#[derive(Clone)]
pub struct SplitLocator<'a> {
    /// A local hash that incorporates all layers since the last memoization
    /// boundary.
    local: u128,
    /// A pointer to an outer cached locator, which contributes the information
    /// for all the layers beyond the memoization boundary on-demand.
    outer: Option<&'a LocatorLink<'a>>,
    /// Simply counts up the number of times we've seen each local hash.
    disambiguators: HashMap<u128, usize>,
}

impl<'a> SplitLocator<'a> {
    /// Produces a sublocator for a subtree keyed by `key`. The keys do *not*
    /// need to be unique among the `next()` calls on this split locator. (They
    /// can even all be `&()`.)
    ///
    /// However, stable & mostly unique keys lead to more stable locations
    /// throughout edits, improving incremental compilation performance.
    ///
    /// A common choice for a key is the span of the content that will be
    /// layouted with this locator.
    pub fn next<K: Hash>(&mut self, key: &K) -> Locator<'a> {
        self.next_inner(typst_utils::hash128(key))
    }

    /// Produces a sublocator for a subtree.
    pub fn next_inner(&mut self, key: u128) -> Locator<'a> {
        // Produce a locator disambiguator, for elements with the same key
        // within this `SplitLocator`.
        let disambiguator = {
            let slot = self.disambiguators.entry(key).or_default();
            std::mem::replace(slot, *slot + 1)
        };

        // Combine the key, disambiguator and local hash into a sub-local hash.
        // The outer information is not yet merged into this, it is added
        // on-demand in `Locator::resolve`.
        let local = typst_utils::hash128(&(key, disambiguator, self.local));

        Locator { outer: self.outer, local }
    }

    /// Produces a unique location for an element.
    pub fn next_location(
        &mut self,
        introspector: Tracked<Introspector>,
        key: u128,
    ) -> Location {
        match self.next_inner(key).resolve() {
            Resolved::Hash(hash) => Location::new(hash),
            Resolved::Measure(anchor) => {
                // If we aren't able to find a matching element in the document,
                // default to the anchor, so that it's at least remotely in
                // the right area (so that counters can be resolved).
                introspector.locator(key, anchor).unwrap_or(anchor)
            }
        }
    }
}

/// A locator can be linked to this type to only access information across the
/// memoization boundary on-demand, improving the cache hit chance.
pub struct LocatorLink<'a> {
    /// The link itself.
    kind: LinkKind<'a>,
    /// The cached resolved link.
    resolved: OnceLock<Resolved>,
}

/// The different kinds of locator links.
enum LinkKind<'a> {
    /// An outer `Locator`, which we can resolved if necessary.
    ///
    /// We need to override the constraint's lifetime here so that `Tracked` is
    /// covariant over the constraint. If it becomes invariant, we're in for a
    /// world of lifetime pain.
    Outer(Tracked<'a, Locator<'a>, <Locator<'static> as Validate>::Constraint>),
    /// A link which indicates that we are in measurement mode.
    Measure(Location),
}

impl<'a> LocatorLink<'a> {
    /// Create a locator link.
    pub fn new(outer: Tracked<'a, Locator<'a>>) -> Self {
        LocatorLink {
            kind: LinkKind::Outer(outer),
            resolved: OnceLock::new(),
        }
    }

    /// Creates a link that puts any linked downstream locator into measurement
    /// mode.
    ///
    /// Read the "Dealing with measurement" section of the [`Locator`] docs for
    /// more details.
    pub fn measure(anchor: Location) -> Self {
        LocatorLink {
            kind: LinkKind::Measure(anchor),
            resolved: OnceLock::new(),
        }
    }

    /// Resolve the link.
    ///
    /// The result is cached in this link, so that we don't traverse the link
    /// chain over and over again.
    fn resolve(&self) -> Resolved {
        *self.resolved.get_or_init(|| match self.kind {
            LinkKind::Outer(outer) => outer.resolve(),
            LinkKind::Measure(anchor) => Resolved::Measure(anchor),
        })
    }
}
