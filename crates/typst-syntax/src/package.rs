//! Package manifest parsing.

use std::collections::BTreeMap;
use std::fmt::{self, Debug, Display, Formatter};
use std::str::FromStr;

use ecow::{EcoString, eco_format};
use serde::de::IgnoredAny;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use unscanny::Scanner;

use crate::is_ident;

/// A type alias for a map of key-value pairs used to collect unknown fields
/// where values are completely discarded.
pub type UnknownFields = BTreeMap<EcoString, IgnoredAny>;

/// A parsed package manifest.
///
/// The `unknown_fields` contains fields which were found but not expected.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PackageManifest {
    /// Details about the package itself.
    pub package: PackageInfo,
    /// Details about the template, if the package is one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub template: Option<TemplateInfo>,
    /// The tools section for third-party configuration.
    #[serde(default)]
    pub tool: ToolInfo,
    /// All parsed but unknown fields, this can be used for validation.
    #[serde(flatten, skip_serializing)]
    pub unknown_fields: UnknownFields,
}

/// The `[tool]` key in the manifest. This field can be used to retrieve
/// 3rd-party tool configuration.
///
/// # Examples
/// ```
/// # use serde::{Deserialize, Serialize};
/// # use ecow::EcoString;
/// # use typst_syntax::package::PackageManifest;
/// #[derive(Debug, PartialEq, Serialize, Deserialize)]
/// struct MyTool {
///     key: EcoString,
/// }
///
/// let mut manifest: PackageManifest = toml::from_str(r#"
///     [package]
///     name = "package"
///     version = "0.1.0"
///     entrypoint = "src/lib.typ"
///
///     [tool.my-tool]
///     key = "value"
/// "#)?;
///
/// let my_tool = manifest
///     .tool
///     .sections
///     .remove("my-tool")
///     .ok_or("tool.my-tool section missing")?;
/// let my_tool = MyTool::deserialize(my_tool)?;
///
/// assert_eq!(my_tool, MyTool { key: "value".into() });
/// # Ok::<_, Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ToolInfo {
    /// Any fields parsed in the tool section.
    #[serde(flatten)]
    pub sections: BTreeMap<EcoString, toml::Table>,
}

/// The `[template]` key in the manifest.
///
/// The `unknown_fields` contains fields which were found but not expected.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TemplateInfo {
    /// The directory within the package that contains the files that should be
    /// copied into the user's new project directory.
    pub path: EcoString,
    /// A path relative to the template's path that points to the file serving
    /// as the compilation target.
    pub entrypoint: EcoString,
    /// A path relative to the package's root that points to a PNG or lossless
    /// WebP thumbnail for the template.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thumbnail: Option<EcoString>,
    /// All parsed but unknown fields, this can be used for validation.
    #[serde(flatten, skip_serializing)]
    pub unknown_fields: UnknownFields,
}

/// The `[package]` key in the manifest.
///
/// The `unknown_fields` contains fields which were found but not expected.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PackageInfo {
    /// The name of the package within its namespace.
    pub name: EcoString,
    /// The package's version.
    pub version: PackageVersion,
    /// The path of the entrypoint into the package.
    pub entrypoint: EcoString,
    /// A list of the package's authors.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub authors: Vec<EcoString>,
    ///  The package's license.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub license: Option<EcoString>,
    /// A short description of the package.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<EcoString>,
    /// A link to the package's web presence.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub homepage: Option<EcoString>,
    /// A link to the repository where this package is developed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repository: Option<EcoString>,
    /// An array of search keywords for the package.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub keywords: Vec<EcoString>,
    /// An array with up to three of the predefined categories to help users
    /// discover the package.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub categories: Vec<EcoString>,
    /// An array of disciplines defining the target audience for which the
    /// package is useful.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub disciplines: Vec<EcoString>,
    /// The minimum required compiler version for the package.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compiler: Option<VersionBound>,
    /// An array of globs specifying files that should not be part of the
    /// published bundle.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub exclude: Vec<EcoString>,
    /// All parsed but unknown fields, this can be used for validation.
    #[serde(flatten, skip_serializing)]
    pub unknown_fields: UnknownFields,
}

impl PackageManifest {
    /// Create a new package manifest with the given package info.
    pub fn new(package: PackageInfo) -> Self {
        PackageManifest {
            package,
            template: None,
            tool: ToolInfo::default(),
            unknown_fields: UnknownFields::new(),
        }
    }

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
            if !current.matches_ge(&required) {
                return Err(eco_format!(
                    "package requires typst {required} or newer \
                     (current version is {current})"
                ));
            }
        }

        Ok(())
    }
}

impl TemplateInfo {
    /// Create a new template info with only required fields.
    pub fn new(path: impl Into<EcoString>, entrypoint: impl Into<EcoString>) -> Self {
        TemplateInfo {
            path: path.into(),
            entrypoint: entrypoint.into(),
            thumbnail: None,
            unknown_fields: UnknownFields::new(),
        }
    }
}

impl PackageInfo {
    /// Create a new package info with only required fields.
    pub fn new(
        name: impl Into<EcoString>,
        version: PackageVersion,
        entrypoint: impl Into<EcoString>,
    ) -> Self {
        PackageInfo {
            name: name.into(),
            version,
            entrypoint: entrypoint.into(),
            authors: vec![],
            categories: vec![],
            compiler: None,
            description: None,
            disciplines: vec![],
            exclude: vec![],
            homepage: None,
            keywords: vec![],
            license: None,
            repository: None,
            unknown_fields: BTreeMap::new(),
        }
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

impl PackageSpec {
    pub fn versionless(&self) -> VersionlessPackageSpec {
        VersionlessPackageSpec {
            namespace: self.namespace.clone(),
            name: self.name.clone(),
        }
    }
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

    /// Performs an `==` match with the given version bound. Version elements
    /// missing in the bound are ignored.
    pub fn matches_eq(&self, bound: &VersionBound) -> bool {
        self.major == bound.major
            && bound.minor.is_none_or(|minor| self.minor == minor)
            && bound.patch.is_none_or(|patch| self.patch == patch)
    }

    /// Performs a `>` match with the given version bound. The match only
    /// succeeds if some version element in the bound is actually greater than
    /// that of the version.
    pub fn matches_gt(&self, bound: &VersionBound) -> bool {
        if self.major != bound.major {
            return self.major > bound.major;
        }
        let Some(minor) = bound.minor else { return false };
        if self.minor != minor {
            return self.minor > minor;
        }
        let Some(patch) = bound.patch else { return false };
        if self.patch != patch {
            return self.patch > patch;
        }
        false
    }

    /// Performs a `<` match with the given version bound. The match only
    /// succeeds if some version element in the bound is actually less than that
    /// of the version.
    pub fn matches_lt(&self, bound: &VersionBound) -> bool {
        if self.major != bound.major {
            return self.major < bound.major;
        }
        let Some(minor) = bound.minor else { return false };
        if self.minor != minor {
            return self.minor < minor;
        }
        let Some(patch) = bound.patch else { return false };
        if self.patch != patch {
            return self.patch < patch;
        }
        false
    }

    /// Performs a `>=` match with the given versions. The match succeeds when
    /// either a `==` or `>` match does.
    pub fn matches_ge(&self, bound: &VersionBound) -> bool {
        self.matches_eq(bound) || self.matches_gt(bound)
    }

    /// Performs a `<=` match with the given versions. The match succeeds when
    /// either a `==` or `<` match does.
    pub fn matches_le(&self, bound: &VersionBound) -> bool {
        self.matches_eq(bound) || self.matches_lt(bound)
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

/// A version bound for compatibility specification.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct VersionBound {
    /// The bounds's major version.
    pub major: u32,
    /// The bounds's minor version.
    pub minor: Option<u32>,
    /// The bounds's patch version. Can only be present if minor is too.
    pub patch: Option<u32>,
}

impl FromStr for VersionBound {
    type Err = EcoString;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts = s.split('.');
        let mut next = |kind| {
            if let Some(part) = parts.next() {
                part.parse::<u32>().map(Some).map_err(|_| {
                    eco_format!("`{part}` is not a valid {kind} version bound")
                })
            } else {
                Ok(None)
            }
        };

        let major = next("major")?
            .ok_or_else(|| eco_format!("version bound is missing major version"))?;
        let minor = next("minor")?;
        let patch = next("patch")?;
        if let Some(rest) = parts.next() {
            Err(eco_format!("version bound has unexpected fourth component: `{rest}`"))?;
        }

        Ok(Self { major, minor, patch })
    }
}

impl Debug for VersionBound {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        Display::fmt(self, f)
    }
}

impl Display for VersionBound {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", self.major)?;
        if let Some(minor) = self.minor {
            write!(f, ".{minor}")?;
        }
        if let Some(patch) = self.patch {
            write!(f, ".{patch}")?;
        }
        Ok(())
    }
}

impl Serialize for VersionBound {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for VersionBound {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let string = EcoString::deserialize(d)?;
        string.parse().map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    #[test]
    fn version_version_match() {
        let v1_1_1 = PackageVersion::from_str("1.1.1").unwrap();

        assert!(v1_1_1.matches_eq(&VersionBound::from_str("1").unwrap()));
        assert!(v1_1_1.matches_eq(&VersionBound::from_str("1.1").unwrap()));
        assert!(!v1_1_1.matches_eq(&VersionBound::from_str("1.2").unwrap()));

        assert!(!v1_1_1.matches_gt(&VersionBound::from_str("1").unwrap()));
        assert!(v1_1_1.matches_gt(&VersionBound::from_str("1.0").unwrap()));
        assert!(!v1_1_1.matches_gt(&VersionBound::from_str("1.1").unwrap()));

        assert!(!v1_1_1.matches_lt(&VersionBound::from_str("1").unwrap()));
        assert!(!v1_1_1.matches_lt(&VersionBound::from_str("1.1").unwrap()));
        assert!(v1_1_1.matches_lt(&VersionBound::from_str("1.2").unwrap()));
    }

    #[test]
    fn minimal_manifest() {
        assert_eq!(
            toml::from_str::<PackageManifest>(
                r#"
                [package]
                name = "package"
                version = "0.1.0"
                entrypoint = "src/lib.typ"
            "#
            ),
            Ok(PackageManifest {
                package: PackageInfo::new(
                    "package",
                    PackageVersion { major: 0, minor: 1, patch: 0 },
                    "src/lib.typ"
                ),
                template: None,
                tool: ToolInfo { sections: BTreeMap::new() },
                unknown_fields: BTreeMap::new(),
            })
        );
    }

    #[test]
    fn tool_section() {
        // NOTE: tool section must be table of tables, but we can't easily
        // compare the error structurally
        assert!(
            toml::from_str::<PackageManifest>(
                r#"
                [package]
                name = "package"
                version = "0.1.0"
                entrypoint = "src/lib.typ"

                [tool]
                not-table = "str"
            "#
            )
            .is_err()
        );

        #[derive(Debug, PartialEq, Serialize, Deserialize)]
        struct MyTool {
            key: EcoString,
        }

        let mut manifest: PackageManifest = toml::from_str(
            r#"
            [package]
            name = "package"
            version = "0.1.0"
            entrypoint = "src/lib.typ"

            [tool.my-tool]
            key = "value"
        "#,
        )
        .unwrap();

        let my_tool = manifest.tool.sections.remove("my-tool").unwrap();
        let my_tool = MyTool::deserialize(my_tool).unwrap();

        assert_eq!(my_tool, MyTool { key: "value".into() });
    }

    #[test]
    fn unknown_keys() {
        let manifest: PackageManifest = toml::from_str(
            r#"
            [package]
            name = "package"
            version = "0.1.0"
            entrypoint = "src/lib.typ"

            [unknown]
        "#,
        )
        .unwrap();

        assert!(manifest.unknown_fields.contains_key("unknown"));
    }
}
