//! Virtual, cross-platform reproducible path handling.

use std::fmt::{self, Debug, Formatter};
use std::path::{self, Path, PathBuf};

use ecow::{EcoString, eco_format};

// Special symbols in virtual paths.
const SEPARATOR: char = '/';
const CURRENT: &str = ".";
const PARENT: &str = "..";

/// A path in a virtual file system.
#[derive(Clone, Eq, PartialEq, Hash)]
pub struct VirtualPath(Segments);

impl VirtualPath {
    /// Creates a new virtual path.
    pub fn new(path: impl AsRef<str>) -> Result<Self, PathError> {
        let segments = Segments::normalize(components(path.as_ref()))?;
        Ok(Self(segments))
    }

    /// Creates a virtual path from a real path and a real root.
    ///
    /// Returns `None` if the file path is not contained in the root (i.e. if
    /// `root_path` is not a lexical prefix of `path`). No file system
    /// operations are performed.
    ///
    /// This is the single function that translates from a real path to a
    /// virtual path. Its counterpart is [`VirtualPath::realize`].
    pub fn virtualize(root_path: &Path, path: &Path) -> Result<Self, VirtualizeError> {
        let path = path.strip_prefix(root_path).map_err(|_| PathError::Escapes)?;
        let mut segments = Segments::new();
        for c in path.components() {
            let comp = match c {
                path::Component::RootDir => Component::Root,
                path::Component::CurDir => Component::Current,
                path::Component::ParentDir => Component::Parent,
                path::Component::Normal(s) => {
                    let string = s.to_str().ok_or(VirtualizeError::Utf8)?;
                    let segment = Segment::new(string)
                        .map_err(|s| VirtualizeError::Invalid(s.into()))?;
                    Component::Normal(segment)
                }
                path::Component::Prefix(_) => return Err(PathError::Escapes.into()),
            };
            segments.push_component(comp)?;
        }
        Ok(Self(segments))
    }

    /// Turns the virtual path into an actual file system path (where the
    /// project or package resides). You need to provide the appropriate `root`
    /// path, relative to which this path will be resolved.
    ///
    /// This can be used in the implementations of `World::source` and
    /// `World::file`.
    ///
    /// This is the single function that translates from a virtual path to a
    /// real path. Its counterpart is [`VirtualPath::virtualize`].
    pub fn realize(&self, root: &Path) -> PathBuf {
        let mut out = root.to_path_buf();
        for s in self.0.iter() {
            out.push(s.get());
        }
        out
    }

    /// Returns the path with a leading slash.
    pub fn get_with_slash(&self) -> &str {
        self.0.get_with_slash()
    }

    /// Returns the path without a leading slash.
    pub fn get_without_slash(&self) -> &str {
        self.0.get_without_slash()
    }

    /// Returns the file name portion of the path.
    pub fn file_name(&self) -> Option<&str> {
        self.0.last().map(Segment::get)
    }

    /// Returns the file name portion of the path without the extension.
    pub fn file_stem(&self) -> Option<&str> {
        let last = self.0.last()?;
        let (before, after) = last.split_dot();
        before.or(after)
    }

    /// Returns the file extension of the path.
    pub fn extension(&self) -> Option<&str> {
        let last = self.0.last()?;
        let (before, after) = last.split_dot();
        before.and(after)
    }

    /// Returns a modified path with an adjusted extension.
    ///
    /// # Panics
    /// Panics if the resulting path segment would be invalid, e.g. because the
    /// extension contains a forward or backslash.
    #[track_caller]
    pub fn with_extension(&self, ext: &str) -> Self {
        let Some(stem) = self.file_stem() else { return self.clone() };
        let buf = eco_format!("{stem}.{ext}");
        let segment = Segment::new(&buf).expect("extension is invalid");

        let mut segments = self.0.clone();
        segments.pop();
        segments.push(segment);
        Self(segments)
    }

    /// Returns the path with its final component removed.
    ///
    /// Returns `None` if the path is already at the root.
    pub fn parent(&self) -> Option<Self> {
        let mut segments = self.0.clone();
        if !segments.pop() {
            return None;
        }
        Some(Self(segments))
    }

    /// Joins the given `path` to `self`.
    pub fn join(&self, path: &str) -> Result<Self, PathError> {
        let combined = self
            .0
            .iter()
            .map(|c| Ok(Component::Normal(c)))
            .chain(components(path));
        let segments = Segments::normalize(combined)?;
        Ok(Self(segments))
    }

    /// Tries to express this path as a relative path from the given base path.
    pub fn relative_from(&self, base: &Self) -> Option<EcoString> {
        // Adapted from rustc's `path_relative_from` function (MIT).
        // Copyright 2012-2015 The Rust Project Developers.
        // See NOTICE for full attribution.
        let mut ita = self.0.iter();
        let mut itb = base.0.iter();
        let mut buf: Vec<&str> = vec![];
        loop {
            match (ita.next(), itb.next()) {
                (None, None) => break,
                (Some(a), None) => {
                    buf.push(a.get());
                    buf.extend(ita.map(Segment::get));
                    break;
                }
                (None, Some(_)) => buf.push(".."),
                (Some(a), Some(b)) if buf.is_empty() && a == b => (),
                (Some(a), Some(_)) => {
                    buf.extend(std::iter::repeat_n("..", 1 + itb.count()));
                    buf.push(a.get());
                    buf.extend(ita.map(Segment::get));
                    break;
                }
            }
        }

        Some(buf.join("/").into())
    }
}

impl VirtualPath {
    /// Create a virtual path from a real path and a real root.
    #[deprecated = "use `virtualize` with swapped arguments instead"]
    pub fn within_root(path: &Path, root: &Path) -> Option<Self> {
        Self::virtualize(root, path).ok()
    }

    /// Resolve the virtual path relative to an actual file system root
    /// (where the project or package resides).
    #[deprecated = "use `realize` instead"]
    pub fn resolve(&self, root: &Path) -> Option<PathBuf> {
        Some(self.realize(root))
    }

    /// Get the underlying path without a leading `/` or `\`.
    #[deprecated = "use `get_without_slash` instead"]
    pub fn as_rootless_path(&self) -> &Path {
        Path::new(self.get_without_slash())
    }

    /// Get the underlying path with a leading `/` or `\`.
    #[deprecated = "use `get_with_slash` instead"]
    pub fn as_rooted_path(&self) -> &Path {
        Path::new(self.get_with_slash())
    }
}

impl Debug for VirtualPath {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        self.get_with_slash().fmt(f)
    }
}

/// A component in a virtual path.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum Component<'a> {
    Root,
    Current,
    Parent,
    Normal(Segment<'a>),
}

/// Splits a user-supplied path into its constituent parts.
///
/// This only splits and recognizes special segments. It does not check the
/// validity of normal segments. This is done in [`Segments::push`].
fn components(path: &str) -> impl Iterator<Item = Result<Component<'_>, PathError>> {
    path.split(SEPARATOR).enumerate().map(|(i, s)| {
        match s {
            // A leading separator indicates an absolute path.
            "" if i == 0 && !path.is_empty() => Ok(Component::Root),
            // Consecutive separators have no effect.
            "" => Ok(Component::Current),
            CURRENT => Ok(Component::Current),
            PARENT => Ok(Component::Parent),
            other => match Segment::new(other) {
                Ok(segment) => Ok(Component::Normal(segment)),
                Err("\\") => Err(PathError::Backslash),
                Err(_) => unreachable!(),
            },
        }
    })
}

/// A segment in a normalized path.
///
/// A segments is never empty, `.`, or `..` and it never contains back- or
/// forward slashes.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
struct Segment<'a>(&'a str);

impl<'a> Segment<'a> {
    fn new(segment: &'a str) -> Result<Self, &'a str> {
        // These invariants are important to avoid the path from escaping the
        // root after being realized, in particular the `..` part.
        if matches!(segment, "" | CURRENT | PARENT) {
            return Err(segment);
        }

        // Interior separators or backslashes are not allowed.
        if let Some(m) = segment.matches([SEPARATOR, '\\']).next() {
            return Err(m);
        }

        Ok(Self(segment))
    }

    fn new_unchecked(segment: &'a str) -> Self {
        debug_assert!(Self::new(segment).is_ok());
        Self(segment)
    }

    fn get(self) -> &'a str {
        self.0
    }

    fn split_dot(self) -> (Option<&'a str>, Option<&'a str>) {
        let mut iter = self.0.rsplitn(2, '.');
        let after = iter.next();
        let before = iter.next();
        if before == Some("") { (Some(self.0), None) } else { (before, after) }
    }
}

/// Stores a sequence of path segments as a string.
///
/// The underlying string always represents a normalized absolute path and is
/// guaranteed to start with a slash. Segments are never empty, `.`, or `..` and
/// they never contain back- or forward slashes.
#[derive(Clone, Eq, PartialEq, Hash)]
struct Segments(EcoString);

impl Segments {
    fn new() -> Self {
        Self(EcoString::from(SEPARATOR))
    }

    fn normalize<'a>(
        comps: impl IntoIterator<Item = Result<Component<'a>, PathError>>,
    ) -> Result<Segments, PathError> {
        let mut out = Segments::new();
        for component in comps {
            out.push_component(component?)?;
        }
        Ok(out)
    }

    fn is_empty(&self) -> bool {
        self.0.len() == 1
    }

    fn get_with_slash(&self) -> &str {
        &self.0
    }

    fn get_without_slash(&self) -> &str {
        self.0.strip_prefix(SEPARATOR).expect("path to start with slash")
    }

    fn clear(&mut self) {
        self.0.truncate(1);
    }

    fn push_component(&mut self, component: Component) -> Result<(), PathError> {
        match component {
            // Root component resets the path.
            Component::Root => self.clear(),
            // Current component has no effect.
            Component::Current => {}
            // Parent component removes the last segment. If there is no
            // segment, this indicates that the path would escape the root.
            // In this case, we return an error.
            Component::Parent => {
                if !self.pop() {
                    return Err(PathError::Escapes);
                }
            }
            Component::Normal(segment) => self.push(segment),
        }
        Ok(())
    }

    fn push<'a>(&mut self, segment: Segment<'a>) {
        if !self.is_empty() {
            self.0.push(SEPARATOR);
        }
        self.0.push_str(segment.0);
    }

    fn pop(&mut self) -> bool {
        if self.is_empty() {
            return false;
        }
        let i = self.0.rfind(SEPARATOR).expect("to contain a slash");
        self.0.truncate(std::cmp::max(1, i));
        true
    }

    fn last(&self) -> Option<Segment<'_>> {
        self.iter().next_back()
    }

    fn iter(&self) -> impl DoubleEndedIterator<Item = Segment<'_>> {
        let mut iter = self.0[1..].split(SEPARATOR);
        if self.is_empty() {
            iter.next();
        }
        iter.map(Segment::new_unchecked)
    }
}

/// An error that can occur on construction or modification of a
/// [`VirtualPath`].
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum PathError {
    /// The constructed or modified path would escape the root. This would
    /// for instance, when trying to join `..` to the path `/`.
    ///
    /// Note that a path might still escape through symlinks.
    Escapes,
    /// The path contains a backslash. This is not allowed as it leads to
    /// cross-platform compatibility hazards (since Windows uses backslashes as
    /// a path separator).
    Backslash,
}

/// An error that can occur in [`VirtualPath::virtualize`].
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum VirtualizeError {
    /// A normal path error.
    Path(PathError),
    /// A path component contained an invalid string. This should almost never
    /// occur under normal circumstances, but it could happen if some OS allows
    /// forward slashes or dots in path components.
    Invalid(EcoString),
    /// The file path contains non-UTF-8 encodable bytes.
    Utf8,
}

impl From<PathError> for VirtualizeError {
    fn from(err: PathError) -> Self {
        Self::Path(err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[track_caller]
    fn path(p: &str) -> VirtualPath {
        VirtualPath::new(p).unwrap()
    }

    #[test]
    fn test_new() {
        #[track_caller]
        fn test(path: &str, expected: Result<&str, PathError>) {
            let path = VirtualPath::new(path);
            assert_eq!(
                path.as_ref().map(|s| s.get_with_slash()).map_err(Clone::clone),
                expected
            );
        }

        test("", Ok("/"));
        test("a/./file.txt", Ok("/a/file.txt"));
        test("file.txt", Ok("/file.txt"));
        test("/file.txt", Ok("/file.txt"));
        test("hello/world", Ok("/hello/world"));
        test("hello/world/", Ok("/hello/world"));
        test("a///b", Ok("/a/b"));
        test("/a///b", Ok("/a/b"));
        test("./world.txt", Ok("/world.txt"));
        test("./world.txt/", Ok("/world.txt"));
        test("hello/.././/wor/ld.typ.extra", Ok("/wor/ld.typ.extra"));
        test("hello/.../world", Ok("/hello/.../world"));
        test("\u{200b}..", Ok("/\u{200b}.."));
        test("..", Err(PathError::Escapes));
        test("../world.txt", Err(PathError::Escapes));
        test("a\\world.txt", Err(PathError::Backslash));
    }

    #[test]
    #[cfg(unix)]
    fn test_virtualize_unix() {
        test_virtualize("/", "/main.typ", Ok("/main.typ"));
        test_virtualize("//a/b", "/a//b///c//d", Ok("/c/d"));
        test_virtualize(
            "/home/typst/desktop/",
            "/home/typst/desktop/src/main.typ",
            Ok("/src/main.typ"),
        );
        test_virtualize(
            "/home/typst/desktop/",
            "/home/typst/main.typ",
            Err(PathError::Escapes.into()),
        );
    }

    #[test]
    #[cfg(windows)]
    fn test_virtualize_windows() {
        test_virtualize(
            "C:\\Users\\typst\\Desktop",
            "C:\\Users\\typst\\Desktop\\src\\main.typ",
            Ok("/src/main.typ"),
        );
        test_virtualize(
            "C:\\Users\\typst\\Desktop",
            "C:\\Users\\typst\\main.typ",
            Err(PathError::Escapes.into()),
        );
    }

    #[track_caller]
    fn test_virtualize(
        root_path: impl AsRef<Path>,
        path: impl AsRef<Path>,
        expected: Result<&str, VirtualizeError>,
    ) {
        assert_eq!(
            VirtualPath::virtualize(root_path.as_ref(), path.as_ref(),)
                .as_ref()
                .map(|v| v.get_with_slash())
                .map_err(Clone::clone),
            expected,
        );
    }

    #[test]
    fn test_realize() {
        let p = path("src/text/main.typ");
        assert_eq!(
            p.realize(Path::new("/home/users/typst")),
            Path::new("/home/users/typst/src/text/main.typ")
        );
    }

    #[test]
    fn test_file_ops() {
        let p1 = path("src/text/file.typ");
        assert_eq!(p1.file_name(), Some("file.typ"));
        assert_eq!(p1.file_stem(), Some("file"));
        assert_eq!(p1.extension(), Some("typ"));
        assert_eq!(p1.with_extension("txt"), path("src/text/file.txt"));
        assert_eq!(p1.parent(), Some(path("src/text")));

        let p2 = path("src");
        assert_eq!(p2.file_name(), Some("src"));
        assert_eq!(p2.file_stem(), Some("src"));
        assert_eq!(p2.extension(), None);
        assert_eq!(p2.with_extension("txt"), path("src.txt"));
        assert_eq!(p2.parent(), Some(path("/")));

        let p3 = path("");
        assert_eq!(p3.file_name(), None);
        assert_eq!(p3.file_stem(), None);
        assert_eq!(p3.extension(), None);
        assert_eq!(p3.with_extension("txt"), p3);
        assert_eq!(p3.parent(), None);
    }

    #[test]
    fn test_join() {
        let p1 = path("src");
        assert_eq!(p1.join("a\\b"), Err(PathError::Backslash));
        let p2 = p1.join("text").unwrap();
        assert_eq!(p2.get_with_slash(), "/src/text");
        let p3 = p2.join("..").unwrap();
        assert_eq!(p1, p3);
        assert_eq!(p3.get_with_slash(), "/src");
        let p4 = p3.join("..").unwrap();
        assert_eq!(p4.get_with_slash(), "/");
        assert_eq!(p4.join(".."), Err(PathError::Escapes));
    }

    #[test]
    fn test_relative_from() {
        let p1 = path("src/text/main.typ");
        assert_eq!(p1.relative_from(&path("/src/text")), Some("main.typ".into()));
        assert_eq!(p1.relative_from(&path("/src/data")), Some("../text/main.typ".into()));
        assert_eq!(p1.relative_from(&path("src/")), Some("text/main.typ".into()));
        assert_eq!(p1.relative_from(&path("/")), Some("src/text/main.typ".into()));

        let p2 = path("src");
        assert_eq!(p2.relative_from(&path("src")), Some("".into()));
        assert_eq!(p2.relative_from(&path("src/data")), Some("..".into()));
    }

    #[test]
    fn test_segments() {
        let mut s = Segments::new();
        assert_eq!(s.get_with_slash(), "/");
        assert_eq!(s.get_without_slash(), "");
        s.push(Segment::new("to").unwrap());
        assert_eq!(s.get_with_slash(), "/to");
        s.push(Segment::new("hi.txt").unwrap());
        assert_eq!(s.get_with_slash(), "/to/hi.txt");
        assert_eq!(s.get_without_slash(), "to/hi.txt");
        assert_eq!(s.last().map(Segment::get), Some("hi.txt"));
        assert!(s.pop());
        assert_eq!(s.get_with_slash(), "/to");
        assert!(s.pop());
        assert_eq!(s.get_with_slash(), "/");
        assert!(!s.pop());
        assert_eq!(s.get_with_slash(), "/");
        assert_eq!(s.last(), None);
    }

    #[test]
    fn test_segment() {
        assert_eq!(Segment::new("\\b"), Err("\\"));
        assert_eq!(Segment::new("a/b"), Err("/"));
        assert_eq!(Segment::new(""), Err(""));
        assert_eq!(Segment::new("."), Err("."));
        assert_eq!(Segment::new(".."), Err(".."));
    }
}
