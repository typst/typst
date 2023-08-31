use crate::diag::{bail, StrResult};
use crate::eval::Value;
use ecow::EcoVec;
use serde::{Serialize, Serializer};
use std::cmp::Ordering;
use std::fmt::{Debug, Display, Formatter};
use std::hash::{Hash, Hasher};
use std::iter::repeat;
use typst::eval::Array;

/// A version, with any number of components.
///
/// The list of components is semantically extended by an infinite list of zeros
#[derive(Clone)]
pub struct Version(EcoVec<NonNegativeI64>);

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

    /// Construct a new version
    pub fn new(components: impl IntoIterator<Item = i64>) -> Option<Self> {
        components
            .into_iter()
            .map(NonNegativeI64::new)
            .collect::<Option<EcoVec<_>>>()
            .map(Self)
    }

    fn get(&self, index: usize) -> Option<NonNegativeI64> {
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

    /// Get an iterator of the values of all named version components for a version.
    pub fn named_components(&self) -> impl Iterator<Item = (&'static str, i64)> + '_ {
        Self::COMPONENT_NAMES
            .into_iter()
            .enumerate()
            .map(|(i, name)| (name, self.get(i).unwrap_or_default().get()))
    }

    /// Convert a version into an array
    pub fn into_array(self) -> Array {
        self.0.into_iter().map(|i| Value::Int(i.get())).collect()
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        let max_len = self.0.len().max(other.0.len());
        let tail = repeat(&NonNegativeI64::ZERO);

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
        write!(f, "version({})", self)
    }
}

impl Serialize for Version {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(serializer)
    }
}

#[derive(Default, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
#[repr(transparent)]
struct NonNegativeI64(u64);

#[allow(dead_code)] // reason: kept for completeness, might be used in the future
impl NonNegativeI64 {
    pub const ZERO: Self = Self(0);

    pub const MIN: Self = Self(0);
    pub const MAX: Self = Self(i64::MAX as u64);

    pub fn new(value: i64) -> Option<Self> {
        (value >= 0).then_some(Self(value as u64))
    }

    pub fn get(self) -> i64 {
        self.0 as i64
    }
}

impl Display for NonNegativeI64 {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl Debug for NonNegativeI64 {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        Debug::fmt(&self.0, f)
    }
}

impl Serialize for NonNegativeI64 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(serializer)
    }
}
