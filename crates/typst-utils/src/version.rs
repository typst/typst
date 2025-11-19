use std::sync::OnceLock;

/// Static reference to the Typst version.
///
/// Refer to [`TypstVersion::new`] for a controlled way to obtain the locks content. By wrapping
/// [`TypstVersion`] into this structure we achieve two things:
///
/// 1. We have a singleton instance, so no matter how much code uses the version information it is
///    never duplicated.
/// 2. It has a `'static` lifetime, which makes it convenient to use pretty much anywhere in the
///    code.
static TYPST_VERSION_REF: OnceLock<TypstVersion> = OnceLock::new();

/// Typst version definition.
///
/// This structure contains the current Typst version. To query the precise version number, refer
/// to the [`TypstVersion::major()`], [`TypstVersion::minor()`] and [`TypstVersion::patch()`]
/// functions. You can read the underlying, raw version string (e.g. for CLI output) with
/// [`TypstVersion::raw`].
#[derive(Debug)]
pub struct TypstVersion {
    /// Parsed Typst version according to semantic versioning.
    version: semver::Version,
    /// Raw, unmodified version string.
    raw: &'static str,
}

impl TypstVersion {
    /// Get the Typst version.
    ///
    /// The raw Typst version is read from the following environment variables in order of
    /// precedence:
    ///
    /// - `TYPST_VERSION`
    /// - `CARGO_PKG_VERSION`
    ///
    /// Once that is obtained, it is parsed into a SemVer-compatible version structure.
    ///
    /// # Panics
    ///
    /// If all the environment variables mentioned above are undefined, or if an environment
    /// variable holds a version definition that doesn't conform to SemVer.
    pub fn new() -> &'static Self {
        TYPST_VERSION_REF.get_or_init(|| {
            if let Some(raw) = option_env!("TYPST_VERSION") {
                match semver::Version::parse(raw) {
                    Ok(version) => Self { version, raw },
                    Err(err) => panic!(
                        "failed to parse {:?} from variable {:?} as semantic version number: {:?}",
                        raw,
                        "TYPST_VERSION",
                        err,
                    ),
                }
            } else if let Some(raw) = option_env!("CARGO_PKG_VERSION") {
                match semver::Version::parse(raw) {
                    Ok(version) => Self { version, raw },
                    Err(err) => panic!(
                        "failed to parse {:?} from variable {:?} as semantic version number: {:?}",
                        raw,
                        "CARGO_PKG_VERSION",
                        err,
                    ),
                }
            } else {
                panic!("no version was specified at compile time, Typst version is unknown");
            }
        })
    }

    /// Return the Typst major version.
    pub fn major(&self) -> u64 {
        self.version.major
    }

    /// Return the Typst minor version.
    pub fn minor(&self) -> u64 {
        self.version.minor
    }

    /// Return the Typst patch version.
    pub fn patch(&self) -> u64 {
        self.version.patch
    }

    /// Return the raw, unparsed version string.
    pub fn raw(&self) -> &'static str {
        self.raw
    }
}
