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
type Pair = &'static (Option<PackageSpec>, PathBuf);

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
    pub fn new(package: Option<PackageSpec>, path: &Path) -> Self {
        assert_eq!(
            path.components().next(),
            Some(std::path::Component::RootDir),
            "file path must be absolute within project or package: {}",
            path.display(),
        );

        // Try to find an existing entry that we can reuse.
        let pair = (package, normalize_path(path));
        if let Some(&id) = INTERNER.read().unwrap().to_id.get(&pair) {
            return id;
        }

        let mut interner = INTERNER.write().unwrap();
        let len = interner.from_id.len();
        if len >= usize::from(u16::MAX) {
            panic!("too many file specifications");
        }

        // Create a new entry forever by leaking the pair. We can't leak more
        // than 2^16 pair (and typically will leak a lot less), so its not a
        // big deal.
        let id = FileId(len as u16);
        let leaked = Box::leak(Box::new(pair));
        interner.to_id.insert(leaked, id);
        interner.from_id.push(leaked);
        id
    }

    /// Get an id that does not identify any real file.
    pub const fn detached() -> Self {
        Self(u16::MAX)
    }

    /// Whether the id is the detached.
    pub const fn is_detached(self) -> bool {
        self.0 == Self::detached().0
    }

    /// The package the file resides in, if any.
    pub fn package(&self) -> Option<&'static PackageSpec> {
        if self.is_detached() {
            None
        } else {
            self.pair().0.as_ref()
        }
    }

    /// The absolute and normalized path to the file _within_ the project or
    /// package.
    pub fn path(&self) -> &'static Path {
        if self.is_detached() {
            Path::new("/detached.typ")
        } else {
            &self.pair().1
        }
    }

    /// Resolve a file location relative to this file.
    pub fn join(self, path: &str) -> Result<Self, EcoString> {
        if self.is_detached() {
            Err("cannot access file system from here")?;
        }

        let package = self.package().cloned();
        let base = self.path();
        Ok(if let Some(parent) = base.parent() {
            Self::new(package, &parent.join(path))
        } else {
            Self::new(package, Path::new(path))
        })
    }

    /// Construct from a raw number.
    pub(crate) const fn from_u16(v: u16) -> Self {
        Self(v)
    }

    /// Extract the raw underlying number.
    pub(crate) const fn as_u16(self) -> u16 {
        self.0
    }

    /// Get the static pair.
    fn pair(&self) -> Pair {
        INTERNER.read().unwrap().from_id[usize::from(self.0)]
    }
}

impl Display for FileId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let path = self.path().display();
        match self.package() {
            Some(package) => write!(f, "{package}{path}"),
            None => write!(f, "{path}"),
        }
    }
}

impl Debug for FileId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(self, f)
    }
}

/// Lexically normalize a path.
fn normalize_path(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => match out.components().next_back() {
                Some(Component::Normal(_)) => {
                    out.pop();
                }
                _ => out.push(component),
            },
            Component::Prefix(_) | Component::RootDir | Component::Normal(_) => {
                out.push(component)
            }
        }
    }
    if out.as_os_str().is_empty() {
        out.push(Component::CurDir);
    }
    out
}

/// Identifies a package.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
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

impl Display for PackageSpec {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "@{}/{}:{}", self.namespace, self.name, self.version)
    }
}

/// A package's version.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct PackageVersion {
    /// The package's major version.
    pub major: u32,
    /// The package's minor version.
    pub minor: u32,
    /// The package's patch version.
    pub patch: u32,
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
