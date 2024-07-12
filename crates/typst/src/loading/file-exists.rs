use ecow::EcoString;

use crate::diag::{At, SourceResult};
use crate::engine::Engine;
use crate::foundations::func;
use crate::syntax::Spanned;
use crate::World;

/// Checks whether a file exists at a path.
///
/// It does not check for encoding errors.
///
/// # Example
/// ```example
/// An example for safely reading a file: \
/// #if file-exists("example.html") {
///     let text = read("example.html")
///     raw(text, lang: "html")
/// }
/// ```
#[func(title = "file-exists")]
pub fn file_exists(
    /// The engine.
    engine: &mut Engine,
    /// Path to a file.
    path: Spanned<EcoString>,
) -> SourceResult<bool> {
    let Spanned { v: path, span } = path;
    let resolved_path = match span.resolve_path(&path).at(span) {
        Err(_) => return Ok(false),
        Ok(id) => id,
    };
    //since all loading functions should use internal caching and
    //most often the file is read afterwards the penalty of reading should
    //not be big.
    match engine.world.file(resolved_path).at(span) {
        Err(_) => Ok(false),
        Ok(_) => Ok(true),
    }
}
