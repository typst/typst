use std::fmt::{self, Debug, Formatter};

use thin_vec::ThinVec;

/// The number of bits per chunk.
const BITS: usize = usize::BITS as usize;

/// Stores a set of numbers which are expected to be rather small.
///
/// Inserting a very small value is cheap while inserting a large one may be
/// very expensive.
///
/// Unless you're managing small numbers yourself, you should likely prefer
/// `SmallBitSet`, which has a bit larger memory size, but does not allocate
/// for small numbers.
#[derive(Clone, PartialEq, Hash)]
pub struct BitSet(ThinVec<usize>);

impl BitSet {
    /// Creates a new empty bit set.
    pub fn new() -> Self {
        Self(ThinVec::new())
    }

    /// Inserts a number into the set.
    pub fn insert(&mut self, value: usize) {
        let chunk = value / BITS;
        let within = value % BITS;
        if chunk >= self.0.len() {
            self.0.resize(chunk + 1, 0);
        }
        self.0[chunk] |= 1 << within;
    }

    /// Whether a number is present in the set.
    pub fn contains(&self, value: usize) -> bool {
        let chunk = value / BITS;
        let within = value % BITS;
        let Some(bits) = self.0.get(chunk) else { return false };
        (bits & (1 << within)) != 0
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
        let chunks = self.0.len();
        for v in 0..chunks * BITS {
            if self.contains(v) {
                list.entry(&v);
            }
        }
        list.finish()
    }
}

/// Efficiently stores a set of numbers which are expected to be very small.
/// Values `< 32/64` (depending on the architecture) are stored inline, while
/// values larger than that will lead to an allocation.
#[derive(Clone, PartialEq, Hash)]
pub struct SmallBitSet {
    /// Used to store values < BITS.
    low: usize,
    /// Used to store values > BITS.
    hi: BitSet,
}

impl SmallBitSet {
    /// Creates a new empty bit set.
    pub fn new() -> Self {
        Self { low: 0, hi: BitSet::new() }
    }

    /// Inserts a number into the set.
    pub fn insert(&mut self, value: usize) {
        if value < BITS {
            self.low |= 1 << value;
        } else {
            self.hi.insert(value - BITS);
        }
    }

    /// Whether a number is present in the set.
    pub fn contains(&self, value: usize) -> bool {
        if value < BITS {
            (self.low & (1 << value)) != 0
        } else {
            self.hi.contains(value - BITS)
        }
    }
}

impl Default for SmallBitSet {
    fn default() -> Self {
        Self::new()
    }
}

impl Debug for SmallBitSet {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let mut list = f.debug_list();
        let chunks = 1 + self.hi.0.len();
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
        let mut set = SmallBitSet::new();
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
