use std::fmt::{self, Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::ops::Deref;

use super::{Array, Value};
use crate::diag::StrResult;
use crate::util::EcoString;

/// Extra methods on strings.
pub trait StrExt {
    /// Repeat a string a number of times.
    fn repeat(&self, n: i64) -> StrResult<EcoString>;

    /// Split this string at whitespace or a specific pattern.
    fn split(&self, at: Option<EcoString>) -> Array;
}

impl StrExt for EcoString {
    fn repeat(&self, n: i64) -> StrResult<EcoString> {
        let n = usize::try_from(n)
            .ok()
            .and_then(|n| self.len().checked_mul(n).map(|_| n))
            .ok_or_else(|| format!("cannot repeat this string {} times", n))?;

        Ok(self.repeat(n))
    }

    fn split(&self, at: Option<EcoString>) -> Array {
        if let Some(pat) = at {
            self.as_str()
                .split(pat.as_str())
                .map(|s| Value::Str(s.into()))
                .collect()
        } else {
            self.as_str()
                .split_whitespace()
                .map(|s| Value::Str(s.into()))
                .collect()
        }
    }
}

/// A regular expression.
#[derive(Clone)]
pub struct Regex(regex::Regex);

impl Regex {
    /// Create a new regular expression.
    pub fn new(re: &str) -> StrResult<Self> {
        regex::Regex::new(re).map(Self).map_err(|err| err.to_string())
    }
}

impl Deref for Regex {
    type Target = regex::Regex;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Debug for Regex {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "regex({:?})", self.0.as_str())
    }
}

impl PartialEq for Regex {
    fn eq(&self, other: &Self) -> bool {
        self.0.as_str() == other.0.as_str()
    }
}

impl Hash for Regex {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.as_str().hash(state);
    }
}
