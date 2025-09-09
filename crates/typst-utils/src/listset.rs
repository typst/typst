use std::ops::DerefMut;

/// Picked by gut feeling. Could probably even be a bit larger.
const CUT_OFF: usize = 15;

/// A set backed by a mutable slice-like data structure.
///
/// This data structure uses two different strategies depending on size:
///
/// - When the list is small, it is just kept as is and searched linearly in
///   [`contains`](Self::contains).
///
/// - When the list is a bit bigger, it's sorted in [`new`](Self::new) and then
///   binary-searched for containment checks.
pub struct ListSet<S>(S);

impl<T, S> ListSet<S>
where
    S: DerefMut<Target = [T]>,
    T: Ord,
{
    /// Creates a new list set.
    ///
    /// If the list is longer than the cutoff point, it is sorted.
    pub fn new(mut list: S) -> Self {
        if list.len() > CUT_OFF {
            list.sort_unstable();
        }
        Self(list)
    }

    /// Whether the set contains no elements.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Checks whether the set contains the given value.
    ///
    /// If the list is shorter than the cutoff point, performs a linear search.
    /// If it is longer, performs a binary search.
    pub fn contains(&self, value: &T) -> bool {
        if self.0.len() > CUT_OFF {
            self.0.binary_search(value).is_ok()
        } else {
            self.0.contains(value)
        }
    }
}
