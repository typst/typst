use typst_syntax::PathError;
use typst_syntax::{FileId, VirtualRoot};

use crate::diag::{HintedStrResult, HintedString, error};
use crate::foundations::{Repr, Str, cast};

/// A path string.
///
/// This type is commonly accepted by functions that read from a path.
#[derive(Debug, Clone, PartialEq, Hash)]
pub struct PathStr(pub Str);

impl PathStr {
    /// Resolves this path string relative to the file that resides at `within`.
    ///
    /// The path string may be absolute or relative. If relative, it's resolved
    /// relative to the parent directory of `within` (which should point to a
    /// file rather than a directory).
    pub fn resolve(&self, within: FileId) -> HintedStrResult<FileId> {
        let root = within.root();
        let base = within.vpath();
        let resolved = match base.parent() {
            Some(parent) => parent.join(&self.0),
            None => base.join(&self.0),
        }
        .map_err(|err| format_path_error(err, root, &self.0))?;
        Ok(FileId::new(root.clone(), resolved))
    }

    /// [Resolves](Self::resolve) the path if `within` is `Some(_)` or returns
    /// an error that the file system could not be accessed, otherwise.
    pub fn resolve_if_some(&self, within: Option<FileId>) -> HintedStrResult<FileId> {
        self.resolve(within.ok_or("cannot access file system from here")?)
    }
}

cast! {
    PathStr,
    self => self.0.into_value(),
    v: Str => Self(v),
}

/// Format the user-facing path error message.
fn format_path_error(err: PathError, root: &VirtualRoot, path: &str) -> HintedString {
    match err {
        PathError::Escapes => {
            let kind = match root {
                VirtualRoot::Project => "project",
                VirtualRoot::Package(_) => "package",
            };
            let mut diag = error!(
                "path would escape the {kind} root";
                hint: "cannot access files outside of the {kind} sandbox";
            );
            if *root == VirtualRoot::Project {
                diag.hint("you can adjust the project root with the `--root` argument");
            }
            diag
        }
        PathError::Backslash => error!(
            "path must not contain a backslash";
            hint: "use forward slashes instead: `{}`",
            path.replace("\\", "/").repr();
            hint: "in earlier Typst versions, backslashes indicated path separators on Windows";
            hint: "this behavior is no longer supported as it is not portable";
        ),
    }
}

#[cfg(test)]
mod tests {
    use typst_syntax::{VirtualPath, VirtualRoot};

    use super::*;

    #[test]
    fn test_resolve() {
        let id = |p| FileId::new(VirtualRoot::Project, VirtualPath::new(p).unwrap());
        let id1 = id("src/main.typ");
        let resolve =
            |s: &str| PathStr(s.into()).resolve(id1).map_err(|err| err.message().clone());
        assert_eq!(resolve("works.bib"), Ok(id("src/works.bib")));
        assert_eq!(resolve(""), Ok(id("/src")));
        assert_eq!(resolve("."), Ok(id("/src")));
        assert_eq!(resolve(".."), Ok(id("/")));
        assert_eq!(resolve("../.."), Err("path would escape the project root".into()));
        assert_eq!(resolve("a\\b"), Err("path must not contain a backslash".into()));
    }
}
