use ecow::EcoString;
use typst_syntax::Spanned;

use crate::diag::SourceResult;
use crate::engine::Engine;
use crate::foundations::{func, Cast};
use crate::loading::{DataSource, Load, Readable};

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
    let loaded = path.map(DataSource::Path).load(engine.world)?;
    Ok(match encoding {
        None => Readable::Bytes(loaded.data),
        Some(Encoding::Utf8) => Readable::Str(loaded.load_str()?.into()),
    })
}

/// An encoding of a file.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Cast)]
pub enum Encoding {
    /// The Unicode UTF-8 encoding.
    Utf8,
}
