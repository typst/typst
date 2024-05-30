use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt::{self, Debug, Formatter};
use std::sync::RwLock;

use once_cell::sync::Lazy;

/// The global string interner.
static INTERNER: Lazy<RwLock<Interner>> =
    Lazy::new(|| RwLock::new(Interner { to_id: HashMap::new(), from_id: Vec::new() }));

/// A string interner.
struct Interner {
    to_id: HashMap<&'static str, PicoStr>,
    from_id: Vec<&'static str>,
}

/// An interned string.
///
/// The API is purposefully kept small. This is because it might be relatively
/// slow to look up a string in the interner, so we want to avoid doing it
/// unnecessarily. For this reason, the user should use the [`PicoStr::resolve`]
/// method to get the underlying string, such that the lookup is done only once.
#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub struct PicoStr(u32);

impl PicoStr {
    /// Creates a new interned string.
    pub fn new(string: &str) -> Self {
        // Try to find an existing entry that we can reuse.
        //
        // We could check with just a read lock, but if the string is not yet
        // present, we would then need to recheck after acquiring a write lock,
        // which is probably not worth it.
        let mut interner = INTERNER.write().unwrap();
        if let Some(&id) = interner.to_id.get(string) {
            return id;
        }

        // Create a new entry forever by leaking the string. PicoStr is only
        // used for strings that aren't created en masse, so it is okay.
        let num = interner.from_id.len().try_into().expect("out of string ids");
        let id = Self(num);
        let string = Box::leak(string.to_string().into_boxed_str());
        interner.to_id.insert(string, id);
        interner.from_id.push(string);
        id
    }

    /// Resolves the interned string.
    pub fn resolve(&self) -> &'static str {
        INTERNER.read().unwrap().from_id[self.0 as usize]
    }
}

impl Debug for PicoStr {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        self.resolve().fmt(f)
    }
}

impl Ord for PicoStr {
    fn cmp(&self, other: &Self) -> Ordering {
        self.resolve().cmp(other.resolve())
    }
}

impl PartialOrd for PicoStr {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl AsRef<str> for PicoStr {
    fn as_ref(&self) -> &str {
        self.resolve()
    }
}

impl From<&str> for PicoStr {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}
