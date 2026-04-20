use std::path::PathBuf;
use std::sync::{Arc, LazyLock};

use az::SaturatingAs;
use comemo::{Track, TrackedMut};
use ecow::{EcoString, eco_format};
use typst::diag::{At, FileError, FileResult, SourceResult, StrResult, bail};
use typst::engine::Engine;
use typst::foundations::{
    Binding, Bytes, Context, Datetime, Dict, Duration, IntoValue, Label,
    LocatableSelector, Module, NativeElement, PathOrStr, Repr, Scope, Selector, ShowFn,
    Str, Target, Value, array, func,
};
use typst::introspection::{EmptyIntrospector, MetadataElem};
use typst::model::{Destination, EarlyLinkResolver, LinkElem, ResolvedLink};
use typst::routines::SpanMode;
use typst::syntax::{
    FileId, RangeMapper, RootedPath, Source, Spanned, SyntaxMode, VirtualPath,
    VirtualRoot,
};
use typst::text::{Font, FontBook};
use typst::visualize::ImageElem;
use typst::{Features, Library, LibraryExt, World};
use typst_html::{HtmlAttrs, HtmlElem, attr, tag};
use typst_kit::datetime::Time;
use typst_kit::diagnostics::DiagnosticWorld;
use typst_kit::files::{FileLoader, FileStore, FsRoot};
use typst_utils::{LazyHash, PicoStr};

use crate::example::FRAME_RULE;
use crate::live::RangePair;

/// A world for docs compilation.
pub struct DocWorld {
    /// The entrypoint file.
    main: FileId,
    /// Maps file ids to source files and buffers.
    files: FileStore<DocsFiles>,
    /// The current datetime if requested.
    now: Time,
}

impl DocWorld {
    /// Creates a new world for docs compilation, with the given root directory
    /// and entrypoint file.
    pub fn new(root: &str, entrypoint: &str) -> Self {
        Self {
            main: RootedPath::new(
                VirtualRoot::Project,
                VirtualPath::new(entrypoint).unwrap(),
            )
            .intern(),
            files: FileStore::new(DocsFiles(FsRoot::new(PathBuf::from(root)))),
            now: Time::system(),
        }
    }

    /// Return all paths the last compilation depended on.
    pub fn dependencies(&mut self) -> impl Iterator<Item = PathBuf> + '_ {
        let (loader, deps) = self.files.dependencies();
        deps.filter_map(|id| loader.resolve(id).ok())
    }

    /// Reset the compilation state in preparation of a new compilation.
    pub fn reset(&mut self) {
        self.files.reset();
        self.now.reset();
    }
}

impl World for DocWorld {
    fn library(&self) -> &LazyHash<Library> {
        &DOCS_LIBRARY
    }

    fn book(&self) -> &LazyHash<FontBook> {
        &FONTS.0
    }

    fn main(&self) -> FileId {
        self.main
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        self.files.source(id)
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        self.files.file(id)
    }

    fn font(&self, index: usize) -> Option<Font> {
        FONTS.1.get(index).cloned()
    }

    fn today(&self, offset: Option<Duration>) -> Option<Datetime> {
        self.now.today(offset)
    }
}

impl DiagnosticWorld for DocWorld {
    fn name(&self, id: FileId) -> String {
        let vpath = id.vpath();
        match id.root() {
            VirtualRoot::Project => vpath.get_without_slash().into(),
            VirtualRoot::Package(package) => {
                format!("{package}{}", vpath.get_with_slash())
            }
        }
    }
}

/// Provides project files from a configured directory and no packages.
struct DocsFiles(pub FsRoot);

impl DocsFiles {
    fn resolve(&self, id: FileId) -> FileResult<PathBuf> {
        Ok(self.root(id)?.resolve(id.vpath()))
    }

    fn root(&self, id: FileId) -> FileResult<FsRoot> {
        match id.root() {
            VirtualRoot::Project => Ok(self.0.clone()),
            VirtualRoot::Package(_) => {
                Err(FileError::NotFound(id.vpath().get_without_slash().into()))
            }
        }
    }
}

impl FileLoader for DocsFiles {
    fn load(&self, id: FileId) -> FileResult<Bytes> {
        match id.root() {
            VirtualRoot::Project => self.0.load(id.vpath()),
            VirtualRoot::Package(_) => {
                Err(FileError::NotFound(id.vpath().get_without_slash().into()))
            }
        }
    }
}

/// The fonts available to docs compilation.
pub static FONTS: LazyLock<(LazyHash<FontBook>, Vec<Font>)> = LazyLock::new(|| {
    let fonts: Vec<_> = typst_assets::fonts()
        .chain(typst_dev_assets::fonts())
        .flat_map(|data| Font::iter(Bytes::new(data)))
        .collect();
    let book = FontBook::from_fonts(&fonts);
    (LazyHash::new(book), fonts)
});

/// A standard library that is extended for docs compilation. Includes
/// - a `docs` module with various utilities,
/// - a few patched show rules,
/// - `sys.inputs` holding contributor data for the changelog
static DOCS_LIBRARY: LazyLock<LazyHash<Library>> = LazyLock::new(|| {
    let mut lib = Library::builder().with_features(Features::all()).build();
    let scope = lib.global.scope_mut();
    scope.define("docs", docs_module());
    lib.rules.replace(Target::Html, PATCHED_LINK_RULE);
    lib.rules.replace(Target::Html, PATCHED_IMAGE_RULE);
    lib.rules.register(Target::Paged, FRAME_RULE);
    LazyHash::new(lib)
});

/// A module with various utilities for the docs.
fn docs_module() -> Module {
    let mut scope = Scope::new();
    scope.define_func::<read_dev_asset>();
    scope.define_func::<selector_within>();
    scope.define_func::<eval_mapped>();
    scope.define_func::<crate::live::docs_in_source>();
    scope.define_func::<crate::example::compile_example>();
    scope.define_func::<crate::reflect::describe>();
    scope.define_func::<crate::reflect::binding>();
    scope.define_func::<crate::reflect::math_class>();
    scope.define_func::<crate::reflect::is_accent>();
    scope.define_func::<crate::reflect::unicode_name>();
    scope.define_func::<crate::reflect::latex_name>();
    scope.define_func::<crate::reflect::is_global_html_attr>();
    scope.define("shorthands", crate::reflect::shorthands());
    Module::new("docs", scope)
}

/// Loads an asset from the `typst_dev_assets` crate by file name.
#[func]
fn read_dev_asset(filename: EcoString) -> StrResult<Bytes> {
    typst_dev_assets::get_by_name(&filename)
        .ok_or_else(|| eco_format!("asset not found: {}", filename.repr()))
        .map(Bytes::new)
}

/// This exists just because the within selector is not yet publicly exposed.
/// It can be removed once it is.
#[func]
fn selector_within(selector: LocatableSelector, ancestor: LocatableSelector) -> Selector {
    Selector::Within {
        selector: Arc::new(selector.0),
        ancestor: Arc::new(ancestor.0),
    }
}

/// Evaluates a string of Typst markup with mapped spans.
///
/// This makes it possible to evaluate Typst markup in Rust doc comments and to
/// then receive precise diagnostics in these Rust source files.
#[func]
fn eval_mapped(
    engine: &mut Engine,
    /// The source markup.
    text: Str,
    /// The file with which to associate the source text.
    path: Spanned<PathOrStr>,
    /// The ranges with which to associate the source text. Each entry is a pair
    /// that describes where in the `path` file a specific segment of the `text`
    /// is. The segments defined by the ranges are consecutive pieces of `text`.
    /// The sum of all `end - start` in `ranges` is the length of the `text`.
    ranges: Vec<RangePair>,
    /// The syntactical mode in which the string is parsed.
    #[named]
    #[default(SyntaxMode::Code)]
    mode: SyntaxMode,
    /// A scope of definitions that are made available.
    #[named]
    #[default]
    scope: Dict,
) -> SourceResult<Value> {
    let dict = scope;
    let mut scope = Scope::new();
    for (key, value) in dict {
        scope.bind(key.into(), Binding::new(value, path.span));
    }

    let id = path.v.resolve_if_some(path.span.id()).at(path.span)?.intern();
    let mapper = RangeMapper::new(ranges.into_iter().map(|p| p.0));

    typst_eval::eval_string(
        engine.routines,
        engine.world,
        TrackedMut::reborrow_mut(&mut engine.sink),
        EmptyIntrospector.track(),
        Context::none().track(),
        &text,
        SpanMode::Mapped { id, mapper: &mapper },
        mode,
        scope,
    )
}

// HTML export currently always writes relative links. However, we want to
// write absolute links _and_ also avoid the explicit `index.html` in the
// path. Until there is a better built-in support for configuring these
// kinds of details, we override the built-in show rule for links.
const PATCHED_LINK_RULE: ShowFn<LinkElem> = |elem, engine, _| {
    let span = elem.span();
    let dest = elem.dest.resolve_early(engine, span)?;

    let href = match dest {
        Destination::Url(url) => Some(url.clone().into_inner()),
        Destination::Position(_) => {
            bail!(elem.span(), "positional links are not supported")
        }
        Destination::Location(location) => Some({
            let resolved = EarlyLinkResolver::new(elem.location().unwrap(), span)
                .resolve(engine, location)
                .at(span)?;
            match resolved {
                ResolvedLink::Local { anchor } => eco_format!("#{anchor}"),
                ResolvedLink::Cross { from: _, to, anchor } => {
                    let path = to
                        .get_with_slash()
                        .strip_suffix("index.html")
                        .ok_or("expected path to end with index.html")
                        .at(span)?;
                    if anchor.is_empty() {
                        path.into()
                    } else {
                        eco_format!("{path}#{anchor}")
                    }
                }
            }
        }),
    };

    Ok(HtmlElem::new(tag::a)
        .with_optional_attr(attr::href, href)
        .with_body(Some(elem.body.clone()))
        .pack())
};

/// HTML export currently always inlines images as base64. We want to avoid
/// this. Until there is a better built-in support for configuring these kinds
/// of details, we override the built-in show rule for images.
///
/// This rule emits `metadata` labelled `<metadata-asset>` holding (path, data)
/// pairs. This metadata is queried by the asset handling code to produce
/// real asset. The paths are auto-generated based on hashes of the images.
const PATCHED_IMAGE_RULE: ShowFn<ImageElem> = |elem, engine, styles| {
    fn encode_hash(hash: u128) -> String {
        use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
        URL_SAFE_NO_PAD.encode(hash.to_be_bytes())
    }

    let image = elem.decode(engine, styles)?;

    let web_image = typst_svg::WebImage::new(&image);
    let hash = typst_utils::hash128(&web_image.data);
    let path = eco_format!(
        "/assets/images/{}.{}",
        encode_hash(hash),
        web_image.format.extension()
    );
    let label = Label::new(PicoStr::intern("metadata-asset")).unwrap();
    let meta = MetadataElem::new(array![path.clone(), web_image.data].into_value())
        .pack()
        .labelled(label);

    let mut attrs = HtmlAttrs::new();
    attrs.push(attr::src, path);

    if let Some(alt) = elem.alt.get_cloned(styles) {
        attrs.push(attr::alt, alt);
    }

    let cast = |v: f64| eco_format!("{}", v.round().saturating_as::<i64>());
    attrs.push(attr::width, cast(image.width()));
    attrs.push(attr::height, cast(image.height()));

    // We're omitting handling of the CSS properties here because the relevant
    // code in `typst_html` is private. But the docs also don't really need it.
    let img = HtmlElem::new(tag::img).with_attrs(attrs).pack();

    Ok(meta + img)
};
