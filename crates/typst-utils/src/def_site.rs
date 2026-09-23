/// Describes the source code where something is defined.
///
/// This does not include a concrete line number because, due to the way Rust
/// macros expand, a `#[func]` item in a `#[scope]` impl receives the line of
/// the full impl, which is not so useful. Rather, a per-file unique key is used
/// to find the element. Identifiers of a semantical parent may also be used in
/// this key. This has the added benefit that we can reliably find the
/// definition site in the presence of edits (for hot reload).
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct DefSite {
    /// The path to the file as returned by the `file!()` macro.
    ///
    /// Note that the path separator may vary across platforms.
    pub path: &'static str,
    /// An identifying key associated with the definition. Can be used to find
    /// the definition in the file.
    pub key: &'static str,
}
