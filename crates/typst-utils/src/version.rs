//! Typst version information.

/// Typst version definition.
///
/// This structure contains the current Typst version. To query the precise version number, refer
/// to the [`TypstVersion::major()`], [`TypstVersion::minor()`] and [`TypstVersion::patch()`]
/// functions. You can read the underlying, raw version string (e.g. for CLI output) with
/// [`TypstVersion::raw`].
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
        crate::singleton!(TypstVersion, {
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
    pub fn major(&self) -> u32 {
        self.major
    }

    /// Return the Typst minor version.
    pub fn minor(&self) -> u32 {
        self.minor
    }

    /// Return the Typst patch version.
    pub fn patch(&self) -> u32 {
        self.patch
    }

    /// Return the raw, unparsed version string.
    pub fn raw(&self) -> &'static str {
        self.raw
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[test]
    fn version_is_singleton() {
        let one_version = TypstVersion::new();
        let other_version = TypstVersion::new();

        assert!(std::ptr::eq(one_version, other_version));
    }

    #[test]
    fn version_copy_is_not_singleton() {
        let one_version = TypstVersion::new();
        let other_version = &(one_version.clone());

        assert!(!std::ptr::eq(one_version, other_version));
    }
}
