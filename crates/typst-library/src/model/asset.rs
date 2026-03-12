use typst_macros::elem;

use crate::diag::bail;
use crate::foundations::{BundlePath, Bytes, ShowFn, Str, cast};
use crate::introspection::Locatable;

/// Adds a custom file to a bundle.
///
/// This function creates a single file in a [bundle], from [raw byte
/// data]($bytes). Unlike [documents]($document), assets will be emitted as-is
/// without undergoing compilation.
///
/// The `asset` function can be combined with [`read`] to copy a file from the
/// project into the output bundle. The first argument to `asset` defines the
/// output path for the asset in the bundle, while the path passed to `read`
/// defines where in the project to read the data from.
///
/// ```typ
/// // Copy the file `styles.css` into the bundle.
/// #asset("styles.css", read("styles.css"))
/// ```
///
/// That said, `asset` is not tied to `read`. You can also generate bytes
/// directly or use a function like [`json.encode`] to emit serialized data.
///
/// ```typ
/// // Emits a JSON file with the number
/// // of headings in the document.
/// #context {
///   let headings = query(heading)
///   let meta = (
///     count: headings.len(),
///   )
///   asset("meta.json", json.encode(meta))
/// }
///
/// #document("doc.pdf")[
///   = Introduction
///   = Conclusion
/// ]
/// ```
///
/// This would emit a `meta.json` file with the following contents into the
/// resulting bundle:
/// ```json
/// {
///   "count": 2
/// }
/// ```
///
/// This function may only be used in the [bundle] target.
#[elem(Locatable)]
pub struct AssetElem {
    /// The path in the bundle at which the asset will be placed.
    ///
    /// May contain interior slashes, in which case intermediate directories
    /// will be automatically created.
    #[required]
    pub path: BundlePath,

    /// The raw data that will be written into the file at the specified path.
    ///
    /// If a string is given, it will be encoded using UTF-8.
    #[required]
    pub data: AssetData,
}

pub const ASSET_UNSUPPORTED_RULE: ShowFn<AssetElem> = |elem, _, _| {
    bail!(
        elem.span(),
        "assets are only supported in the bundle target";
        // TODO: Support for CLI-specific hints would be nice.
        hint: "try enabling the bundle target";
    )
};

/// The raw data for an asset.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct AssetData(pub Bytes);

cast! {
    AssetData,
    self => self.0.into_value(),
    v: Str => Self(Bytes::from_string(v)),
    v: Bytes => Self(v),
}
