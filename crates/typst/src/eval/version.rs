use std::cmp::Ordering;
use std::fmt::{Debug, Display, Formatter};
use std::hash::{Hash, Hasher};
use std::iter::repeat;
use std::str::FromStr;

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
#[derive(Clone)]
pub struct Version(EcoVec<VersionComponent>);

// helper macro to make sure COMPONENT_NAMES and resolve_component_name never go out of sync
macro_rules! component_names {
    (@gen [$($arms:tt)*] [$($index:tt)*] [$($values:literal),*] $name:literal $(, $($rest:tt)*)?) => {
        component_names!(@gen [$($arms)* $name => $($index)*,] [$($index)* + 1] [$($values,)* $name] $($($rest)*)?);
    };
    (@gen [$($arms:tt)*] [$($index:tt)*] [$($values:literal),*] /* end */) => {
        // note: index is now the index after the end, i.e. the len
        pub const COMPONENT_NAMES: [&'static str; { $($index)* }] = [$($values),*];
        fn resolve_component_name(name: &str) -> Option<usize> {
            Some(match name {
                $($arms)*
                _ => return None
            })
        }
    };
    ($($input:tt)*) => {
        component_names!(@gen [] [0] [] $($input)*);
    };
}

impl Version {
    component_names!["major", "minor", "patch"];

    pub fn new() -> Self {
        Self(EcoVec::new())
    }

    fn get(&self, index: usize) -> Option<VersionComponent> {
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
            .unwrap_or_default()
            .get())
    }

    /// Get a named component of a version.
    ///
    /// Always non-negative. Returns `0` if the version isn't specified to the necessary length.
    pub fn component(&self, name: &str) -> StrResult<i64> {
        match Self::resolve_component_name(name) {
            Some(i) => Ok(self.get(i).unwrap_or_default().get()),
            None => bail!("unknown version component"),
        }
    }

    /// Convert a version into an array
    pub fn into_array(self) -> Array {
        self.0.into_iter().map(|i| Value::Int(i.get())).collect()
    }

    pub fn push(&mut self, component: VersionComponent) {
        self.0.push(component);
    }
}

impl FromIterator<VersionComponent> for Version {
    fn from_iter<T: IntoIterator<Item = VersionComponent>>(iter: T) -> Self {
        Self(EcoVec::from_iter(iter))
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        let max_len = self.0.len().max(other.0.len());
        let tail = repeat(&VersionComponent::ZERO);

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

impl Hash for Version {
    // Don't hash any explicitly suffixed zeros so that equal but non-identical versions (e.g. 1.2 and 1.2.0) hash as the same version
    fn hash<H: Hasher>(&self, state: &mut H) {
        let len = self
            .0
            .iter()
            .rposition(|x| x.get() != 0)
            .unwrap_or_else(|| self.0.len());

        self.0[..len].hash(state);
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

#[derive(Default, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
#[repr(transparent)]
pub struct VersionComponent(u64);

impl VersionComponent {
    pub const ZERO: Self = Self(0);

    pub fn new(value: i64) -> Option<Self> {
        (value >= 0).then_some(Self(value as u64))
    }

    pub fn get(self) -> i64 {
        self.0 as i64
    }
}

impl FromStr for VersionComponent {
    type Err = std::num::ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // ensure there is no negative sign
        let _ = u64::from_str(s)?;
        // actually parse as signed, to ensure the value is below the max
        let i: i64 = s.parse()?;

        Ok(Self(i as u64))
    }
}

impl Display for VersionComponent {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl Debug for VersionComponent {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        Debug::fmt(&self.0, f)
    }
}
