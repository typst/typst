//! File and package management.

use std::collections::HashMap;
use std::fmt::{self, Debug, Display, Formatter};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::RwLock;

use ecow::{eco_format, EcoString};
use once_cell::sync::Lazy;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::diag::{bail, FileError, StrResult};
use crate::syntax::is_ident;
use crate::util::PathExt;

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

/// Identifies a file.
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
        let pair = (package, path.normalize());
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
    pub fn join(self, path: &str) -> StrResult<Self> {
        if self.is_detached() {
            bail!("cannot access file system from here");
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

/// Identifies a package.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct PackageSpec {
    /// The namespace the package lives in.
    pub namespace: EcoString,
    /// The name of the package within its namespace.
    pub name: EcoString,
    /// The package's version.
    pub version: Version,
}

impl FromStr for PackageSpec {
    type Err = EcoString;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut s = unscanny::Scanner::new(s);
        if !s.eat_if('@') {
            bail!("package specification must start with '@'");
        }

        let namespace = s.eat_until('/');
        if namespace.is_empty() {
            bail!("package specification is missing namespace");
        } else if !is_ident(namespace) {
            bail!("`{namespace}` is not a valid package namespace");
        }

        s.eat_if('/');

        let name = s.eat_until(':');
        if name.is_empty() {
            bail!("package specification is missing name");
        } else if !is_ident(name) {
            bail!("`{name}` is not a valid package name");
        }

        s.eat_if(':');

        let version = s.after();
        if version.is_empty() {
            bail!("package specification is missing version");
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
pub struct Version {
    /// The package's major version.
    pub major: u32,
    /// The package's minor version.
    pub minor: u32,
    /// The package's patch version.
    pub patch: u32,
}

impl FromStr for Version {
    type Err = EcoString;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts = s.split('.');
        let mut next = |kind| {
            let Some(part) = parts.next().filter(|s| !s.is_empty()) else {
                bail!("version number is missing {kind} version");
            };
            part.parse::<u32>()
                .map_err(|_| eco_format!("`{part}` is not a valid {kind} version"))
        };

        let major = next("major")?;
        let minor = next("minor")?;
        let patch = next("patch")?;
        if let Some(rest) = parts.next() {
            bail!("version number has unexpected fourth component: `{rest}`");
        }

        Ok(Self { major, minor, patch })
    }
}

impl Display for Version {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl Serialize for Version {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for Version {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let string = EcoString::deserialize(d)?;
        string.parse().map_err(serde::de::Error::custom)
    }
}

/// A parsed package manifest.
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct PackageManifest {
    /// Details about the package itself.
    pub package: PackageInfo,
}

impl PackageManifest {
    /// Parse the manifest from raw bytes.
    pub fn parse(bytes: &[u8]) -> StrResult<Self> {
        let string = std::str::from_utf8(bytes).map_err(FileError::from)?;
        toml::from_str(string).map_err(|err| {
            eco_format!("package manifest is malformed: {}", err.message())
        })
    }

    /// Ensure that this manifest is indeed for the specified package.
    pub fn validate(&self, spec: &PackageSpec) -> StrResult<()> {
        if self.package.name != spec.name {
            bail!("package manifest contains mismatched name `{}`", self.package.name);
        }

        if self.package.version != spec.version {
            bail!(
                "package manifest contains mismatched version {}",
                self.package.version
            );
        }

        Ok(())
    }
}

/// The `package` key in the manifest.
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct PackageInfo {
    /// The name of the package within its namespace.
    pub name: EcoString,
    /// The package's version.
    pub version: Version,
    /// The path of the entrypoint into the package.
    pub entrypoint: EcoString,
}
