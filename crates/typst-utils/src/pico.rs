use std::borrow::Borrow;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt::{self, Debug, Display, Formatter};
use std::hash::{Hash, Hasher};
use std::num::NonZeroU64;
use std::ops::Deref;
use std::sync::{LazyLock, RwLock};

/// Marks a number as a bitcode encoded `PicoStr``.
const MARKER: u64 = 1 << 63;

/// The global runtime string interner.
static INTERNER: LazyLock<RwLock<Interner>> =
    LazyLock::new(|| RwLock::new(Interner { seen: HashMap::new(), strings: Vec::new() }));

/// A string interner.
struct Interner {
    seen: HashMap<&'static str, PicoStr>,
    strings: Vec<&'static str>,
}

/// An interned string representation that is cheap to copy and hash, but more
/// expensive to access.
///
/// This type takes up 8 bytes and is copyable and null-optimized (i.e.
/// `Option<PicoStr>` also takes 8 bytes).
///
/// Supports compile-time string interning via [`PicoStr::constant`] in two
/// flavors:
/// - Strings of length at most 12 containing only chars from 'a'-'z', '1'-'4',
///   and '-' are stored inline in the number
/// - Other strings _can_ be compile-time interned the same way, but must first
///   be added to the list in `exceptions::LIST`.
///
/// No such restrictions apply at runtime (via [`PicoStr::intern`]).
#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub struct PicoStr(NonZeroU64);

impl PicoStr {
    /// Intern a string at runtime.
    pub fn intern(string: &str) -> PicoStr {
        // Try to use bitcode or exception representations.
        if let Ok(value) = PicoStr::try_constant(string) {
            return value;
        }

        // Try to find an existing entry that we can reuse.
        //
        // We could check with just a read lock, but if the string is not yet
        // present, we would then need to recheck after acquiring a write lock,
        // which is probably not worth it.
        let mut interner = INTERNER.write().unwrap();
        if let Some(&id) = interner.seen.get(string) {
            return id;
        }

        // Create a new entry forever by leaking the string. PicoStr is only
        // used for strings that aren't created en masse, so it is okay.
        let num = exceptions::LIST.len() + interner.strings.len() + 1;
        let id = Self(NonZeroU64::new(num as u64).unwrap());
        let string = Box::leak(string.to_string().into_boxed_str());
        interner.seen.insert(string, id);
        interner.strings.push(string);
        id
    }

    /// Creates a compile-time constant `PicoStr`.
    ///
    /// Should only be used in const contexts because it can panic.
    #[track_caller]
    pub const fn constant(string: &'static str) -> PicoStr {
        match PicoStr::try_constant(string) {
            Ok(value) => value,
            Err(err) => panic!("{}", err.message()),
        }
    }

    /// Try to intern a string statically at compile-time.
    pub const fn try_constant(string: &str) -> Result<PicoStr, bitcode::EncodingError> {
        // Try to encode with bitcode.
        let value = match bitcode::encode(string) {
            // Store representation marker in high bit. Bitcode doesn't use
            // 4 high bits.
            Ok(v) => v | MARKER,

            // If that fails, try to use the exception list.
            Err(e) => {
                if let Some(i) = exceptions::get(string) {
                    // Offset by one to make it non-zero.
                    i as u64 + 1
                } else {
                    return Err(e);
                }
            }
        };

        match NonZeroU64::new(value) {
            Some(value) => Ok(Self(value)),
            None => unreachable!(),
        }
    }

    /// Resolve to a decoded string.
    pub fn resolve(self) -> ResolvedPicoStr {
        // If high bit is set, this is a bitcode-encoded string.
        let value = self.0.get();
        if value & MARKER != 0 {
            return bitcode::decode(value & !MARKER);
        }

        let index = (value - 1) as usize;
        let string = if let Some(runtime) = index.checked_sub(exceptions::LIST.len()) {
            INTERNER.read().unwrap().strings[runtime]
        } else {
            exceptions::LIST[index]
        };

        ResolvedPicoStr(Repr::Static(string))
    }
}

impl Debug for PicoStr {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        Debug::fmt(self.resolve().as_str(), f)
    }
}

/// A 5-bit encoding for strings with length up two 12 that are restricted to a
/// specific charset.
mod bitcode {
    use super::{Repr, ResolvedPicoStr};

    /// Maps from encodings to their bytes.
    const DECODE: &[u8; 32] = b"\0abcdefghijklmnopqrstuvwxyz-1234";

    /// Maps from bytes to their encodings.
    const ENCODE: &[u8; 256] = &{
        let mut map = [0; 256];
        let mut i = 0;
        while i < DECODE.len() {
            map[DECODE[i] as usize] = i as u8;
            i += 1;
        }
        map
    };

    /// Try to encode a string as a 64-bit integer.
    pub const fn encode(string: &str) -> Result<u64, EncodingError> {
        let bytes = string.as_bytes();

        if bytes.len() > 12 {
            return Err(EncodingError::TooLong);
        }

        let mut num: u64 = 0;
        let mut i = bytes.len();
        while i > 0 {
            i -= 1;
            let b = bytes[i];
            let v = ENCODE[b as usize];
            if v == 0 {
                return Err(EncodingError::BadChar);
            }
            num <<= 5;
            num |= v as u64;
        }

        Ok(num)
    }

    /// Decode the string for a 64-bit integer.
    pub const fn decode(mut value: u64) -> ResolvedPicoStr {
        let mut buf = [0; 12];
        let mut len = 0;

        while value != 0 {
            let v = value & 0b11111;
            buf[len as usize] = DECODE[v as usize];
            len += 1;
            value >>= 5;
        }

        ResolvedPicoStr(Repr::Inline(buf, len))
    }

    /// A failure during compile-time interning.
    pub enum EncodingError {
        TooLong,
        BadChar,
    }

    impl EncodingError {
        pub const fn message(&self) -> &'static str {
            match self {
                Self::TooLong => {
                    "the maximum auto-internible string length is 12. \
                     you can add an exception to typst-utils/src/pico.rs \
                     to intern longer strings."
                }
                Self::BadChar => {
                    "can only auto-intern the chars 'a'-'z', '1'-'4', and '-'. \
                     you can add an exception to typst-utils/src/pico.rs \
                     to intern other strings."
                }
            }
        }
    }
}

/// Compile-time interned strings that cannot be encoded with `bitcode`.
mod exceptions {
    use std::cmp::Ordering;

    /// A global list of non-bitcode-encodable compile-time internible strings.
    pub const LIST: &[&str] = &[
        "cjk-latin-spacing",
        "discretionary-ligatures",
        "h5",
        "h6",
        "historical-ligatures",
        "mmultiscripts",
        "number-clearance",
        "number-margin",
        "numbering-scope",
        "page-numbering",
        "par-line-marker",
        "transparentize",
    ];

    /// Try to find the index of an exception if it exists.
    pub const fn get(string: &str) -> Option<usize> {
        let mut lo = 0;
        let mut hi = LIST.len();
        while lo < hi {
            let mid = (lo + hi) / 2;
            match strcmp(string, LIST[mid]) {
                Ordering::Less => hi = mid,
                Ordering::Greater => lo = mid + 1,
                Ordering::Equal => return Some(mid),
            }
        }
        None
    }

    /// Compare two strings.
    const fn strcmp(a: &str, b: &str) -> Ordering {
        let a = a.as_bytes();
        let b = b.as_bytes();
        let l = min(a.len(), b.len());

        let mut i = 0;
        while i < l {
            if a[i] == b[i] {
                i += 1;
            } else if a[i] < b[i] {
                return Ordering::Less;
            } else {
                return Ordering::Greater;
            }
        }

        if i < b.len() {
            Ordering::Less
        } else if i < a.len() {
            Ordering::Greater
        } else {
            Ordering::Equal
        }
    }

    /// Determine the minimum of two integers.
    const fn min(a: usize, b: usize) -> usize {
        if a < b {
            a
        } else {
            b
        }
    }
}

/// This is returned by [`PicoStr::resolve`].
///
/// Dereferences to a `str`.
pub struct ResolvedPicoStr(Repr);

/// Representation of a resolved string.
enum Repr {
    Inline([u8; 12], u8),
    Static(&'static str),
}

impl ResolvedPicoStr {
    /// Retrieve the underlying string.
    pub fn as_str(&self) -> &str {
        match &self.0 {
            Repr::Inline(buf, len) => unsafe {
                std::str::from_utf8_unchecked(&buf[..*len as usize])
            },
            Repr::Static(s) => s,
        }
    }
}

impl Debug for ResolvedPicoStr {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Debug::fmt(self.as_str(), f)
    }
}

impl Display for ResolvedPicoStr {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(self.as_str(), f)
    }
}

impl Deref for ResolvedPicoStr {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl AsRef<str> for ResolvedPicoStr {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Borrow<str> for ResolvedPicoStr {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl Eq for ResolvedPicoStr {}

impl PartialEq for ResolvedPicoStr {
    fn eq(&self, other: &Self) -> bool {
        self.as_str().eq(other.as_str())
    }
}

impl Ord for ResolvedPicoStr {
    fn cmp(&self, other: &Self) -> Ordering {
        self.as_str().cmp(other.as_str())
    }
}

impl PartialOrd for ResolvedPicoStr {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Hash for ResolvedPicoStr {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_str().hash(state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[track_caller]
    fn roundtrip(s: &str) {
        assert_eq!(PicoStr::intern(s).resolve().as_str(), s);
    }

    #[test]
    fn test_pico_str() {
        // Test comparing compile-time and runtime-interned bitcode string.
        const H1: PicoStr = PicoStr::constant("h1");
        assert_eq!(H1, PicoStr::intern("h1"));
        assert_eq!(H1.resolve().as_str(), "h1");

        // Test comparing compile-time and runtime-interned exception.
        const DISC: PicoStr = PicoStr::constant("discretionary-ligatures");
        assert_eq!(DISC, PicoStr::intern("discretionary-ligatures"));
        assert_eq!(DISC.resolve().as_str(), "discretionary-ligatures");

        // Test just roundtripping some strings.
        roundtrip("");
        roundtrip("hi");
        roundtrip("∆@<hi-10_");
        roundtrip("you");
        roundtrip("discretionary-ligatures");
    }

    /// Ensures that none of the exceptions is bitcode-encodable.
    #[test]
    fn test_exceptions_not_bitcode_encodable() {
        for s in exceptions::LIST {
            assert!(
                bitcode::encode(s).is_err(),
                "{s:?} can be encoded with bitcode and should not be an exception"
            );
        }
    }

    /// Ensures that the exceptions are sorted.
    #[test]
    fn test_exceptions_sorted() {
        for group in exceptions::LIST.windows(2) {
            assert!(group[0] < group[1], "{group:?} are out of order");
        }
    }

    /// Ensures that all exceptions can be found.
    #[test]
    fn test_exception_find() {
        for (i, s) in exceptions::LIST.iter().enumerate() {
            assert_eq!(exceptions::get(s), Some(i), "wrong index for {s:?}");
        }
        assert_eq!(exceptions::get("a"), None);
        assert_eq!(exceptions::get("another-"), None);
        assert_eq!(exceptions::get("z"), None);
    }
}
