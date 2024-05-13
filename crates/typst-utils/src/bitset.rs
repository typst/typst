use std::fmt::{self, Debug, Formatter};

/// Efficiently stores a set of numbers which are expected to be very small
/// (< 32/64 depending on the architecture).
///
/// Inserting a very small value is very cheap while inserting a large one may
/// be very expensive.
#[derive(Clone, PartialEq, Hash)]
pub struct BitSet {
    /// Used to store values < BITS.
    low: usize,
    /// Used to store values > BITS. We have the extra `Box` to keep the memory
    /// size of the `BitSet` down.
    #[allow(clippy::box_collection)]
    hi: Option<Box<Vec<usize>>>,
}

/// The number of bits per chunk.
const BITS: usize = usize::BITS as usize;

impl BitSet {
    /// Creates a new empty bit set.
    pub fn new() -> Self {
        Self { low: 0, hi: None }
    }

    /// Inserts a number into the set.
    pub fn insert(&mut self, value: usize) {
        if value < BITS {
            self.low |= 1 << value;
        } else {
            let chunk = value / BITS - 1;
            let within = value % BITS;
            let vec = self.hi.get_or_insert_with(Default::default);
            if chunk >= vec.len() {
                vec.resize(chunk + 1, 0);
            }
            vec[chunk] |= 1 << within;
        }
    }

    /// Whether a number is present in the set.
    pub fn contains(&self, value: usize) -> bool {
        if value < BITS {
            (self.low & (1 << value)) != 0
        } else {
            let Some(hi) = &self.hi else { return false };
            let chunk = value / BITS - 1;
            let within = value % BITS;
            let Some(bits) = hi.get(chunk) else { return false };
            (bits & (1 << within)) != 0
        }
    }
}

impl Default for BitSet {
    fn default() -> Self {
        Self::new()
    }
}

impl Debug for BitSet {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let mut list = f.debug_list();
        let chunks = 1 + self.hi.as_ref().map_or(0, |v| v.len());
        for v in 0..chunks * BITS {
            if self.contains(v) {
                list.entry(&v);
            }
        }
        list.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bitset() {
        let mut set = BitSet::new();
        assert!(!set.contains(0));
        assert!(!set.contains(5));
        set.insert(0);
        set.insert(1);
        set.insert(5);
        set.insert(64);
        set.insert(105);
        set.insert(208);
        assert!(set.contains(0));
        assert!(set.contains(1));
        assert!(!set.contains(2));
        assert!(set.contains(5));
        assert!(!set.contains(63));
        assert!(set.contains(64));
        assert!(!set.contains(65));
        assert!(!set.contains(104));
        assert!(set.contains(105));
        assert!(!set.contains(106));
        assert!(set.contains(208));
        assert!(!set.contains(209));
        assert_eq!(format!("{set:?}"), "[0, 1, 5, 64, 105, 208]");
    }
}
