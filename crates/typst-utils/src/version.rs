//! Typst version information.

/// Returns the version of Typst.
///
/// The information is read from the following sources:
///
/// - For the version number: The `TYPST_VERSION` environment variable
/// - For the commit hash: The `TYPST_COMMIT_SHA` environment variable
///
/// Build tooling can set these environment variables to configure the exposed
/// information. If the environment variables are left unset, the values are
/// populated via `build.rs` from the Cargo package manifest version and the git
/// hash in the current repository (if any).
///
/// # Panics
/// If the `TYPST_VERSION` environment variable holds a version string that
/// doesn't conform to SemVer.
pub fn version() -> TypstVersion {
    *crate::singleton!(TypstVersion, {
        let raw = env!("TYPST_VERSION");
        let commit = option_env!("TYPST_COMMIT_SHA");
        match semver::Version::parse(raw) {
            Ok(version) => {
                return TypstVersion {
                    major: version.major.try_into().unwrap(),
                    minor: version.minor.try_into().unwrap(),
                    patch: version.patch.try_into().unwrap(),
                    raw,
                    commit,
                };
            }
            Err(err) => {
                panic!("failed to parse {raw:?} as semantic version number: {err:?}")
            }
        }
    })
}

/// Typst version definition.
///
/// This structure contains the current Typst version. To query the precise
/// version number, refer to the [`TypstVersion::major()`],
/// [`TypstVersion::minor()`] and [`TypstVersion::patch()`] functions. You can
/// read the underlying, raw version string (e.g., for CLI output) with
/// [`TypstVersion::raw`].
///
/// Optionally, this may also contain the hash value of the Git commit from
/// which Typst was built. However, this field may be unpopulated.
#[derive(Debug, Clone, Copy)]
pub struct TypstVersion {
    /// Typst major version number.
    major: u32,
    /// Typst minor version number.
    minor: u32,
    /// Typst patch version number.
    patch: u32,
    /// Raw, unmodified version string.
    raw: &'static str,
    /// The raw commit hash.
    commit: Option<&'static str>,
}

impl TypstVersion {
    /// Returns the Typst major version.
    pub fn major(&self) -> u32 {
        self.major
    }

    /// Returns the Typst minor version.
    pub fn minor(&self) -> u32 {
        self.minor
    }

    /// Returns the Typst patch version.
    pub fn patch(&self) -> u32 {
        self.patch
    }

    /// Returns the raw, unparsed version string.
    ///
    /// Guaranteed to conform to SemVer.
    pub fn raw(&self) -> &'static str {
        self.raw
    }

    /// Returns the commit Typst was built from, if known.
    pub fn commit(&self) -> Option<&'static str> {
        self.commit
    }
}
