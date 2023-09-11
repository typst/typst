//! File and package management.

use std::collections::HashMap;
use std::fmt::{self, Debug, Display, Formatter};
use std::path::{Component, Path, PathBuf};
use std::str::FromStr;
use std::sync::RwLock;

use ecow::{eco_format, EcoString};
use once_cell::sync::Lazy;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::is_ident;

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
        let pair = (package, path);
        if let Some(&id) = INTERNER.read().unwrap().to_id.get(&pair) {
            return id;
        }

        let mut interner = INTERNER.write().unwrap();
        let num = interner.from_id.len().try_into().expect("out of file ids");

        // Create a new entry forever by leaking the pair. We can't leak more
        // than 2^16 pair (and typically will leak a lot less), so its not a
        // big deal.
        let id = FileId(num);
        let leaked = Box::leak(Box::new(pair));
        interner.to_id.insert(leaked, id);
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

    /// Construct from a raw number.
    pub(crate) const fn from_raw(v: u16) -> Self {
        Self(v)
    }

    /// Extract the raw underlying number.
    pub(crate) const fn into_raw(self) -> u16 {
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

/// An absolute path in the virtual file system of a project or package.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct VirtualPath(PathBuf);

impl VirtualPath {
    /// Create a new virtual path.
    ///
    /// Even if it doesn't start with `/` or `\`, it is still interpreted as
    /// starting from the root.
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self::new_impl(path.as_ref())
    }

    /// Non generic new implementation.
    fn new_impl(path: &Path) -> Self {
        let mut out = Path::new(&Component::RootDir).to_path_buf();
        for component in path.components() {
            match component {
                Component::Prefix(_) | Component::RootDir => {}
                Component::CurDir => {}
                Component::ParentDir => match out.components().next_back() {
                    Some(Component::Normal(_)) => {
                        out.pop();
                    }
                    _ => out.push(component),
                },
                Component::Normal(_) => out.push(component),
            }
        }
        Self(out)
    }

    /// Create a virtual path from a real path and a real root.
    ///
    /// Returns `None` if the file path is not contained in the root (i.e. if
    /// `root` is not a lexical prefix of `path`). No file system operations are
    /// performed.
    pub fn within_root(path: &Path, root: &Path) -> Option<Self> {
        path.strip_prefix(root).ok().map(Self::new)
    }

    /// Get the underlying path with a leading `/` or `\`.
    pub fn as_rooted_path(&self) -> &Path {
        &self.0
    }

    /// Get the underlying path without a leading `/` or `\`.
    pub fn as_rootless_path(&self) -> &Path {
        self.0.strip_prefix(Component::RootDir).unwrap_or(&self.0)
    }

    /// Resolve the virtual path relative to an actual file system root
    /// (where the project or package resides).
    ///
    /// Returns `None` if the path lexically escapes the root. The path might
    /// still escape through symlinks.
    pub fn resolve(&self, root: &Path) -> Option<PathBuf> {
        let root_len = root.as_os_str().len();
        let mut out = root.to_path_buf();
        for component in self.0.components() {
            match component {
                Component::Prefix(_) => {}
                Component::RootDir => {}
                Component::CurDir => {}
                Component::ParentDir => {
                    out.pop();
                    if out.as_os_str().len() < root_len {
                        return None;
                    }
                }
                Component::Normal(_) => out.push(component),
            }
        }
        Some(out)
    }

    /// Resolve a path relative to this virtual path.
    pub fn join(&self, path: impl AsRef<Path>) -> Self {
        if let Some(parent) = self.0.parent() {
            Self::new(parent.join(path))
        } else {
            Self::new(path)
        }
    }
}

impl Debug for VirtualPath {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        Display::fmt(&self.0.display(), f)
    }
}

/// Identifies a package.
#[derive(Clone, Eq, PartialEq, Hash)]
pub struct PackageSpec {
    /// The namespace the package lives in.
    pub namespace: EcoString,
    /// The name of the package within its namespace.
    pub name: EcoString,
    /// The package's version.
    pub version: PackageVersion,
}

impl FromStr for PackageSpec {
    type Err = EcoString;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut s = unscanny::Scanner::new(s);
        if !s.eat_if('@') {
            Err("package specification must start with '@'")?;
        }

        let namespace = s.eat_until('/');
        if namespace.is_empty() {
            Err("package specification is missing namespace")?;
        } else if !is_ident(namespace) {
            Err(eco_format!("`{namespace}` is not a valid package namespace"))?;
        }

        s.eat_if('/');

        let name = s.eat_until(':');
        if name.is_empty() {
            Err("package specification is missing name")?;
        } else if !is_ident(name) {
            Err(eco_format!("`{name}` is not a valid package name"))?;
        }

        s.eat_if(':');

        let version = s.after();
        if version.is_empty() {
            Err("package specification is missing version")?;
        }

        Ok(Self {
            namespace: namespace.into(),
            name: name.into(),
            version: version.parse()?,
        })
    }
}

impl Debug for PackageSpec {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        Display::fmt(self, f)
    }
}

impl Display for PackageSpec {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "@{}/{}:{}", self.namespace, self.name, self.version)
    }
}

/// A package's version.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct PackageVersion {
    /// The package's major version.
    pub major: u32,
    /// The package's minor version.
    pub minor: u32,
    /// The package's patch version.
    pub patch: u32,
}

impl PackageVersion {
    /// The current compiler version.
    pub fn compiler() -> Self {
        Self {
            major: env!("CARGO_PKG_VERSION_MAJOR").parse().unwrap(),
            minor: env!("CARGO_PKG_VERSION_MINOR").parse().unwrap(),
            patch: env!("CARGO_PKG_VERSION_PATCH").parse().unwrap(),
        }
    }
}

impl FromStr for PackageVersion {
    type Err = EcoString;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts = s.split('.');
        let mut next = |kind| {
            let part = parts
                .next()
                .filter(|s| !s.is_empty())
                .ok_or_else(|| eco_format!("version number is missing {kind} version"))?;
            part.parse::<u32>()
                .map_err(|_| eco_format!("`{part}` is not a valid {kind} version"))
        };

        let major = next("major")?;
        let minor = next("minor")?;
        let patch = next("patch")?;
        if let Some(rest) = parts.next() {
            Err(eco_format!("version number has unexpected fourth component: `{rest}`"))?;
        }

        Ok(Self { major, minor, patch })
    }
}

impl Debug for PackageVersion {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        Display::fmt(self, f)
    }
}

impl Display for PackageVersion {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl Serialize for PackageVersion {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for PackageVersion {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let string = EcoString::deserialize(d)?;
        string.parse().map_err(serde::de::Error::custom)
    }
}
