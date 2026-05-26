use comemo::Tracked;
use typst_syntax::package::{CompilerVersion, PackageManifest, VersionBound};
use typst_syntax::{FileId, VirtualRoot};

use crate::World;
use crate::diag::{StrResult, bail};
use crate::engine::Sink;
use ecow::eco_format;

// NOTE: Do not reorder these fields, the PartialOrd/Ord implementations rely on
// these.

/// The current compatibility mode of the Typst compiler.
///
/// Consider the case of string interpolation as a practical example. If this
/// was added in `0.15` and resulted in breakage across a large number of
/// packages, the Typst authors may opt to disable the new parsing rules for
/// packages which don't target `0.15` while enabling them in those that do.
///
/// This is done by checking the target compiler version of a package or project
/// and deciding which rule to use at runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Compat {
    /// The major version.
    pub major: u32,

    /// The minor version.
    pub minor: u32,
}

impl Compat {
    /// Creates a compatibility version bound.
    pub const fn new(major: u32, minor: u32) -> Self {
        Self { major, minor }
    }

    /// Creates a compatibility version bound for the current compiler version.
    pub fn compiler() -> Self {
        let compiler = typst_utils::version();
        Self::new(compiler.major(), compiler.minor())
    }

    /// Creates a new compatibility for the given `compiler.preferred` version
    /// bound. The patch version is ignored, as patch versions should never add,
    /// deprecate, or remove features.
    pub fn from_preferred(preferred: VersionBound) -> StrResult<Self> {
        if preferred.major != 0 {
            bail!(
                "major versions other than 0 are not supported for `compiler.preferred`"
            );
        }

        let Some(minor) = preferred.minor else {
            bail!("major version only bounds are not supported for `compiler.preferred`");
        };

        if preferred.patch.is_some() {
            bail!("patch versions are not supported for `compiler.preferred`");
        }

        Ok(Self::new(preferred.major, minor))
    }
}

impl Compat {
    /// Whether the string interpolation (`let x = "who"; "Horten hears a #x!"`)
    /// is supported.
    pub fn string_interpolation(&self, _sink: &mut Sink) -> bool {
        *self >= Compat::new(0, 15)
    }
}

// TODO: If I understood comemo right, this would create and cache a new package
// manifest for every `id` + `world.file(manifest(id))` combination despite many
// of them being similar. Ideally an inner memoized function should be used to
// normalize the id -> manifest(id) mapping.

// TODO: Is `StrResult` the correct type here? If yes, properly format the inner
// errors.

// TODO: Benchmark whether the memoized call is too slow for the hot paths in
// parsing, eval, or layout.

// TODO: Not all errors here should have the same span.

/// Return the compatibility mode for a file.
///
/// For files in the project root of the compilation this is simply
/// [`Compatibility::Current`]. For files in packages, this depends on the
/// configured preferred compiler version in the package manifest.
#[comemo::memoize]
pub fn get_compatibility(
    world: Tracked<'_, dyn World + '_>,
    id: FileId,
) -> StrResult<Compat> {
    let root = id.root();
    let path = match root {
        VirtualRoot::Project => return Ok(Compat::compiler()),
        VirtualRoot::Package(_) => {
            root.join("typst.toml").map_err(|e| eco_format!("{e:?}"))?
        }
    };

    let manifest = world.file(path.intern())?;
    let manifest = manifest.as_str().map_err(|e| eco_format!("{e:?}"))?;

    let manifest: PackageManifest =
        toml::from_str(manifest).map_err(|e| eco_format!("{e:?}"))?;

    let Some(compiler) = manifest.package.compiler else {
        return Ok(Compat::compiler());
    };

    match compiler {
        CompilerVersion::Minimum(_) => Ok(Compat::compiler()),
        CompilerVersion::Compatibility { preferred, .. } => {
            Ok(Compat::from_preferred(preferred)?)
        }
    }
}
