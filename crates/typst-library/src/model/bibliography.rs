use std::any::TypeId;
use std::fmt::{self, Debug, Formatter};
use std::num::NonZeroUsize;
use std::path::Path;
use std::sync::{Arc, LazyLock};

use comemo::{Track, Tracked, TrackedMut};
use ecow::{EcoString, EcoVec, eco_format, eco_vec};
use hayagriva::archive::ArchivedStyle;
use hayagriva::io::BibLaTeXError;
use hayagriva::{
    BibliographyDriver, BibliographyRequest, CitationItem, CitationRequest, Library,
    SpecificLocator, TransparentLocator, citationberg,
};
use indexmap::IndexMap;
use rustc_hash::{FxBuildHasher, FxHashMap};
use smallvec::SmallVec;
use typst_syntax::{Span, Spanned, SyntaxMode};
use typst_utils::{
    LazyHash, ManuallyHash, NonZeroExt, PicoStr, Protected, ResolvedPicoStr,
};

use crate::World;
use crate::diag::{
    At, HintedStrResult, HintedString, LoadError, LoadResult, LoadedWithin,
    ReportTextPos, SourceDiagnostic, SourceResult, StrResult, bail, error, warning,
};
use crate::engine::{Engine, Route, Sink, Traced};
use crate::foundations::{
    Bytes, CastInfo, Content, Context, Derived, FromValue, IntoValue, Label,
    LocatableSelector, NativeElement, OneOrMultiple, Packed, Reflect, Repr, Scope,
    Selector, ShowSet, Smart, StyleChain, Styles, Synthesize, Value, elem,
};
use crate::introspection::{
    EmptyIntrospector, History, Introspect, Introspector, Location, QueryIntrospection,
};
use crate::layout::{BlockElem, Em, HElem, PadElem};
use crate::loading::{DataSource, Load, LoadSource, Loaded, format_yaml_error};
use crate::model::{
    CitationForm, CiteElem, CiteGroup, Destination, DirectLinkElem, FootnoteElem,
    HeadingElem, LinkElem, Url,
};
use crate::routines::SpanMode;
use crate::text::{Lang, LocalName, Region, SmallcapsElem, SubElem, SuperElem, TextElem};

/// A bibliography / reference listing.
///
/// You can create a new bibliography by calling this function with a path to a
/// bibliography file in either one of two formats:
///
/// - A Hayagriva `.yaml`/`.yml` file. Hayagriva is a new bibliography file
///   format designed for use with Typst. Visit its
///   #link("https://github.com/typst/hayagriva/blob/main/docs/file-format.md")[documentation]
///   for more details.
/// - A BibLaTeX `.bib` file.
///
/// As soon as you add a bibliography somewhere in your document, you can start
/// citing things with reference syntax (`[@key]`) or explicit calls to the
/// @cite[citation] function (`[#cite(<key>)]`). The bibliography will only show
/// entries for works that were referenced in the document.
///
/// = Example <example>
/// ```example
/// This was already noted by
/// pirates long ago. @arrgh
///
/// Multiple sources say ...
/// @arrgh @netwok.
///
/// #bibliography("works.bib")
/// ```
///
/// = Styles <styles>
/// Typst offers a wide selection of built-in
/// @bibliography.style[citation and bibliography styles]. Beyond those, you can
/// add and use custom #link("https://citationstyles.org/")[CSL] (Citation Style
/// Language) files. Wondering which style to use? Here are some good defaults
/// based on what discipline you're working in:
///
/// #docs-table(
///   table.header[Fields][Typical Styles],
///
///   [Engineering, IT],
///   [`{"ieee"}`],
///
///   [Psychology, Life Sciences],
///   [`{"apa"}`],
///
///   [Social sciences],
///   [`{"chicago-author-date"}`],
///
///   [Humanities],
///   [`{"mla"}`, `{"chicago-notes"}`, `{"harvard-cite-them-right"}`],
///
///   [Economics],
///   [`{"harvard-cite-them-right"}`],
///
///   [Physics],
///   [`{"american-physics-society"}`],
/// )
///
/// = Multiple bibliographies <multiple-bibliographies>
/// When a Typst document contains multiple bibliographies, each citation is
/// assigned to one of them. By default, Typst will automatically pick a
/// suitable bibliography (typically, the closest following one that contains
/// the referenced citation key). This covers common cases like by-chapter or
/// thematic bibliographies. For more fine-grained control, citations can be
/// explicitly targeted by a bibliography through a
/// @bibliography.target[`target`] selector.
#[elem(Locatable, Synthesize, ShowSet, LocalName)]
pub struct BibliographyElem {
    /// One or multiple paths to or raw bytes for Hayagriva `.yaml` and/or
    /// BibLaTeX `.bib` files.
    ///
    /// This can be a:
    /// - A path string or @path to load a bibliography file from.
    /// - Raw bytes from which the bibliography should be decoded.
    /// - An array where each item is one of the above.
    #[required]
    #[parse(
        let sources = args.expect("sources")?;
        Bibliography::load(engine.world, sources)?
    )]
    pub sources: Derived<OneOrMultiple<DataSource>, Bibliography>,

    /// The title of the bibliography.
    ///
    /// - When set to `{auto}`, an appropriate title for the
    ///   @text.lang[text language] will be used. This is the default.
    /// - When set to `{none}`, the bibliography will not have a title.
    /// - A custom title can be set by passing content.
    ///
    /// The bibliography's heading will not be numbered by default, but you can
    /// force it to be with a show-set rule:
    /// `{show bibliography: set heading(numbering: "1.")}`
    pub title: Smart<Option<Content>>,

    /// Whether to include all works from the given bibliography files, even
    /// those that weren't cited in the document.
    ///
    /// To selectively add individual cited works without showing them, you can
    /// also use the `cite` function with @cite.form[`form`] set to `{none}`.
    #[default(false)]
    pub full: bool,

    /// The bibliography style.
    ///
    /// This can be:
    /// - A string with the name of one of the built-in styles (see below). Some
    ///   of the styles listed below appear twice, once with their full name and
    ///   once with a short alias.
    /// - A path string or @path to a
    ///   #link("https://citationstyles.org/")[CSL file].
    /// - Raw bytes from which a CSL style should be decoded.
    #[parse(match args.named::<Spanned<CslSource>>("style")? {
        Some(source) => Some(CslStyle::load(engine, source)?),
        None => None,
    })]
    #[default({
        let default = ArchivedStyle::InstituteOfElectricalAndElectronicsEngineers;
        Derived::new(CslSource::Named(default, None), CslStyle::from_archived(default))
    })]
    pub style: Derived<CslSource, CslStyle>,

    /// Defines which citations to include in the bibliography.
    ///
    /// Typst will automatically assign each citation in the document to a
    /// bibliography. Concretely, a citation will be assigned to (in order of
    /// precedence)
    /// + the first bibliography that includes it in its `target` selector; or
    ///   if no such bibliography exists
    /// + the closest _following_ bibliography with `{target: auto}` that
    ///   contains its key; or if no such bibliography follows
    /// + the closest _preceding_ bibliography with `{target: auto}` that
    ///   contains its key.
    ///
    /// #example(
    ///   title: [Local bibliography],
    ///   ```
    ///   #let info(body) = block(
    ///     stroke: (left: 1.5pt + blue),
    ///     fill: aqua.lighten(50%),
    ///     inset: 1em,
    ///     context {
    ///       body
    ///       show divider: set block(spacing: 1.2em)
    ///       divider()
    ///       bibliography(
    ///         "works.bib",
    ///         title: none,
    ///         target: selector(cite).within(here()),
    ///         style: "mla",
    ///       )
    ///     }
    ///   )
    ///
    ///   = On the matter of dumplings
    ///   In recent years, we can observe an uptick in
    ///   dumpling consumption across the board. @netwok
    ///
    ///   #info[
    ///     Dumplings are particularly enjoyed
    ///     among pirates. @arrgh
    ///   ]
    ///
    ///   #bibliography("works.bib")
    ///   ```
    /// )
    pub target: Smart<LocatableSelector>,

    /// Conceptually groups this bibliography with other bibliographies for
    /// numbering purposes. Bibliographies in the same group will assign
    /// consecutive citation numbers.
    ///
    /// This can be:
    /// - `{none}`: The bibliography will be numbered in isolation.
    /// - `{auto}`: The bibliography will be consecutively numbered with all
    ///   other bibliographies in the `{auto}` group.
    /// - A @str[string]: The bibliography will be consecutively numbered with
    ///   all other bibliographies with the same `group` value.
    ///
    /// The `{auto}` group works just like any string group, but it is the
    /// canonical default group.
    ///
    /// #example(
    ///   title: [Consecutive citation numbers],
    ///   ```
    ///   #show bibliography: set heading(
    ///     offset: 1,
    ///   )
    ///
    ///   = First part
    ///   Starts at one: @netwok @arrgh
    ///   #bibliography(
    ///     "works.bib",
    ///     style: "ieee",
    ///   )
    ///
    ///   = Second part
    ///   Continues with three: @distress
    ///   #bibliography(
    ///     "works.bib",
    ///     style: "nlm-citation-sequence",
    ///   )
    ///   ```
    /// )
    ///
    /// #example(
    ///   title: [Separate citation numbers],
    ///   ```
    ///   #show bibliography: set heading(
    ///     offset: 1,
    ///   )
    ///   #set bibliography(group: none)
    ///
    ///   = First part
    ///   Starts at one: @netwok @arrgh
    ///   #bibliography(
    ///     "works.bib",
    ///     style: "ieee",
    ///   )
    ///
    ///   = Second part
    ///   Resets to one: @distress
    ///   #bibliography(
    ///     "works.bib",
    ///     style: "nlm-citation-sequence",
    ///   )
    ///   ```
    /// )
    #[default(Some(Smart::Auto))]
    pub group: Option<Smart<EcoString>>,

    /// The language setting where the bibliography is.
    #[internal]
    #[synthesized]
    pub lang: Lang,

    /// The region setting where the bibliography is.
    #[internal]
    #[synthesized]
    pub region: Option<Region>,
}

impl BibliographyElem {
    /// Whether any bibliography contains the given key.
    pub fn has(engine: &mut Engine, key: Label, span: Span) -> bool {
        engine
            .introspect(QueryIntrospection(Self::ELEM.select(), span))
            .iter()
            .any(|elem| elem.to_packed::<Self>().unwrap().sources.derived.has(key))
    }

    /// Find all bibliography keys.
    pub fn keys(
        introspector: Tracked<dyn Introspector + '_>,
    ) -> Vec<(Label, Option<EcoString>)> {
        let mut vec = vec![];
        for elem in introspector.query(&Self::ELEM.select()).iter() {
            let this = elem.to_packed::<Self>().unwrap();
            for (key, entry) in this.sources.derived.iter() {
                let detail = entry.title().map(|title| title.value.to_str().into());
                vec.push((key, detail))
            }
        }
        vec
    }
}

impl Packed<BibliographyElem> {
    /// Produces the heading for the bibliography, if any.
    pub fn realize_title(&self, styles: StyleChain) -> Option<Content> {
        self.title
            .get_cloned(styles)
            .unwrap_or_else(|| {
                Some(TextElem::packed(Packed::<BibliographyElem>::local_name_in(styles)))
            })
            .map(|title| {
                HeadingElem::new(title)
                    .with_depth(NonZeroUsize::ONE)
                    .pack()
                    .spanned(self.span())
            })
    }
}

impl Synthesize for Packed<BibliographyElem> {
    fn synthesize(&mut self, _: &mut Engine, styles: StyleChain) -> SourceResult<()> {
        let elem = self.as_mut();
        elem.lang = Some(styles.get(TextElem::lang));
        elem.region = Some(styles.get(TextElem::region));
        Ok(())
    }
}

impl ShowSet for Packed<BibliographyElem> {
    fn show_set(&self, _: StyleChain) -> Styles {
        const INDENT: Em = Em::new(1.0);
        let mut out = Styles::new();
        out.set(HeadingElem::numbering, None);
        out.set(PadElem::left, INDENT.into());
        out
    }
}

impl LocalName for Packed<BibliographyElem> {
    const KEY: &'static str = "bibliography";
}

/// A loaded bibliography.
#[derive(Clone, PartialEq, Hash)]
pub struct Bibliography(
    Arc<ManuallyHash<IndexMap<Label, hayagriva::Entry, FxBuildHasher>>>,
);

impl Bibliography {
    /// Load a bibliography from data sources.
    fn load(
        world: Tracked<dyn World + '_>,
        sources: Spanned<OneOrMultiple<DataSource>>,
    ) -> SourceResult<Derived<OneOrMultiple<DataSource>, Self>> {
        let loaded = sources.load(world)?;
        let bibliography = Self::decode(&loaded)?;
        Ok(Derived::new(sources.v, bibliography))
    }

    /// Decode a bibliography from loaded data sources.
    #[comemo::memoize]
    #[typst_macros::time(name = "load bibliography")]
    fn decode(data: &[Loaded]) -> SourceResult<Bibliography> {
        let mut map = IndexMap::default();
        let mut duplicates = Vec::<EcoString>::new();

        // We might have multiple bib/yaml files
        for d in data.iter() {
            let library = decode_library(d)?;
            for entry in library {
                let label = Label::new(PicoStr::intern(entry.key()))
                    .ok_or("bibliography contains entry with empty key")
                    .at(d.source.span)?;

                match map.entry(label) {
                    indexmap::map::Entry::Vacant(vacant) => {
                        vacant.insert(entry);
                    }
                    indexmap::map::Entry::Occupied(_) => {
                        duplicates.push(entry.key().into());
                    }
                }
            }
        }

        if !duplicates.is_empty() {
            // TODO: Store spans of entries for duplicate key error messages.
            // Requires hayagriva entries to store their location, which should
            // be fine, since they are 1kb anyway.
            let span = data.first().unwrap().source.span;
            bail!(span, "duplicate bibliography keys: {}", duplicates.join(", "));
        }

        Ok(Bibliography(Arc::new(ManuallyHash::new(map, typst_utils::hash128(data)))))
    }

    fn has(&self, key: Label) -> bool {
        self.0.contains_key(&key)
    }

    fn get(&self, key: Label) -> Option<&hayagriva::Entry> {
        self.0.get(&key)
    }

    fn iter(&self) -> impl Iterator<Item = (Label, &hayagriva::Entry)> {
        self.0.iter().map(|(&k, v)| (k, v))
    }
}

impl Debug for Bibliography {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_set().entries(self.0.keys()).finish()
    }
}

/// Decode on library from one data source.
fn decode_library(loaded: &Loaded) -> SourceResult<Library> {
    let data = loaded.data.as_str().within(loaded)?;

    if let LoadSource::Path(file_id) = loaded.source.v {
        // If we got a path, use the extension to determine whether it is
        // YAML or BibLaTeX.
        let ext = file_id.vpath().extension().unwrap_or_default();
        match ext.to_lowercase().as_str() {
            "yml" | "yaml" => hayagriva::io::from_yaml_str(data)
                .map_err(format_yaml_error)
                .within(loaded),
            "bib" => hayagriva::io::from_biblatex_str(data)
                .map_err(format_biblatex_error)
                .within(loaded),
            _ => bail!(
                loaded.source.span,
                "unknown bibliography format (must be .yaml/.yml or .bib)"
            ),
        }
    } else {
        // If we just got bytes, we need to guess. If it can be decoded as
        // hayagriva YAML, we'll use that.
        let haya_err = match hayagriva::io::from_yaml_str(data) {
            Ok(library) => return Ok(library),
            Err(err) => err,
        };

        // If it can be decoded as BibLaTeX, we use that instead.
        let bib_errs = match hayagriva::io::from_biblatex_str(data) {
            // If the file is almost valid yaml, but contains no `@` character
            // it will be successfully parsed as an empty BibLaTeX library,
            // since BibLaTeX does support arbitrary text outside of entries.
            Ok(library) if !library.is_empty() => return Ok(library),
            Ok(_) => None,
            Err(err) => Some(err),
        };

        // If neither decoded correctly, check whether `:` or `{` appears
        // more often to guess whether it's more likely to be YAML or BibLaTeX
        // and emit the more appropriate error.
        let mut yaml = 0;
        let mut biblatex = 0;
        for c in data.chars() {
            match c {
                ':' => yaml += 1,
                '{' => biblatex += 1,
                _ => {}
            }
        }

        match bib_errs {
            Some(bib_errs) if biblatex >= yaml => {
                Err(format_biblatex_error(bib_errs)).within(loaded)
            }
            _ => Err(format_yaml_error(haya_err)).within(loaded),
        }
    }
}

/// Format a BibLaTeX loading error.
fn format_biblatex_error(errors: Vec<BibLaTeXError>) -> LoadError {
    // TODO: return multiple errors?
    let Some(error) = errors.into_iter().next() else {
        // TODO: can this even happen, should we just unwrap?
        return LoadError::text(
            ReportTextPos::None,
            "failed to parse BibLaTeX",
            "something went wrong",
        );
    };

    let (range, msg) = match error {
        BibLaTeXError::Parse(error) => (error.span, error.kind.to_string()),
        BibLaTeXError::Type(error) => (error.span, error.kind.to_string()),
    };

    LoadError::text(range, "failed to parse BibLaTeX", msg)
}

/// A loaded CSL style.
#[derive(Debug, Clone, PartialEq, Hash)]
pub struct CslStyle(Arc<ManuallyHash<citationberg::IndependentStyle>>);

impl CslStyle {
    /// Load a CSL style from a data source.
    pub fn load(
        engine: &mut Engine,
        Spanned { v: source, span }: Spanned<CslSource>,
    ) -> SourceResult<Derived<CslSource, Self>> {
        let style = match &source {
            CslSource::Named(style, deprecation) => {
                if let Some(message) = deprecation {
                    engine.sink.warn(SourceDiagnostic::warning(span, message.clone()));
                }
                Self::from_archived(*style)
            }
            CslSource::Normal(source) => {
                let loaded = Spanned::new(source, span).load(engine.world)?;
                Self::from_data(&loaded.data).within(&loaded)?
            }
        };
        Ok(Derived::new(source, style))
    }

    /// Load a built-in CSL style.
    #[comemo::memoize]
    pub fn from_archived(archived: ArchivedStyle) -> CslStyle {
        match archived.get() {
            citationberg::Style::Independent(style) => Self(Arc::new(ManuallyHash::new(
                style,
                typst_utils::hash128(&(TypeId::of::<ArchivedStyle>(), archived)),
            ))),
            // Ensured by `test_bibliography_load_builtin_styles`.
            _ => unreachable!("archive should not contain dependent styles"),
        }
    }

    /// Load a CSL style from file contents.
    #[comemo::memoize]
    pub fn from_data(bytes: &Bytes) -> LoadResult<CslStyle> {
        let text = bytes.as_str()?;
        citationberg::IndependentStyle::from_xml(text)
            .map(|style| {
                Self(Arc::new(ManuallyHash::new(
                    style,
                    typst_utils::hash128(&(TypeId::of::<Bytes>(), bytes)),
                )))
            })
            .map_err(|err| {
                LoadError::text(ReportTextPos::None, "failed to load CSL style", err)
            })
    }

    /// Get the underlying independent style.
    pub fn get(&self) -> &citationberg::IndependentStyle {
        self.0.as_ref()
    }
}

/// Source for a CSL style.
#[derive(Debug, Clone, PartialEq, Hash)]
pub enum CslSource {
    /// A predefined named style and potentially a deprecation warning.
    Named(ArchivedStyle, Option<EcoString>),
    /// A normal data source.
    Normal(DataSource),
}

impl Reflect for CslSource {
    #[comemo::memoize]
    fn input() -> CastInfo {
        let source = std::iter::once(DataSource::input());

        /// All possible names and their short documentation for `ArchivedStyle`, including aliases.
        static ARCHIVED_STYLE_NAMES: LazyLock<Vec<(&&str, &'static str)>> =
            LazyLock::new(|| {
                ArchivedStyle::all()
                    .iter()
                    .flat_map(|name| {
                        let (main_name, aliases) = name
                            .names()
                            .split_first()
                            .expect("all ArchivedStyle should have at least one name");

                        std::iter::once((main_name, name.display_name())).chain(
                            aliases.iter().map(move |alias| {
                                // Leaking is okay here, because we are in a `LazyLock`.
                                let docs: &'static str = Box::leak(
                                    format!("A short alias of `{main_name}`")
                                        .into_boxed_str(),
                                );
                                (alias, docs)
                            }),
                        )
                    })
                    .collect()
            });
        let names = ARCHIVED_STYLE_NAMES
            .iter()
            .map(|(value, docs)| CastInfo::Value(value.into_value(), docs));

        CastInfo::Union(source.into_iter().chain(names).collect())
    }

    fn output() -> CastInfo {
        DataSource::output()
    }

    fn castable(value: &Value) -> bool {
        DataSource::castable(value)
    }
}

impl FromValue for CslSource {
    fn from_value(value: Value) -> HintedStrResult<Self> {
        if EcoString::castable(&value) {
            let string = EcoString::from_value(value.clone())?;
            if Path::new(string.as_str()).extension().is_none() {
                let replacement = replacement(&string);
                let deprecation = replacement.map(|instead| {
                    eco_format!(
                        "style `{}` has been deprecated in favor of `{}`",
                        string.repr(),
                        instead.repr(),
                    )
                });
                let style = ArchivedStyle::by_name(&string).ok_or_else(|| {
                    deprecation
                        .clone()
                        .unwrap_or_else(|| eco_format!("unknown style: {string}"))
                })?;
                return Ok(CslSource::Named(style, deprecation));
            }
        }

        DataSource::from_value(value).map(CslSource::Normal)
    }
}

impl IntoValue for CslSource {
    fn into_value(self) -> Value {
        match self {
            // We prefer the shorter names which are at the back of the array.
            Self::Named(v, _) => v.names().last().unwrap().into_value(),
            Self::Normal(v) => v.into_value(),
        }
    }
}

/// Maps from style names to their replacements.
///
/// TODO: Fully move this into hayagriva somehow.
fn replacement(style: &str) -> Option<&'static str> {
    Some(match style {
        "chicago-fullnotes" => "chicago-notes",
        "modern-humanities-research-association" => {
            "modern-humanities-research-association-notes"
        }
        "council-of-science-editors" => "cse-citation-sequence-brackets-8th-edition",
        "council-of-science-editors-author-date" => "cse-name-year",
        "modern-language-association-8" | "mla-8" => "modern-language-association",
        "vancouver" => "nlm-citation-sequence",
        "vancouver-superscript" => "nlm-citation-sequence-superscript",
        _ => return None,
    })
}

/// Fully formatted citations and references, generated once (through
/// memoization) for the whole document. This setup is necessary because
/// citation formatting is inherently stateful and we need access to all
/// citations to do it.
/// Fully formatted citation groups and bibliographies, generated once (through
/// memoization) for the whole document.
///
/// This setup is necessary because citation formatting is inherently stateful
/// and we need access to all citations to do it.
pub struct Works {
    /// The document's rendered [`BibliographyElem`]s, keyed by their locations.
    bibliographies: FxHashMap<Location, SourceResult<RenderedBibliography>>,
    /// The document's rendered [`CiteGroup`]s, keyed by their locations.
    groups: FxHashMap<Location, SourceResult<Content>>,
}

/// The rendered parts for a bibliography.
pub struct RenderedBibliography {
    /// Lists all entries in the bibliography, with optional prefix.
    pub entries: Vec<RenderedEntry>,
    /// Whether the bibliography should have hanging indent applied.
    pub hanging_indent: bool,
}

/// The rendered parts for a bibliography entry.
pub struct RenderedEntry {
    /// An optional prefix. This is exposed separately because this will go into
    /// its own column for grid-based styles.
    pub prefix: Option<Content>,
    /// The main content of the rendered bibliography entry.
    pub body: Content,
    /// A location that should be attached to the rendered entry in some way.
    /// Citations will link there.
    pub backlink: Location,
}

impl Works {
    /// Generates and formats all bibliographies and citations.
    pub fn generate(engine: &mut Engine, span: Span) -> SourceResult<Arc<Works>> {
        let bibs_and_groups = engine.introspect(BibliographyIntrospection(span));
        Self::generate_impl(
            engine.world,
            engine.library,
            engine.introspector.into_raw(),
            engine.traced,
            TrackedMut::reborrow_mut(&mut engine.sink),
            engine.route.track(),
            &bibs_and_groups,
        )
        .at(span)
    }

    /// The internal implementation of [`Works::generate`].
    #[comemo::memoize]
    fn generate_impl(
        world: Tracked<dyn World + '_>,
        library: &LazyHash<crate::Library>,
        introspector: Tracked<dyn Introspector + '_>,
        traced: Tracked<Traced>,
        sink: TrackedMut<Sink>,
        route: Tracked<Route>,
        bibs_and_groups: &[Content],
    ) -> StrResult<Arc<Works>> {
        let mut engine = Engine {
            world,
            library,
            introspector: Protected::from_raw(introspector),
            traced,
            sink,
            route: Route::extend(route),
        };

        // Prepare bibliographies and citation groups for rendering with
        // hayagriva.
        let p = prepare(&mut engine, bibs_and_groups);

        // Render the bibliography and citations with hayagriva.
        let mut offsets = FxHashMap::default();
        let rendered =
            p.bibs.iter().map(|bib| render(bib, &mut offsets)).collect::<Vec<_>>();

        Ok(Arc::new(Works {
            bibliographies: show_bibliographies(world, &p, &rendered),
            groups: show_cite_groups(world, p, &rendered),
        }))
    }

    /// Returns the shown content for a citation.
    pub fn citation(&self, loc: Location, span: Span) -> SourceResult<Content> {
        self.groups
            .get(&loc)
            .cloned()
            .ok_or_else(citation_could_not_be_located)
            .at(span)?
    }

    /// Returns the shown content for a bibliography.
    pub fn bibliography(
        &self,
        loc: Location,
        span: Span,
    ) -> SourceResult<&RenderedBibliography> {
        self.bibliographies
            .get(&loc)
            .ok_or_else(bibliography_could_not_be_located)
            .at(span)?
            .as_ref()
            .map_err(Clone::clone)
    }
}

/// Preprocessed information for all bibliographies and citation groups in the
/// document, ready for rendering with hayagriva.
struct Preparation<'a> {
    /// Preprocessed information for all bibliographies in the document, in
    /// document order.
    bibs: Vec<PreparedBibliography<'a>>,
    /// Preprocessed information for all [`CiteGroup`] elements in the document,
    /// keyed by their [`Location`].
    groups: FxHashMap<Location, SourceResult<PreparedCiteGroup<'a>>>,
}

/// Preprocessed information for a bibliography and the citations assigned to
/// it, ready for processing by hayagriva.
struct PreparedBibliography<'a> {
    /// The underlying bibliography element.
    elem: &'a Packed<BibliographyElem>,
    /// Information about citation subgroups assigned to this bibliography, in
    /// document order. Each subgroup turns into one citation sent to hayagriva,
    /// A single [`CiteGroup`] can comprise multiple subgroups assigned to
    /// different bibliographies.
    subgroups: Vec<Subgroup<'a>>,
}

/// Holds consecutive citations from a `CiteGroup` that were assigned to the
/// same bibliography.
///
/// See [`CiteGroup`] for more details on citation grouping.
struct Subgroup<'a> {
    /// The underlying citation group.
    elem: &'a Packed<CiteGroup>,
    /// The citations in this subgroup.
    citations: SmallVec<[&'a Packed<CiteElem>; 1]>,
    /// The style picked for this subgroup. Citations are not segmented by style
    /// (at least currently); we simply pick the style of the first citation.
    style: &'a CslStyle,
}

/// Preprocessed information for a [`CiteGroup`]. Can be used to show the group
/// as [`Content`] after bibliographies are processed with hayagriva.
struct PreparedCiteGroup<'a>(SmallVec<[GroupPart<'a>; 1]>);

/// A segment in a preprocessed [`CiteGroup`].
enum GroupPart<'a> {
    /// This content should be displayed verbatim. In practice, this is only
    /// ever a [`SpaceElem`](crate::text::SpaceElem) between subgroups.
    Content(&'a Content),
    /// Points to a subgroup in one of the bibliographies in the document and
    /// should be substituted by the content produced for the citation.
    Subgroup(BibIndex, SubgroupIndex),
}

/// The index of a bibliography among all bibliographies in `p.bibs` where
/// `p: Preparation`.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
struct BibIndex(usize);

/// The index of a subgroup in `bib.subgroups` where `bib: PreparedBibliography`
/// among those assigned to the same bibliography.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
struct SubgroupIndex(usize);

/// Prepares the document's bibliographies and citation groups for rendering
/// with hayagriva.
///
/// Primarily, this involves
/// - assigning citations to bibliographies (and while doing so splitting
///   [`CiteGroup`]s into subgroups assigned to the same bibliography).
/// - retaining data structures that can be used to show the hayagriva output as
///   [`Content`] for bibliography and citation group elements.
fn prepare<'a>(engine: &mut Engine, bibs_and_groups: &'a [Content]) -> Preparation<'a> {
    // Maps from citations to bibliography indices for citations that were
    // explicitly selected via a bibliography with a `target`.
    let mut selected = FxHashMap::<Location, BibIndex>::default();

    // First, we process bibliographies. This involves:
    // - Creating a slot in `bibs` for each
    // - Creating a mapping from citations to bibliography index for all
    //   citations specifically targetted by a bibliography.
    let mut bibs = Vec::<PreparedBibliography>::new();
    for elem in bibs_and_groups {
        let Some(bib) = elem.to_packed::<BibliographyElem>() else { continue };
        let idx = BibIndex(bibs.len());
        bibs.push(PreparedBibliography { elem: bib, subgroups: vec![] });

        if let Smart::Custom(LocatableSelector(selector)) =
            bib.target.get_cloned(StyleChain::default())
        {
            for citation in engine.introspect(QueryIntrospection(selector, bib.span())) {
                selected.entry(citation.location().unwrap()).or_insert(idx);
            }
        }
    }

    // Then, we process citations groups. See the doc comment of
    // `prepare_cite_group` for more information on which steps this involves.
    let mut groups = FxHashMap::default();
    let mut bib_cursor = 0;
    for elem in bibs_and_groups {
        let Some(group) = elem.to_packed::<CiteGroup>() else {
            debug_assert!(elem.is::<BibliographyElem>());
            bib_cursor += 1;
            continue;
        };

        let loc = group.location().unwrap();
        let result = prepare_cite_group(&mut bibs, bib_cursor, group, &selected);
        groups.insert(loc, result);
    }

    Preparation { bibs, groups }
}

/// Prepares a [`CiteGroup`] by
/// - splitting it into subgroups assigned to the same bibliography
/// - storing a [`Subgroup`] for each of these in the approprate slot in `bibs`
/// - creating a [`PreparedCiteGroup`] which can be used to stitch the shown
///   content for the subgroups together after it was rendered with hayagriva.
fn prepare_cite_group<'a>(
    bibs: &mut [PreparedBibliography<'a>],
    bib_cursor: usize,
    group: &'a Packed<CiteGroup>,
    selected: &FxHashMap<Location, BibIndex>,
) -> SourceResult<PreparedCiteGroup<'a>> {
    // Holds the collected segments of the citation group.
    let mut parts = SmallVec::new();
    // Citations that make up the current subgroup.
    let mut subgroup = SmallVec::new();
    // The bibliography index for the current subgroup. If `Some(_)`, then
    // `subgroup` is not empty.
    let mut subgroup_bib = None;
    // The elements in `group.children[tail..i]` are interior spaces.
    let mut tail = 0;
    // Holds errors for any uncovered citation. We collect them instead of
    // bailing early so that we can give multiple errors at once.
    let mut errors = EcoVec::new();

    for (i, child) in group.children.iter().enumerate() {
        // The children are either citations or spaces. We skip interior spaces
        // without updating `tail`, so that at any point we can produce a slice
        // with the trailing spaces.
        //
        // Note that exterior spaces are not supported, but they also will never
        // appear in groups in practice.
        let Some(citation) = child.to_packed::<CiteElem>() else { continue };
        let spaces = &group.children[tail..i];
        tail = i + 1;

        // Determine the pre-selected bibliography or assign an auto
        // bibliography.
        let bib_idx = if let Some(&idx) = selected.get(&citation.location().unwrap()) {
            // Ensure that the bibliography contains the key.
            let bib = &bibs[idx.0];
            if !bib.elem.sources.derived.has(citation.key) {
                errors.push(key_does_not_exist(citation, bib));
                continue;
            }
            idx
        } else if let Some(idx) = select_auto_bib(citation, bibs, bib_cursor) {
            idx
        } else {
            errors.push(uncovered_citation(citation, bibs));
            continue;
        };

        // If the assigned bibliography changes, flush the previous subgroup and
        // the spaces between it and the current `child`.
        if let Some(subgroup_bib) = subgroup_bib
            && subgroup_bib != bib_idx
        {
            parts.push(save_subgroup(
                bibs,
                subgroup_bib,
                group,
                std::mem::take(&mut subgroup),
            ));
            parts.extend(spaces.iter().map(GroupPart::Content));
        }

        subgroup_bib = Some(bib_idx);
        subgroup.push(citation);
    }

    // Flush the final subgroup if any.
    if let Some(subgroup_bib) = subgroup_bib {
        parts.push(save_subgroup(bibs, subgroup_bib, group, subgroup));
    }

    if !errors.is_empty() {
        return Err(errors);
    }

    Ok(PreparedCiteGroup(parts))
}

/// Builds a [`Subgroup`] and stores it in the selected bibliography. Returns a
/// [`GroupPart`] to remember that this subgroup belongs to the [`CiteGroup`]
/// that's currently being prepared.
fn save_subgroup<'a>(
    bibs: &mut [PreparedBibliography<'a>],
    bib_idx: BibIndex,
    elem: &'a Packed<CiteGroup>,
    citations: SmallVec<[&'a Packed<CiteElem>; 1]>,
) -> GroupPart<'a> {
    let bib = &mut bibs[bib_idx.0];
    let style = if let Some(first) = citations.first()
        && let Smart::Custom(style) = first.style.get_ref(StyleChain::default())
    {
        &style.derived
    } else {
        &bib.elem.style.get_ref(StyleChain::default()).derived
    };

    let sub_idx = SubgroupIndex(bib.subgroups.len());
    bib.subgroups.push(Subgroup { elem, citations, style });

    GroupPart::Subgroup(bib_idx, sub_idx)
}

/// Selects the appropriate bibliography for a citation that was not explicitly
/// targeted by a bibliography.
///
/// The priority is:
/// 1. First following auto bibliography containing the citation key
/// 2. First preceding auto bibliography containing the citation key
///
/// Returns `None` if no auto bibliography contains the key.
fn select_auto_bib(
    citation: &Packed<CiteElem>,
    bibs: &[PreparedBibliography],
    bib_cursor: usize,
) -> Option<BibIndex> {
    let bibs = bibs.iter().enumerate();
    let before = bibs.clone().take(bib_cursor);
    let after = bibs.skip(bib_cursor);
    after
        .chain(before.rev())
        .find(|(_, bib)| {
            bib.elem.target.get_ref(StyleChain::default()).is_auto()
                && bib.elem.sources.derived.has(citation.key)
        })
        .map(|(idx, _)| BibIndex(idx))
}

/// Renders the bibliography and citation groups with hayagriva.
fn render<'a>(
    bib: &PreparedBibliography<'a>,
    offsets: &mut FxHashMap<Smart<&'a str>, usize>,
) -> hayagriva::Rendered {
    static LOCALES: LazyLock<Vec<citationberg::Locale>> =
        LazyLock::new(hayagriva::archive::locales);

    let database = &bib.elem.sources.derived;

    let mut driver = BibliographyDriver::new();
    let mut offset = bib
        .elem
        .group
        .get_ref(StyleChain::default())
        .as_ref()
        .map(|group| offsets.entry(group.as_deref()).or_insert(0));

    if let Some(offset) = &mut offset {
        driver = driver.with_citation_number_offset(**offset);
    }

    for group in &bib.subgroups {
        let items = group
            .citations
            .iter()
            .map(|child| {
                let entry = database.get(child.key).expect("entry to be present");
                citation_item(entry, child)
            })
            .collect::<Vec<_>>();

        let first = &group.citations[0];
        let locale = locale(first.lang.unwrap_or(Lang::ENGLISH), first.region.flatten());

        driver.citation(CitationRequest::new(
            items,
            group.style.get(),
            Some(locale),
            &LOCALES,
            None,
        ));
    }

    let bib_style = &bib.elem.style.get_ref(StyleChain::default()).derived;
    let locale =
        locale(bib.elem.lang.unwrap_or(Lang::ENGLISH), bib.elem.region.flatten());

    // Add hidden items for everything if we should print the whole
    // bibliography.
    if bib.elem.full.get(StyleChain::default()) {
        for (_, entry) in database.iter() {
            driver.citation(CitationRequest::new(
                vec![CitationItem::new(entry, None, None, true, None)],
                bib_style.get(),
                Some(locale.clone()),
                &LOCALES,
                None,
            ));
        }
    }

    let rendered = driver.finish(BibliographyRequest {
        style: bib_style.get(),
        locale: Some(locale),
        locale_files: &LOCALES,
    });

    if let Some(offset) = offset
        && let Some(bib) = &rendered.bibliography
        // Check whether the bibliography or any citation displays citation
        // numbers. Only then does the bibliography occupy a numbering range
        // that subsequent bibliographies in the same group must skip.
        && (bib.items.iter().any(displays_citation_number)
            || rendered.citations.iter().any(|rendered| {
                rendered
                    .citation
                    .find_meta(&hayagriva::ElemMeta::CitationNumber)
                    .is_some()
            }))
    {
        *offset += bib.items.len();
    }

    rendered
}

/// Whether a rendered bibliography item displays a citation number.
///
/// For styles with `second-field-align` (like IEEE), the number resides in
/// the item's first field rather than in its content.
fn displays_citation_number(item: &hayagriva::BibliographyItem) -> bool {
    item.content.find_meta(&hayagriva::ElemMeta::CitationNumber).is_some()
        || item.first_field.as_ref().is_some_and(|child| match child {
            hayagriva::ElemChild::Elem(elem) => {
                elem.meta == Some(hayagriva::ElemMeta::CitationNumber)
                    || elem
                        .children
                        .find_meta(&hayagriva::ElemMeta::CitationNumber)
                        .is_some()
            }
            _ => false,
        })
}

/// Creates a hayagriva citation item for a citation element.
fn citation_item<'a>(
    entry: &'a hayagriva::Entry,
    child: &'a Packed<CiteElem>,
) -> CitationItem<'a, hayagriva::Entry> {
    let supplement = child.supplement.get_cloned(StyleChain::default());
    let locator = supplement.as_ref().map(|c| {
        SpecificLocator(
            citationberg::taxonomy::Locator::Custom,
            hayagriva::LocatorPayload::Transparent(TransparentLocator::new(c.clone())),
        )
    });

    let mut hidden = false;
    let special_form = match child.form.get(StyleChain::default()) {
        None => {
            hidden = true;
            None
        }
        Some(CitationForm::Normal) => None,
        Some(CitationForm::Prose) => Some(hayagriva::CitePurpose::Prose),
        Some(CitationForm::Full) => Some(hayagriva::CitePurpose::Full),
        Some(CitationForm::Author) => Some(hayagriva::CitePurpose::Author),
        Some(CitationForm::Year) => Some(hayagriva::CitePurpose::Year),
    };

    CitationItem::new(entry, locator, None, hidden, special_form)
}

/// Produces the structured content for all [`BibliographyElem`]s in the
/// document. The output of this is directly stored in the [`Works`] and
/// consumed by the target-specific bibliography show rules.
fn show_bibliographies(
    world: Tracked<dyn World + '_>,
    p: &Preparation,
    rendered: &[hayagriva::Rendered],
) -> FxHashMap<Location, SourceResult<RenderedBibliography>> {
    p.bibs
        .iter()
        .zip(rendered)
        .map(|(bib, rendered)| {
            let loc = bib.elem.location().unwrap();
            let result = rendered
                .bibliography
                .as_ref()
                .ok_or_else(|| {
                    style_unsuitable(
                        &bib.elem.style.get_ref(StyleChain::default()).source,
                    )
                })
                .and_then(|rendered| show_bibliography(world, bib, rendered))
                .at(bib.elem.span());
            (loc, result)
        })
        .collect()
}

/// Turns a bibliography rendered with hayagriva into a final
/// [`RenderedBibliography`].
fn show_bibliography(
    world: Tracked<dyn World + '_>,
    bib: &PreparedBibliography,
    rendered: &hayagriva::RenderedBibliography,
) -> StrResult<RenderedBibliography> {
    let to_citations = links_to_citations(&bib.subgroups);

    let mut entries = Vec::with_capacity(rendered.items.len());
    for (k, item) in rendered.items.iter().enumerate() {
        let ctx = ShowCtx {
            world,
            span: bib.elem.span(),
            supplement: &|_| None,
            link: &|_| None,
        };

        // Render the first field.
        let mut prefix = item
            .first_field
            .as_ref()
            .map(|elem| show_elem_child(&ctx, elem, None, false))
            .transpose()?;

        // Render the main reference content.
        let body = show_elem_children(&ctx, &item.content, Some(&mut prefix), false)?;

        // Attach link to citation to the prefix.
        let prefix = prefix.map(|content| {
            if let Some(location) = to_citations.get(item.key.as_str()) {
                let alt = content.plain_text();
                let body = content.spanned(ctx.span);
                DirectLinkElem::new(*location, body, Some(alt)).pack()
            } else {
                content
            }
        });

        entries.push(RenderedEntry { prefix, body, backlink: entry_location(bib, k) });
    }

    Ok(RenderedBibliography { entries, hanging_indent: rendered.hanging_indent })
}

/// Produces the content for all [`CiteGroup`]s in the document. The output of
/// this is directly stored in the [`Works`] and consumed by [`CiteGroup`] show
/// rule.
fn show_cite_groups(
    world: Tracked<dyn World + '_>,
    p: Preparation,
    rendered: &[hayagriva::Rendered],
) -> FxHashMap<Location, SourceResult<Content>> {
    let to_entries = links_to_entries(&p, rendered);
    p.groups
        .into_iter()
        .map(|(loc, group)| {
            let result = group.and_then(|group| {
                show_cite_group(world, &group, &p.bibs, rendered, |idx, key| {
                    to_entries.get(&(idx, key)).copied()
                })
            });
            (loc, result)
        })
        .collect()
}

/// Produces the content for a [`CiteGroup`] by stitching together the hayagriva
/// output for each subgroup and interspersing potential space elements that
/// were retained between subgroups.
fn show_cite_group(
    world: Tracked<dyn World + '_>,
    group: &PreparedCiteGroup,
    prepared: &[PreparedBibliography],
    rendered: &[hayagriva::Rendered],
    to_entry: impl Fn(BibIndex, &str) -> Option<Location>,
) -> SourceResult<Content> {
    let mut seq = vec![];
    for part in &group.0 {
        seq.push(match part {
            GroupPart::Content(c) => (**c).clone(),
            GroupPart::Subgroup(bib_idx, sub_idx) => {
                let subgroup = &prepared[bib_idx.0].subgroups[sub_idx.0];
                let item = &rendered[bib_idx.0].citations[sub_idx.0];
                show_subgroup(world, subgroup, item, |key| to_entry(*bib_idx, key))?
            }
        });
    }
    Ok(Content::sequence(seq))
}

/// Displays a single citation subgroup.
fn show_subgroup(
    world: Tracked<dyn World + '_>,
    group: &Subgroup,
    citation: &hayagriva::RenderedCitation,
    to_entry: impl Fn(&str) -> Option<Location>,
) -> SourceResult<Content> {
    if group
        .citations
        .iter()
        .all(|sub| sub.form.get(StyleChain::default()).is_none())
    {
        return Ok(Content::empty());
    }

    let span = Span::find(group.citations.iter().map(|elem| elem.span()));
    let supplement =
        |i: usize| group.citations.get(i)?.supplement.get_cloned(StyleChain::default());
    let link = |i: usize| to_entry(group.citations.get(i)?.key.resolve().as_str());
    let ctx = ShowCtx { world, span, supplement: &supplement, link: &link };

    let mut realized =
        show_elem_children(&ctx, &citation.citation, None, true).at(span)?;

    if group.style.get().settings.class == citationberg::StyleClass::Note
        && group.citations.iter().all(|sub| {
            matches!(
                sub.form.get(StyleChain::default()),
                None | Some(CitationForm::Normal)
            )
        })
    {
        realized = FootnoteElem::with_content(realized).pack();
    }

    Ok(realized)
}

/// Creates a map from citation keys to the citation group containing the first
/// citation assigned to a particular bibliography that references the key.
///
/// This is used by bibliography entries to link back to the first citation that
/// references them.
fn links_to_citations(groups: &[Subgroup]) -> FxHashMap<ResolvedPicoStr, Location> {
    let mut map = FxHashMap::default();
    for group in groups {
        for child in &group.citations {
            let key = child.key.resolve();
            map.entry(key).or_insert(group.elem.location().unwrap());
        }
    }
    map
}

/// Creates a map from a bibliography index + citation key to the corresponding
/// entry in the bibliography.
///
/// This is used by citations to link forward to the bibliography entry they
/// reference.
fn links_to_entries<'a>(
    p: &Preparation,
    rendered: &'a [hayagriva::Rendered],
) -> FxHashMap<(BibIndex, &'a str), Location> {
    let mut links = FxHashMap::default();
    for (i, (bib, rendered)) in p.bibs.iter().zip(rendered).enumerate() {
        let Some(rendered) = &rendered.bibliography else { continue };
        for (k, item) in rendered.items.iter().enumerate() {
            links.insert((BibIndex(i), item.key.as_str()), entry_location(bib, k));
        }
    }
    links
}

/// Each reference is assigned a manually created well-known location that is
/// derived from the bibliography's location. This way, citations can link to
/// them without having to query for them (which would incur an extra layout
/// iteration).
fn entry_location(bib: &PreparedBibliography, k: usize) -> Location {
    bib.elem.location().unwrap().variant(k + 1)
}

/// Additional data needed to show hayagriva elements as content.
struct ShowCtx<'a> {
    /// The world that is used to evaluate mathematical material.
    world: Tracked<'a, dyn World + 'a>,
    /// The span that is attached to all of the resulting content.
    span: Span,
    /// Resolves the supplement of i-th citation in the request.
    supplement: &'a dyn Fn(usize) -> Option<Content>,
    /// Resolves where the i-th citation in the request should link to.
    link: &'a dyn Fn(usize) -> Option<Location>,
}

/// Displays rendered hayagriva elements.
///
/// The `prefix` can be a separate content storage where `left-margin`
/// elements will be accumulated into.
///
/// `is_citation` dictates whether whitespace at the start of the citation
/// will be eliminated. Some CSL styles yield whitespace at the start of
/// their citations, which should instead be handled by Typst.
fn show_elem_children(
    ctx: &ShowCtx,
    elems: &hayagriva::ElemChildren,
    mut prefix: Option<&mut Option<Content>>,
    is_citation: bool,
) -> StrResult<Content> {
    Ok(Content::sequence(
        elems
            .0
            .iter()
            .enumerate()
            .map(|(i, elem)| {
                show_elem_child(ctx, elem, prefix.as_deref_mut(), is_citation && i == 0)
            })
            .collect::<StrResult<Vec<_>>>()?,
    ))
}

/// Displays a rendered hayagriva element.
fn show_elem_child(
    ctx: &ShowCtx,
    elem: &hayagriva::ElemChild,
    prefix: Option<&mut Option<Content>>,
    trim_start: bool,
) -> StrResult<Content> {
    Ok(match elem {
        hayagriva::ElemChild::Text(formatted) => {
            show_formatted(ctx, formatted, trim_start)
        }
        hayagriva::ElemChild::Elem(elem) => show_elem(ctx, elem, prefix)?,
        hayagriva::ElemChild::Markup(markup) => show_math(ctx, markup),
        hayagriva::ElemChild::Link { text, url } => show_link(ctx, text, url)?,
        hayagriva::ElemChild::Transparent { cite_idx, format } => {
            show_transparent(ctx, *cite_idx, format)
        }
    })
}

/// Displays a block-level element.
fn show_elem(
    ctx: &ShowCtx,
    elem: &hayagriva::Elem,
    mut prefix: Option<&mut Option<Content>>,
) -> StrResult<Content> {
    use citationberg::Display;

    let block_level = matches!(elem.display, Some(Display::Block | Display::Indent));

    let mut content = show_elem_children(
        ctx,
        &elem.children,
        if block_level { None } else { prefix.as_deref_mut() },
        false,
    )?;

    match elem.display {
        Some(Display::Block) => {
            content = BlockElem::packed(content).spanned(ctx.span);
        }
        Some(Display::Indent) => {
            content = CslIndentElem::new(content).pack().spanned(ctx.span);
        }
        Some(Display::LeftMargin) => {
            // The `display="left-margin"` attribute is only supported at
            // the top-level (when prefix is `Some(_)`). Within a
            // block-level container, it is ignored. The CSL spec is not
            // specific about this, but it is in line with citeproc.js's
            // behaviour.
            if let Some(prefix) = prefix {
                *prefix.get_or_insert_with(Default::default) += content;
                return Ok(Content::empty());
            }
        }
        _ => {}
    }

    content = content.spanned(ctx.span);

    if let Some(hayagriva::ElemMeta::Entry(i)) = elem.meta
        && let Some(location) = (ctx.link)(i)
    {
        let alt = content.plain_text();
        content = DirectLinkElem::new(location, content, Some(alt)).pack();
    }

    Ok(content)
}

/// Displays math.
fn show_math(ctx: &ShowCtx, math: &str) -> Content {
    let library = ctx.world.library();
    (library.routines.eval_string)(
        ctx.world,
        library,
        // TODO: propagate warnings
        Sink::new().track_mut(),
        EmptyIntrospector.track(),
        Context::none().track(),
        math,
        SpanMode::Uniform(ctx.span),
        SyntaxMode::Math,
        Scope::new(),
    )
    .map(Value::display)
    .unwrap_or_else(|_| TextElem::packed(math).spanned(ctx.span))
}

/// Displays a link.
fn show_link(
    ctx: &ShowCtx,
    text: &hayagriva::Formatted,
    url: &str,
) -> StrResult<Content> {
    let dest = Destination::Url(Url::new(url)?);
    Ok(LinkElem::new(dest.into(), show_formatted(ctx, text, false))
        .pack()
        .spanned(ctx.span))
}

/// Displays transparent pass-through content.
fn show_transparent(ctx: &ShowCtx, i: usize, format: &hayagriva::Formatting) -> Content {
    let content = (ctx.supplement)(i).unwrap_or_default();
    show_with_formatting(content, format)
}

/// Displays formatted hayagriva text as content.
fn show_formatted(
    ctx: &ShowCtx,
    formatted: &hayagriva::Formatted,
    trim_start: bool,
) -> Content {
    let formatted_text =
        if trim_start { formatted.text.trim_start() } else { formatted.text.as_str() };

    let content = TextElem::packed(formatted_text).spanned(ctx.span);
    show_with_formatting(content, &formatted.formatting)
}

/// Applies hayagriva formatting to content.
fn show_with_formatting(mut content: Content, format: &hayagriva::Formatting) -> Content {
    match format.font_style {
        citationberg::FontStyle::Normal => {}
        citationberg::FontStyle::Italic => {
            content = content.emph();
        }
    }

    match format.font_variant {
        citationberg::FontVariant::Normal => {}
        citationberg::FontVariant::SmallCaps => {
            content = SmallcapsElem::new(content).pack();
        }
    }

    match format.font_weight {
        citationberg::FontWeight::Normal => {}
        citationberg::FontWeight::Bold => {
            content = content.strong();
        }
        citationberg::FontWeight::Light => {
            // We don't have a semantic element for "light" and a `StrongElem`
            // with negative delta does not have the appropriate semantics, so
            // keeping this as a direct style.
            content = CslLightElem::new(content).pack();
        }
    }

    match format.text_decoration {
        citationberg::TextDecoration::None => {}
        citationberg::TextDecoration::Underline => {
            content = content.underlined();
        }
    }

    let span = content.span();
    match format.vertical_align {
        citationberg::VerticalAlign::None => {}
        citationberg::VerticalAlign::Baseline => {}
        citationberg::VerticalAlign::Sup => {
            // Add zero-width weak spacing to make the superscript "sticky".
            content =
                HElem::hole().clone() + SuperElem::new(content).pack().spanned(span);
        }
        citationberg::VerticalAlign::Sub => {
            content = HElem::hole().clone() + SubElem::new(content).pack().spanned(span);
        }
    }

    content
}

/// Creates a locale code from language and optionally region.
fn locale(lang: Lang, region: Option<Region>) -> citationberg::LocaleCode {
    let mut value = String::with_capacity(5);
    value.push_str(lang.as_str());
    if let Some(region) = region {
        value.push('-');
        value.push_str(region.as_str())
    }
    citationberg::LocaleCode(value)
}

/// Translation of `font-weight="light"` in CSL.
///
/// We translate `font-weight: "bold"` to `<strong>` since it's likely that the
/// CSL spec just talks about bold because it has no notion of semantic
/// elements. The benefits of a strict reading of the spec are also rather
/// questionable, while using semantic elements makes the bibliography more
/// accessible, easier to style, and more portable across export targets.
#[elem]
pub struct CslLightElem {
    #[required]
    pub body: Content,
}

/// Translation of `display="indent"` in CSL.
///
/// A `display="block"` is simply translated to a Typst `BlockElem`. Similarly,
/// we could translate `display="indent"` to a `PadElem`, but (a) it does not
/// yet have support in HTML and (b) a `PadElem` described a fixed padding while
/// CSL leaves the amount of padding user-defined so it's not a perfect fit.
#[elem]
pub struct CslIndentElem {
    #[required]
    pub body: Content,
}

/// Retrieves all bibliographies and citation groups in the document.
///
/// This is separate from `QueryIntrospection` so that we can customize the
/// diagnostic as the `CiteGroup` is internal. The default query message is also
/// not that helpful in this case.
#[derive(Debug, Clone, PartialEq, Hash)]
struct BibliographyIntrospection(Span);

impl Introspect for BibliographyIntrospection {
    type Output = EcoVec<Content>;

    fn introspect(
        &self,
        _: &mut Engine,
        introspector: Tracked<dyn Introspector + '_>,
    ) -> Self::Output {
        introspector.query(&Selector::Or(eco_vec![
            BibliographyElem::ELEM.select(),
            CiteGroup::ELEM.select(),
        ]))
    }

    fn diagnose(&self, _: &History<Self::Output>) -> SourceDiagnostic {
        warning!(self.0, "citations and bibliographies did not stabilize")
    }
}

/// The diagnostic when a citation wasn't found in the pre-formatted list.
fn citation_could_not_be_located() -> HintedString {
    error!(
        "citation could not be located";
        hint: "this citation is not stably present in the document";
        hint: "this can be caused by measurement or introspection";
    )
}

/// The diagnostic when a bibliography wasn't found in the pre-formatted list.
fn bibliography_could_not_be_located() -> HintedString {
    error!(
        "bibliography could not be located";
        hint: "this bibliography is not stably present in the document";
        hint: "this can be caused by measurement or introspection";
    )
}

/// The diagnostic when a citation is not picked up by any bibliography.
fn uncovered_citation(
    citation: &Packed<CiteElem>,
    bibs: &[PreparedBibliography],
) -> SourceDiagnostic {
    let span = citation.span();
    let key = citation.key.resolve();
    if bibs.is_empty() {
        error!(span, "the document does not contain a bibliography")
    } else if let Some(bib) =
        bibs.iter().find(|bib| bib.elem.sources.derived.has(citation.key))
    {
        error!(
            span,
            "citation is not covered by any bibliography";
            hint[bib.elem.span()]:
            "a bibliography containing the key `{key}` exists, \
             but its `target` excludes this citation";
        )
    } else {
        error!(
            span,
            "citation key `{key}` is not present in {} bibliography",
            if bibs.len() == 1 { "the" } else { "any" },
        )
    }
}

/// The diagnostic when a citation is explicitly targeted by a bibliography,
/// but its key does not exist in said bibliography.
fn key_does_not_exist(
    citation: &Packed<CiteElem>,
    bib: &PreparedBibliography,
) -> SourceDiagnostic {
    error!(
        citation.span(),
        "key `{}` does not exist in the bibliography",
        citation.key.resolve();
        hint[bib.elem.span()]: "the citation was assigned to this bibliography";
    )
}

/// The error message when a CSL style cannot be used for bibliographies.
#[cold]
fn style_unsuitable(source: &CslSource) -> EcoString {
    match source {
        CslSource::Named(style, _) => eco_format!(
            "CSL style \"{}\" is not suitable for bibliographies",
            style.display_name()
        ),
        CslSource::Normal(..) => "CSL style is not suitable for bibliographies".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bibliography_load_builtin_styles() {
        for &archived in ArchivedStyle::all() {
            let _ = CslStyle::from_archived(archived);
        }
    }

    #[test]
    fn test_csl_source_cast_info_include_all_names() {
        let CastInfo::Union(cast_info) = CslSource::input() else {
            panic!("the cast info of CslSource should be a union");
        };

        let missing: Vec<_> = ArchivedStyle::all()
            .iter()
            .flat_map(|style| style.names())
            .filter(|name| {
                let found = cast_info.iter().any(|info| match info {
                    CastInfo::Value(Value::Str(n), _) => n.as_str() == **name,
                    _ => false,
                });
                !found
            })
            .collect();

        assert!(
            missing.is_empty(),
            "missing style names in CslSource cast info: '{missing:?}'"
        );
    }
}
