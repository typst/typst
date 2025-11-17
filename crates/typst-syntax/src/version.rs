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

/// Bundle an environment variable key with its value.
struct EnvWithSource {
    /// The key of the environment variable that has been read.
    key: &'static str,
    /// The value read from the environment variable.
    value: &'static str,
}

/// Construct an environment variable key along with its value.
///
/// This is glue code necessitated by the fact that `option_env!` expects a string literal as
/// argument. In order to prevent typing the env name twice, we wrap it into a macro that does this
/// for us.
macro_rules! env_with_source {
    ($key:literal) => {
        option_env!($key).map(|value| EnvWithSource { key: $key, value })
    };
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
            env_with_source!("TYPST_VERSION")
                .or(env_with_source!("CARGO_PKG_VERSION"))
                .ok_or(VersionError::Unknown)
                .and_then(|raw| match semver::Version::parse(raw.value) {
                    Ok(version) => Ok(Self { version, raw: raw.value }),
                    Err(_) => Err(VersionError::Invalid(raw)),
                })
                // NOTE: Strictly speaking we could return the `Result` instance and call it a day,
                // but then all callers of this code must handle that. The code previously didn't
                // do anything beyond unwrapping the versions parsed from `CARGO_PKG_VERSION`
                // anyway, so we might as well do it here once and be done with it.
                .expect(
                    "Typst version number should be available through the environment",
                )
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

/// Custom error type for Typst compiler version detection.
enum VersionError {
    /// Invalid version number (failed to parse)
    Invalid(EnvWithSource),
    /// Unknown version number (not defined)
    Unknown,
}

// For pretty printing the error in panic messages
impl std::fmt::Debug for VersionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Invalid(raw) => {
                writeln!(
                    f,
                    "failed to parse {:?} from variable {:?} as semantic version number",
                    raw.value, raw.key
                )
            }
            Self::Unknown => writeln!(
                f,
                "no version was specified at compile time, Typst version is unknown"
            ),
        }
    }
}
