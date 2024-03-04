//! Package manifest parsing.

use std::fmt::{self, Debug, Display, Formatter};
use std::str::FromStr;

use ecow::{eco_format, EcoString};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use unscanny::Scanner;

use crate::is_ident;

/// A parsed package manifest.
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct PackageManifest {
    /// Details about the package itself.
    pub package: PackageInfo,
    /// Details about the template, if the package is one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template: Option<TemplateInfo>,
}

/// The `[template]` key in the manifest.
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct TemplateInfo {
    /// The path of the starting point within the package.
    pub path: EcoString,
    /// The path of the entrypoint relative to the starting point's `path`.
    pub entrypoint: EcoString,
}

/// The `[package]` key in the manifest.
///
/// More fields are specified, but they are not relevant to the compiler.
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct PackageInfo {
    /// The name of the package within its namespace.
    pub name: EcoString,
    /// The package's version.
    pub version: PackageVersion,
    /// The path of the entrypoint into the package.
    pub entrypoint: EcoString,
    /// The minimum required compiler version for the package.
    pub compiler: Option<PackageVersion>,
}

impl PackageManifest {
    /// Ensure that this manifest is indeed for the specified package.
    pub fn validate(&self, spec: &PackageSpec) -> Result<(), EcoString> {
        if self.package.name != spec.name {
            return Err(eco_format!(
                "package manifest contains mismatched name `{}`",
                self.package.name
            ));
        }

        if self.package.version != spec.version {
            return Err(eco_format!(
                "package manifest contains mismatched version {}",
                self.package.version
            ));
        }

        if let Some(required) = self.package.compiler {
            let current = PackageVersion::compiler();
            if current < required {
                return Err(eco_format!(
                    "package requires typst {required} or newer \
                     (current version is {current})"
                ));
            }
        }

        Ok(())
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
        let namespace = parse_namespace(&mut s)?.into();
        let name = parse_name(&mut s)?.into();
        let version = parse_version(&mut s)?;
        Ok(Self { namespace, name, version })
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

/// Identifies a package, but not a specific version of it.
#[derive(Clone, Eq, PartialEq, Hash)]
pub struct VersionlessPackageSpec {
    /// The namespace the package lives in.
    pub namespace: EcoString,
    /// The name of the package within its namespace.
    pub name: EcoString,
}

impl VersionlessPackageSpec {
    /// Fill in the `version` to get a complete [`PackageSpec`].
    pub fn at(self, version: PackageVersion) -> PackageSpec {
        PackageSpec {
            namespace: self.namespace,
            name: self.name,
            version,
        }
    }
}

impl FromStr for VersionlessPackageSpec {
    type Err = EcoString;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut s = unscanny::Scanner::new(s);
        let namespace = parse_namespace(&mut s)?.into();
        let name = parse_name(&mut s)?.into();
        if !s.done() {
            Err("unexpected version in versionless package specification")?;
        }
        Ok(Self { namespace, name })
    }
}

impl Debug for VersionlessPackageSpec {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        Display::fmt(self, f)
    }
}

impl Display for VersionlessPackageSpec {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "@{}/{}", self.namespace, self.name)
    }
}

fn parse_namespace<'s>(s: &mut Scanner<'s>) -> Result<&'s str, EcoString> {
    if !s.eat_if('@') {
        Err("package specification must start with '@'")?;
    }

    let namespace = s.eat_until('/');
    if namespace.is_empty() {
        Err("package specification is missing namespace")?;
    } else if !is_ident(namespace) {
        Err(eco_format!("`{namespace}` is not a valid package namespace"))?;
    }

    Ok(namespace)
}

fn parse_name<'s>(s: &mut Scanner<'s>) -> Result<&'s str, EcoString> {
    s.eat_if('/');

    let name = s.eat_until(':');
    if name.is_empty() {
        Err("package specification is missing name")?;
    } else if !is_ident(name) {
        Err(eco_format!("`{name}` is not a valid package name"))?;
    }

    Ok(name)
}

fn parse_version(s: &mut Scanner) -> Result<PackageVersion, EcoString> {
    s.eat_if(':');

    let version = s.after();
    if version.is_empty() {
        Err("package specification is missing version")?;
    }

    version.parse()
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
