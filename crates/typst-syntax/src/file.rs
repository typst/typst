//! File and package management.

use std::collections::HashMap;
use std::fmt::{self, Debug, Formatter};
use std::sync::RwLock;

use once_cell::sync::Lazy;

use crate::package::PackageSpec;
use crate::VirtualPath;

/// The global package-path interner.
static INTERNER: Lazy<RwLock<Interner>> =
    Lazy::new(|| RwLock::new(Interner { to_id: HashMap::new(), from_id: Vec::new() }));

/// A package-path interner.
struct Interner {
    to_id: HashMap<Pair, FileId>,
    from_id: Vec<Pair>,
}

/// An interned pair of a package specification and a path.
type Pair = &'static (Option<PackageSpec>, VirtualPath);

/// Identifies a file in a project or package.
///
/// This type is globally interned and thus cheap to copy, compare, and hash.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct FileId(u16);

impl FileId {
    /// Create a new interned file specification.
    ///
    /// The path must start with a `/` or this function will panic.
    /// Note that the path is normalized before interning.
    #[track_caller]
    pub fn new(package: Option<PackageSpec>, path: VirtualPath) -> Self {
        // Try to find an existing entry that we can reuse.
        //
        // We could check with just a read lock, but if the pair is not yet
        // present, we would then need to recheck after acquiring a write lock,
        // which is probably not worth it.
        let pair = (package, path);
        let mut interner = INTERNER.write().unwrap();
        if let Some(&id) = interner.to_id.get(&pair) {
            return id;
        }

        // Create a new entry forever by leaking the pair. We can't leak more
        // than 2^16 pair (and typically will leak a lot less), so its not a
        // big deal.
        let num = interner.from_id.len().try_into().expect("out of file ids");
        let id = FileId(num);
        let leaked = Box::leak(Box::new(pair));
        interner.to_id.insert(leaked, id);
        interner.from_id.push(leaked);
        id
    }

    /// Create a new unique ("fake") file specification, which is not
    /// accessible by path.
    ///
    /// Caution: the ID returned by this method is the *only* identifier of the
    /// file, constructing a file ID with a path will *not* reuse the ID even
    /// if the path is the same. This method should only be used for generating
    /// "virtual" file ids such as content read from stdin.
    #[track_caller]
    pub fn new_fake(path: VirtualPath) -> Self {
        let mut interner = INTERNER.write().unwrap();
        let num = interner.from_id.len().try_into().expect("out of file ids");

        let id = FileId(num);
        let leaked = Box::leak(Box::new((None, path)));
        interner.from_id.push(leaked);
        id
    }

    /// The package the file resides in, if any.
    pub fn package(&self) -> Option<&'static PackageSpec> {
        self.pair().0.as_ref()
    }

    /// The absolute and normalized path to the file _within_ the project or
    /// package.
    pub fn vpath(&self) -> &'static VirtualPath {
        &self.pair().1
    }

    /// Resolve a file location relative to this file.
    pub fn join(self, path: &str) -> Self {
        Self::new(self.package().cloned(), self.vpath().join(path))
    }

    /// The same file location, but with a different extension.
    pub fn with_extension(&self, extension: &str) -> Self {
        Self::new(self.package().cloned(), self.vpath().with_extension(extension))
    }

    /// Construct from a raw number.
    ///
    /// Should only be used with numbers retrieved via
    /// [`into_raw`](Self::into_raw). Misuse may results in panics, but no
    /// unsafety.
    pub const fn from_raw(v: u16) -> Self {
        Self(v)
    }

    /// Extract the raw underlying number.
    pub const fn into_raw(self) -> u16 {
        self.0
    }

    /// Get the static pair.
    fn pair(&self) -> Pair {
        INTERNER.read().unwrap().from_id[usize::from(self.0)]
    }
}

impl Debug for FileId {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let vpath = self.vpath();
        match self.package() {
            Some(package) => write!(f, "{package:?}{vpath:?}"),
            None => write!(f, "{vpath:?}"),
        }
    }
}
