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
