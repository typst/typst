use std::cmp::Ordering;
use std::fmt::{Debug, Display, Formatter};
use std::hash::Hash;
use std::iter::repeat;

use ecow::EcoVec;

use super::{Array, Value, func, scope, ty, cast};
use crate::diag::{bail, error, StrResult};

/// A version, with any number of components.
///
/// The list of components is semantically extended
/// by an infinite list of zeros.
/// This means that, for example, `0.8` is the same as `0.8.0`.
/// As a special case, the empty version (that has no components at all)
/// is the same as `0`, `0.0`, `0.0.0`, and so on.
///
/// The first three components have names: `major`, `minor`, `patch`.
/// All components after that do not have names.
// reason: hash is for incremental compilation, so it needs to be different
// for values that display differently.
// It being different from `Eq` is consistent with many other typst types.
#[allow(clippy::derived_hash_with_manual_eq)]
#[ty(scope)]
#[derive(Default, Clone, Hash)]
pub struct Version(EcoVec<u32>);

impl Version {
    /// The names for the first components of a version.
    pub const COMPONENT_NAMES: [&'static str; 3] = ["major", "minor", "patch"];

    /// Create a new (empty) version.
    pub fn new() -> Self {
        Self::default()
    }

    fn get(&self, index: usize) -> Option<u32> {
        self.0.get(index).copied()
    }

    /// Get a named component of a version.
    ///
    /// Always non-negative. Returns `0` if the version isn't specified to the
    /// necessary length.
    pub fn component(&self, name: &str) -> StrResult<i64> {
        self.0
            .iter()
            .zip(Self::COMPONENT_NAMES)
            .find_map(|(&i, s)| (s == name).then_some(i as i64))
            .ok_or_else(|| error!("unknown version component"))
    }

    /// Convert a version into an array
    pub fn into_array(self) -> Array {
        self.0.into_iter().map(|i| Value::Int(i as i64)).collect()
    }

    /// Push a component to the end of this version.
    pub fn push(&mut self, component: u32) {
        self.0.push(component);
    }
}

#[scope]
impl Version {
    /// Creates a new version.
    ///
    /// It can have any number of components (even zero).
    ///
    /// ```example
    /// #version() \
    /// #version(1) \
    /// #version(1, 2, 3, 4) \
    /// #version((1, 2, 3, 4)) \
    /// #version((1, 2), 3)
    /// ```
    #[func(constructor)]
    pub fn construct(
        /// The components of the version (array arguments are flattened)
        #[variadic]
        components: Vec<VersionComponents>,
    ) -> Version {
        let mut res = Version::new();

        for c in components {
            match c {
                VersionComponents::Single(i) => res.push(i),
                VersionComponents::Multiple(v) => {
                    for i in v {
                        res.push(i);
                    }
                }
            }
        }

        res
    }
    
    /// Get a component of a version.
    ///
    /// Always non-negative. Returns `0` if the version isn't specified to the
    /// necessary length.
    #[func]
    pub fn at(
        &self,
        /// The index at which to retrieve the component.
        /// If negative, indexes from the back of the explicitly given components.
        index: i64
    ) -> StrResult<i64> {
        let mut index = index;
        if index < 0 {
            match (self.0.len() as i64).checked_add(index) {
                Some(pos_index) if pos_index >= 0 => index = pos_index,
                _ => bail!("version component index out of bounds (index: {index}, len: {})", self.0.len()),
            }
        }
        Ok(usize::try_from(index)
            .ok()
            .and_then(|i| self.get(i))
            .unwrap_or_default() as i64)
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

/// One or multiple version components
pub enum VersionComponents {
    Single(u32),
    Multiple(Vec<u32>),
}

cast! {
    VersionComponents,
    i: u32 => Self::Single(i),
    arr: Vec<u32> => Self::Multiple(arr)
}