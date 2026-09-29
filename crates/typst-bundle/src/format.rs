use typst_library::Feature;
use typst_library::diag::{SourceResult, bail};
use typst_library::engine::Engine;
use typst_library::format::{Format, FormatElement};
use typst_library::foundations::{
    Args, BundlePath, Bytes, Construct, Content, NativeRuleMap, ShowFn, Str, Target,
    cast, elem, scope,
};
use typst_library::model::DocumentElem;

/// The format element for registering the bundle format.
pub const FORMAT: Format = Format::new::<BundleFormat>()
    .with_feature(Feature::Bundle)
    .with_rules(register);

/// Registers show rules for bundle specific elements.
pub fn register(rules: &mut NativeRuleMap) {
    for target in [Target::Paged, Target::Html] {
        rules.replace(target, DOCUMENT_UNSUPPORTED_RULE);
        rules.register(target, ASSET_UNSUPPORTED_RULE);
    }
}

/// Typst's bundle export format (experimental).
///
/// #info[
///   Bundle export is only available for experimentation behind a feature flag.
///   Do not use this feature for production use cases. In the CLI, you can
///   experiment with it by passing `--features bundle` or setting the
///   `TYPST_FEATURES` environment variable to `bundle`. To use both the
///   `bundle` and the `html` feature at the same time, specify both separated
///   with a comma (i.e. `bundle,html`). In the web app, bundle export is not
///   available at this time.
/// ]
///
/// With Typst's bundle export, you can emit multiple output files from a single
/// Typst project. Bundle output is useful for creating multi-page websites with
/// HTML export, but it is not limited to HTML export. You can create bundles
/// containing any combination of @html[HTML pages], @pdf[PDFs],
/// @format.png[PNGs], @format.svg[SVGs], and arbitrary @asset[assets].
///
/// = Exporting as a bundle <exporting-as-a-bundle>
/// == Command Line <command-line>
/// Pass `--format bundle` to the `compile` or `watch` subcommand. Note that you
/// must also pass `--features bundle` or set `TYPST_FEATURES=bundle` to enable
/// this experimental export target.
///
/// When using `typst watch`, Typst will launch a live-reloading HTTP server
/// serving your files. You can configure it as follows:
///
/// - Pass `--port` to change the port. (Defaults to the first free port in the
///   range 3000-3005.)
/// - Pass `--no-reload` to disable injection of a live reload script into HTML
///   pages. (The HTML that is written to disk isn't affected either way.)
///   Non-HTML documents do not support live reload.
/// - Pass `--no-serve` to disable the server altogether.
///
/// == Web App <web-app>
/// Not currently available.
///
/// = Creating files <creating-files>
/// A bundle is a collection of files. Each of these bundle files falls into one
/// of two categories: Document or asset. A @document takes @content[content]
/// that is exported with one of Typst's other export formats. Meanwhile, an
/// @asset takes raw @bytes[byte data] of your choice that will be written to
/// disk as-is. Both elements take the desired output path as their first
/// argument.
///
/// The example below shows a basic example of how bundle export could be used
/// in practice:
///
/// ```typ
/// #document("index.html", title: [Home])[
///   #title()
///   - #link(<blog>)[Go to blog]
/// ]
///
/// #document("blog.html", title: [Blog])[
///   #title()
///   Welcome to my blog!
///
///   ...
///
///   This blog also exists as a
///   #link(<blog-pdf>)[single PDF].
/// ] <blog>
///
/// #document("blog.pdf", title: [Blog])[
///   ...
/// ] <blog-pdf>
///
/// #asset(
///   "favicon.ico",
///   read("images/favicon.ico", encoding: none),
/// )
/// ```
///
/// In the example, we create two HTML documents: A home page and a blog. The
/// home page links to the blog through a label link. Typst's built-in linking
/// mechanism natively supports @link:links-in-bundle-export[cross-document
/// links] and resolves the correct relative paths for you. The bundle also
/// contains a PDF version of the blog, which is linked from the HTML version.
/// In practice, you could now share the content between the HTML and PDF
/// version by storing it in a variable and using it in both. This is omitted
/// here for brevity. Finally, the bundle contains an icon asset for the
/// website. In this case, we're providing the asset's data by reading a file
/// from disk. Alternatively, it's also possible to generate asset data from
/// within Typst (e.g. via a function like @json.encode).
///
/// Documents and assets are normal elements, so you can use them with Typst's
/// usual scripting, styling, and introspection mechanisms. For more details,
/// refer to the @document and @asset documentation.
///
/// = Introspection <introspection>
/// Introspections always observe the full bundle rather than individual
/// documents. For instance, querying for headings will give you all headings in
/// all documents rather than the ones in the current document. Similarly,
/// labels are global to the bundle—you can locate and
/// @link:links-in-bundle-export[link to labels in other documents]. Counters
/// and states are likewise global. #footnote[An exception to this forms the
/// page counter, which is (naturally) per document.] In particular, if you
/// enable things like heading numbering, the numbering will progress
/// consecutively across the full bundle.
///
/// If you're using bundle export to build one conceptual work that is split up
/// across multiple output files, this is typically what you want. If, however,
/// you're using bundle export to export multiple conceptually separate works,
/// you might want introspections to consider each document in isolation.
/// Currently, you'll have to do this manually (e.g. by resetting counters,
/// adjusting selectors, etc.) We
/// #link("https://github.com/typst/typst/issues/7735#issuecomment-3908841853")[plan
/// to provide more tools] for managing the precise scope of introspection in
/// the future.
#[elem(scope, name = "bundle", since = "unreleased", Construct)]
pub struct BundleFormat {}

impl Construct for BundleFormat {
    fn construct(_: &mut Engine, args: &mut Args) -> SourceResult<Content> {
        bail!(args.span, "cannot be constructed manually")
    }
}

impl FormatElement for BundleFormat {
    type Options = ();
}

#[scope(category = Bundle)]
impl BundleFormat {
    #[elem]
    type AssetElem;
}

pub const DOCUMENT_UNSUPPORTED_RULE: ShowFn<DocumentElem> = |elem, _, _| {
    bail!(
        elem.span(),
        "constructing a document is only supported in the bundle target";
        // TODO: Support for CLI-specific hints would be nice.
        hint: "try enabling the bundle target";
        hint: "or use a `set document(..)` rule to configure metadata";
    )
};

/// Adds a custom file to a bundle.
///
/// This function creates a single file in a @format.bundle[bundle], from
/// @bytes[raw byte data]. Unlike @document[documents], assets will be emitted
/// as-is without undergoing compilation.
///
/// The `asset` function can be combined with @read to copy a file from the
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
/// directly or use a function like @json.encode to emit serialized data.
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
///
/// ```json
/// {
///   "count": 2
/// }
/// ```
///
/// This function is defined on the @format.bundle element, but is also
/// available directly in the global scope. It may only be used in the
/// @format.bundle[bundle] target.
#[elem(since = "0.15.0", Locatable)]
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
