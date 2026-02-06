use ecow::{EcoString, eco_format};
use typst_syntax::{FileId, PathError, RootedPath, Spanned, VirtualRoot};

use crate::diag::{At, HintedStrResult, HintedString, SourceResult, error};
use crate::foundations::{Repr, Str, cast, func, scope, ty};

/// A file system path.
///
/// When splitting up your project or package across multiple files, or
/// referencing resources such as images or bibliographies, you'll need to
/// interact with _paths._
///
/// # Path strings
/// Commonly, paths are simply expressed as [strings]($str). Built-in functions
/// that expect paths typically also accept strings. For instance, you can
/// write:
///
/// ```typ
/// #figure(
///   // Path to an image
///   image("tiger.jpg"),
///   caption: [A tiger],
/// )
///
/// // Path to a Typst file
/// #include "chapter.typ"
/// ```
///
/// There are two kinds of such path strings: Relative and absolute.
///
/// - A **relative path** resolves in relation to the parent directory of the
///   Typst file where the function is called. While this is the default, a path
///   can also be explicitly specified as being relative by starting it with
///   `./`.
///   ```typ
///   #image("images/logo.png")
///   #image("./images/logo.png") // This is equivalent
///   ```
///
/// - An **absolute path** always resolves relative to the
///   [_root_]($path/#project-root) of the project. Such a path is indicated by
///   a leading `/`:
///   ```typ
///   #image("/assets/logo.png")
///   ```
///
/// Paths consist of segments that are separated by forward slashes, with
/// interior segments indicating directories and the final one a file or a
/// directory. There are two path components that are treated specially:
///
/// - The segment `.` refers to the _current_ directory. This is why
///   `{"./image.png"}` and `{"image.png"}` are equivalent.
///
/// - The segment `..` refers to the _parent_ directory. If you have three files
///   `main.typ`, `utils.typ`, and `text/chapter1.typ`, then you can reference
///   your utility file from chapter 1 through the path `{"../utils.typ"}`.
///
/// # The path type { #path-type }
/// For most typical usage of paths, strings are all you need. However,
/// sometimes you need a bit more control. For instance, you may want to resolve
/// a path relative to the file you are currently writing in, but then pass it
/// to a package and let the package read from the path. This is where the path
/// type comes in.
///
/// With it, you can fully resolve a path string relative to the file where you
/// construct it. Any following operations performed with the path (such as a
/// file read or an image load), will then behave the same regardless of where
/// in the code they occur.
///
/// Here's an example of how we could have a `main.typ` with a `data.json` file
/// directly next to it and still let a package we've built read that file.
///
/// ```typ
/// // This is main.typ, with data.json next to it.
/// #import "@local/my-pkg:0.1.0": process
/// #let data-path = path("data.json")
/// #process(data-path)
/// ```
///
/// # Roots
/// ## The project root { #project-root }
/// For security and reproducibility reasons, Typst encapsulates file access. A
/// Typst project can only access paths within its _project root._ If you try to
/// create or access a path outside of this root, you'll get an error:
///
/// ```typ
/// // ❌ Error: path `"../secret.txt"` would escape the project root
/// #path("../secret.txt")
/// ```
///
/// By default, the project root is the parent directory of the main Typst file.
/// If you wish to use another folder as the root of your project, you can use
/// the CLI's `--root` flag:
///
/// ```bash
/// typst compile --root .. file.typ
/// ```
///
/// Make sure that the main file is contained in the folder's subtree, so that
/// Typst can access it.
///
/// In the web app, the project itself is the root directory. You can always
/// read all files within it, no matter which one is previewed (via the eye
/// toggle next to each Typst file in the file panel).
///
/// ## Package roots
/// Just like the project, each package you import has its own root. Within a
/// package, absolute paths point to the package root rather than the project
/// root. On its own, code in a package cannot construct a path that lives in
/// the project or another package.
///
/// If you need to provide a package with resources from the project (such as a
/// logo image), you can do so by explicitly creating a path to the resource in
/// your code with the [path constructor]($path/#constructor). You can then pass
/// the resulting path to the package. An example of this is shown in the
/// section ["The path type"]($path/#path-type) above.
///
/// Alternatively, you can perform the path operation in your code and pass the
/// result to the package. This could, for example, be the result of a [`read`]
/// call or a complete image (e.g. as a named parameter `{logo:
/// image("mylogo.svg")}`). Note that if you pass an image to a package like
/// this, you can still customize the image's appearance with a set rule within
/// the package.
///
/// # Further operations
/// For now, the path type's purpose is limited to correctly handling and
/// transferring paths across files in your project and packages. In the future,
/// it may enable additional capabilities like checking for the existence of a
/// file or enumerating files in a directory.
#[ty(scope, name = "path")]
#[derive(Debug, Clone, PartialEq, Hash)]
type RootedPath;

#[scope(ext)]
impl RootedPath {
    /// Creates a path from a string.
    ///
    /// ```typ
    /// // A relative path without a leading slash.
    /// // May optionally start with `./`.
    /// #path("relative/path/to/file.typ")
    /// #path("./relative/path/to/file.typ")
    ///
    /// // An absolute path with a leading slash.
    /// #path("/absolute/path/to/file.typ")
    /// ```
    #[func(constructor)]
    pub fn construct(
        /// Converts a string or path to a path.
        ///
        /// If this is a [path string]($path/#path-strings):
        /// - If the path is absolute, it is resolved relative to the root of
        ///   the project or package in which this function is called.
        /// - If the path is relative, it is resolved relative to the file where
        ///   this function is called.
        ///
        /// If this is already a `path`, it is returned unchanged.
        path: Spanned<PathOrStr>,
    ) -> SourceResult<RootedPath> {
        path.v.resolve_if_some(path.span.id()).at(path.span)
    }
}

impl Repr for RootedPath {
    fn repr(&self) -> EcoString {
        // The package spec is hard to reasonably express, but I'm also not sure
        // whether we want to expose it. For the path itself, we always use an
        // absolute one as that's the most portable representation.
        eco_format!("path({})", self.vpath().get_with_slash().repr())
    }
}

/// A string or a path.
///
/// This type is commonly accepted by functions that read from a path.
#[derive(Debug, Clone, PartialEq, Hash)]
pub enum PathOrStr {
    Path(RootedPath),
    Str(Str),
}

impl PathOrStr {
    /// Resolves this path or string relative to the file that resides at
    /// `within`.
    ///
    /// The path string may be absolute or relative. If relative, it's resolved
    /// relative to the parent directory of `within` (which should point to a
    /// file rather than a directory).
    pub fn resolve(&self, within: FileId) -> HintedStrResult<RootedPath> {
        Ok(match self {
            PathOrStr::Path(v) => v.clone(),
            PathOrStr::Str(v) => {
                let root = within.root();
                let base = within.vpath();
                let resolved = match base.parent() {
                    Some(parent) => parent.join(v),
                    None => base.join(v),
                }
                .map_err(|err| format_path_error(err, root, v))?;
                RootedPath::new(root.clone(), resolved)
            }
        })
    }

    /// [Resolves](Self::resolve) the path if `within` is `Some(_)` or returns
    /// an error that the file system could not be accessed, otherwise.
    pub fn resolve_if_some(&self, within: Option<FileId>) -> HintedStrResult<RootedPath> {
        self.resolve(within.ok_or("cannot access file system from here")?)
    }
}

cast! {
    PathOrStr,
    self => match self {
        Self::Path(v) => v.into_value(),
        Self::Str(v) => v.into_value(),
    },
    v: RootedPath => Self::Path(v),
    v: Str => Self::Str(v),
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
                "path `{}` would escape the {kind} root", path.repr();
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
        let path =
            |p| RootedPath::new(VirtualRoot::Project, VirtualPath::new(p).unwrap());
        let id = |p| path(p).intern();
        let id1 = id("src/main.typ");
        let resolve = |s: &str| {
            PathOrStr::Str(s.into())
                .resolve(id1)
                .map_err(|err| err.message().clone())
        };
        assert_eq!(resolve("works.bib"), Ok(path("src/works.bib")));
        assert_eq!(resolve(""), Ok(path("/src")));
        assert_eq!(resolve("."), Ok(path("/src")));
        assert_eq!(resolve(".."), Ok(path("/")));
        assert_eq!(
            resolve("../.."),
            Err("path `\"../..\"` would escape the project root".into())
        );
        assert_eq!(resolve("a\\b"), Err("path must not contain a backslash".into()));
    }
}
