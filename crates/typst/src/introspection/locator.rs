use std::cell::RefCell;
use std::collections::HashMap;

use comemo::{Track, Tracked, Validate};

use crate::introspection::{Location, Meta};
use crate::layout::{Frame, FrameItem};

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
        self.hashes.get_mut().insert(hash, disambiguator + 1);

        // Create the location in its default variant.
        Location { hash, disambiguator, variant: 0 }
    }

    /// Advance past a frame.
    pub fn visit_frame(&mut self, frame: &Frame) {
        for (_, item) in frame.items() {
            match item {
                FrameItem::Group(group) => self.visit_frame(&group.frame),
                FrameItem::Meta(Meta::Elem(elem), _) => {
                    let hashes = self.hashes.get_mut();
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
