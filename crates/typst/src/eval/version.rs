use std::cmp::Ordering;
use std::fmt::{Debug, Display, Formatter};
use std::hash::Hash;
use std::iter::repeat;

use ecow::EcoVec;

use super::{Array, Value};
use crate::diag::{bail, StrResult};

/// A version, with any number of components.
///
/// The list of components is semantically extended
/// by an infinite list of zeros.
/// This means that, for example, `0.8` is the same as `0.8.0`.
/// As a special case, the empty version (that has no components at all)
/// is the same as `0`, `0.0`, `0.0.0`, and so on.
// reason: hash is for incremental compilation, so it needs to be different
// for values that display differently.
// It being different from `Eq` is consistent with many other typst types.
#[allow(clippy::derived_hash_with_manual_eq)]
#[derive(Default, Clone, Hash)]
pub struct Version(EcoVec<u32>);

impl Version {
    pub const COMPONENT_NAMES: [&'static str; 3] = ["major", "minor", "patch"];

    pub fn new() -> Self {
        Self::default()
    }

    fn get(&self, index: usize) -> Option<u32> {
        self.0.get(index).copied()
    }

    /// Get a component of a version.
    ///
    /// Always non-negative. Returns `0` if the version isn't specified to the necessary length.
    pub fn at(&self, index: i64) -> StrResult<i64> {
        if index < 0 {
            bail!("version component index out of bounds ({index})");
        }
        Ok(usize::try_from(index)
            .ok()
            .and_then(|i| self.get(i))
            .unwrap_or_default() as i64)
    }

    /// Get a named component of a version.
    ///
    /// Always non-negative. Returns `0` if the version isn't specified to the necessary length.
    pub fn component(&self, name: &str) -> StrResult<i64> {
        match Self::COMPONENT_NAMES.iter().position(|&s| name == s) {
            Some(i) => Ok(self.get(i).unwrap_or_default() as i64),
            None => bail!("unknown version component"),
        }
    }

    /// Convert a version into an array
    pub fn into_array(self) -> Array {
        self.0.into_iter().map(|i| Value::Int(i as i64)).collect()
    }

    pub fn push(&mut self, component: u32) {
        self.0.push(component);
    }
}

impl FromIterator<u32> for Version {
    fn from_iter<T: IntoIterator<Item = u32>>(iter: T) -> Self {
        Self(EcoVec::from_iter(iter))
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        let max_len = self.0.len().max(other.0.len());
        let tail = repeat(&0);

        let self_iter = self.0.iter().chain(tail.clone());
        let other_iter = other.0.iter().chain(tail);

        for (l, r) in self_iter.zip(other_iter).take(max_len) {
            match l.cmp(r) {
                Ordering::Equal => (),
                ord => return ord,
            }
        }
        Ordering::Equal
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Eq for Version {}

impl PartialEq for Version {
    fn eq(&self, other: &Self) -> bool {
        matches!(self.cmp(other), Ordering::Equal)
    }
}

impl Display for Version {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        let mut first = true;
        for &i in &self.0 {
            if first {
                first = false;
            } else {
                write!(f, ".")?;
            }
            write!(f, "{i}")?;
        }
        Ok(())
    }
}

impl Debug for Version {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "version({self})")
    }
}
