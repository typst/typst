use std::fmt::{self, Debug, Display, Formatter};
use std::path::{Component, Path, PathBuf};

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

    /// The same path, but with a different extension.
    pub fn with_extension(&self, extension: &str) -> Self {
        Self(self.0.with_extension(extension))
    }
}

impl Debug for VirtualPath {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        Display::fmt(&self.0.display(), f)
    }
}
