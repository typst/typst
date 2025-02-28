use ecow::EcoString;
use typst_syntax::Spanned;

use crate::diag::{At, FileError, SourceResult};
use crate::engine::Engine;
use crate::foundations::{func, Cast};
use crate::loading::Readable;
use crate::World;

/// Reads plain text or data from a file.
///
/// By default, the file will be read as UTF-8 and returned as a [string]($str).
///
/// If you specify `{encoding: none}`, this returns raw [bytes] instead.
///
/// # Example
/// ```example
/// An example for a HTML file: \
/// #let text = read("example.html")
/// #raw(text, lang: "html")
///
/// Raw bytes:
/// #read("tiger.jpg", encoding: none)
/// ```
#[func]
pub fn read(
    engine: &mut Engine,
    /// Path to a file.
    ///
    /// For more details, see the [Paths section]($syntax/#paths).
    path: Spanned<EcoString>,
    /// The encoding to read the file with.
    ///
    /// If set to `{none}`, this function returns raw bytes.
    #[named]
    #[default(Some(Encoding::Utf8))]
    encoding: Option<Encoding>,
) -> SourceResult<Readable> {
    let Spanned { v: path, span } = path;
    let id = span.resolve_path(&path).at(span)?;
    let data = engine.world.file(id).at(span)?;
    Ok(match encoding {
        None => Readable::Bytes(data),
        Some(Encoding::Utf8) => {
            Readable::Str(data.to_str().map_err(FileError::from).at(span)?)
        }
    })
}

/// An encoding of a file.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Cast)]
pub enum Encoding {
    /// The Unicode UTF-8 encoding.
    Utf8,
}
