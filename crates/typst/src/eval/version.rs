use std::cmp::Ordering;
use std::fmt::{self, Display, Formatter, Write};
use std::hash::Hash;
use std::iter::repeat;

use ecow::{eco_format, EcoString, EcoVec};

use super::{cast, func, scope, ty, Repr};
use crate::diag::{bail, error, StrResult};
use crate::util::pretty_array_like;

/// A version, with any number of components.
///
/// The list of components is semantically extended by an infinite list of
/// zeros. This means that, for example, `0.8` is the same as `0.8.0`. As a
/// special case, the empty version (that has no components at all) is the same
/// as `0`, `0.0`, `0.0.0`, and so on.
///
/// The first three components have names: `major`, `minor`, `patch`. All
/// components after that do not have names.
#[ty(scope)]
#[derive(Debug, Default, Clone, Hash)]
#[allow(clippy::derived_hash_with_manual_eq)]
pub struct Version(EcoVec<u32>);

impl Version {
    /// The names for the first components of a version.
    pub const COMPONENTS: [&'static str; 3] = ["major", "minor", "patch"];

    /// Create a new (empty) version.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get a named component of a version.
    ///
    /// Always non-negative. Returns `0` if the version isn't specified to the
    /// necessary length.
    pub fn component(&self, name: &str) -> StrResult<i64> {
        self.0
            .iter()
            .zip(Self::COMPONENTS)
            .find_map(|(&i, s)| (s == name).then_some(i as i64))
            .ok_or_else(|| error!("unknown version component"))
    }

    /// Push a component to the end of this version.
    pub fn push(&mut self, component: u32) {
        self.0.push(component);
    }

    /// The values of the version
    pub fn values(&self) -> &[u32] {
        &self.0
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
        let mut version = Version::new();
        for c in components {
            match c {
                VersionComponents::Single(v) => version.push(v),
                VersionComponents::Multiple(values) => {
                    for v in values {
                        version.push(v);
                    }
                }
            }
        }
        version
    }

    /// Get a component of a version.
    ///
    /// Always non-negative. Returns `0` if the version isn't specified to the
    /// necessary length.
    #[func]
    pub fn at(
        &self,
        /// The index at which to retrieve the component. If negative, indexes
        /// from the back of the explicitly given components.
        index: i64,
    ) -> StrResult<i64> {
        let mut index = index;
        if index < 0 {
            match (self.0.len() as i64).checked_add(index) {
                Some(pos_index) if pos_index >= 0 => index = pos_index,
                _ => bail!(
                    "component index out of bounds (index: {index}, len: {})",
                    self.0.len()
                ),
            }
        }
        Ok(usize::try_from(index)
            .ok()
            .and_then(|i| self.0.get(i).copied())
            .unwrap_or_default() as i64)
    }
}

impl FromIterator<u32> for Version {
    fn from_iter<T: IntoIterator<Item = u32>>(iter: T) -> Self {
        Self(EcoVec::from_iter(iter))
    }
}

impl IntoIterator for Version {
    type Item = u32;
    type IntoIter = ecow::vec::IntoIter<u32>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
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
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let mut first = true;
        for &v in &self.0 {
            if !first {
                f.write_char('.')?;
            }
            write!(f, "{v}")?;
            first = false;
        }
        Ok(())
    }
}

impl Repr for Version {
    fn repr(&self) -> EcoString {
        let parts: Vec<_> = self.0.iter().map(|v| eco_format!("{v}")).collect();
        eco_format!("version{}", &pretty_array_like(&parts, false))
    }
}

/// One or multiple version components.
pub enum VersionComponents {
    Single(u32),
    Multiple(Vec<u32>),
}

cast! {
    VersionComponents,
    v: u32 => Self::Single(v),
    v: Vec<u32> => Self::Multiple(v)
}
