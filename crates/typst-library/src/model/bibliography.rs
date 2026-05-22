use std::any::TypeId;
use std::fmt::{self, Debug, Formatter};
use std::num::NonZeroUsize;
use std::path::Path;
use std::sync::{Arc, LazyLock};

use comemo::{Track, Tracked};
use ecow::{EcoString, EcoVec, eco_format};
use hayagriva::archive::ArchivedStyle;
use hayagriva::io::BibLaTeXError;
use hayagriva::{
    BibliographyDriver, BibliographyRequest, CitationItem, CitationRequest, Library,
    SpecificLocator, TransparentLocator, citationberg,
};
use indexmap::IndexMap;
use rustc_hash::{FxBuildHasher, FxHashMap};
use typst_syntax::{Span, Spanned, SyntaxMode};
use typst_utils::{ManuallyHash, NonZeroExt, PicoStr, ResolvedPicoStr};

use crate::World;
use crate::diag::{
    At, HintedStrResult, HintedString, LoadError, LoadResult, LoadedWithin,
    ReportTextPos, SourceDiagnostic, SourceResult, StrResult, bail, error, warning,
};
use crate::engine::{Engine, Sink};
use crate::foundations::{
    Bytes, CastInfo, Content, Context, Derived, FromValue, IntoValue, Label,
    NativeElement, OneOrMultiple, Packed, Reflect, Scope, ShowSet, Smart, StyleChain,
    Styles, Synthesize, Value, elem,
};
use crate::introspection::{
    EmptyIntrospector, History, Introspect, Introspector, Locatable, Location,
    QueryIntrospection,
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
    /// Find the document's bibliography.
    pub fn find(engine: &mut Engine, span: Span) -> StrResult<Packed<Self>> {
        let elems = engine.introspect(QueryIntrospection(Self::ELEM.select(), span));

        let mut iter = elems.iter();
        let Some(elem) = iter.next() else {
            bail!("the document does not contain a bibliography");
        };

        if iter.next().is_some() {
            bail!("multiple bibliographies are not yet supported");
        }

        Ok(elem.to_packed::<Self>().unwrap().clone())
    }

    /// Whether the bibliography contains the given key.
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
                    engine.sink.warn(warning!(span, "{message}"));
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
    Named(ArchivedStyle, Option<&'static str>),
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
                let mut warning = None;
                if string.as_str() == "chicago-fullnotes" {
                    warning = Some(
                        "style \"chicago-fullnotes\" has been deprecated \
                         in favor of \"chicago-notes\"",
                    );
                } else if string.as_str() == "modern-humanities-research-association" {
                    warning = Some(
                        "style \"modern-humanities-research-association\" \
                         has been deprecated in favor of \
                         \"modern-humanities-research-association-notes\"",
                    );
                }

                let style = ArchivedStyle::by_name(&string)
                    .ok_or_else(|| eco_format!("unknown style: {}", string))?;
                return Ok(CslSource::Named(style, warning));
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

/// Fully formatted citations and references, generated once (through
/// memoization) for the whole document. This setup is necessary because
/// citation formatting is inherently stateful and we need access to all
/// citations to do it.
pub struct Works {
    /// The document's rendered [`BibliographyElem`].
    bibliography: SourceResult<RenderedBibliography>,
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
    /// Generates and formats the bibliography and citations.
    pub fn generate(engine: &mut Engine, span: Span) -> SourceResult<Arc<Works>> {
        let bibliography = BibliographyElem::find(engine, span).at(span)?;
        let groups = engine.introspect(CiteGroupIntrospection(span));
        Self::generate_impl(engine.world, &bibliography, &groups).at(span)
    }

    /// Same as [`Works::generate`], but reuses an existing
    /// bibliography (no need to query it).
    pub fn with_bibliography(
        engine: &mut Engine,
        bibliography: Packed<BibliographyElem>,
    ) -> SourceResult<Arc<Works>> {
        let span = bibliography.span();
        let groups = engine.introspect(CiteGroupIntrospection(span));
        Self::generate_impl(engine.world, &bibliography, &groups).at(span)
    }

    /// The internal implementation of [`Works::generate`].
    #[comemo::memoize]
    fn generate_impl(
        world: Tracked<dyn World + '_>,
        bibliography: &Packed<BibliographyElem>,
        groups: &[Content],
    ) -> StrResult<Arc<Works>> {
        let mut shown_groups =
            FxHashMap::with_capacity_and_hasher(groups.len(), FxBuildHasher);

        // Render the bibliography and citations with hayagriva. Already inserts
        // errors into `citations` for citation key that don't resolve.
        let (rendered, successes) = render(bibliography, groups, &mut shown_groups);

        // Show the citations.
        let to_entries = match &rendered.bibliography {
            Some(rendered) => links_to_entries(bibliography, rendered),
            None => FxHashMap::default(),
        };
        for (rendered, group) in rendered.citations.iter().zip(successes) {
            let loc = group.location().unwrap();
            let result =
                show_cite_group(world, bibliography, group, rendered, &to_entries);
            shown_groups.insert(loc, result);
        }

        // Show the bibliography.
        let shown_bibliography = rendered
            .bibliography
            .as_ref()
            .ok_or_else(|| {
                style_unsuitable(
                    &bibliography.style.get_ref(StyleChain::default()).source,
                )
            })
            .and_then(|rendered| show_bibliography(world, bibliography, groups, rendered))
            .at(bibliography.span());

        Ok(Arc::new(Works {
            groups: shown_groups,
            bibliography: shown_bibliography,
        }))
    }

    /// Returns the shown content for a citation.
    pub fn citation(&self, loc: Location, span: Span) -> SourceResult<Content> {
        self.groups
            .get(&loc)
            .cloned()
            .ok_or_else(failed_to_format_citation)
            .at(span)?
    }

    /// Returns the shown content for a bibliography.
    pub fn bibliography(&self) -> SourceResult<&RenderedBibliography> {
        self.bibliography.as_ref().map_err(Clone::clone)
    }
}

/// Renders the bibliography and citation groups with hayagriva.
fn render<'a>(
    bibliography: &Packed<BibliographyElem>,
    groups: &'a [Content],
    failures: &mut FxHashMap<Location, SourceResult<Content>>,
) -> (hayagriva::Rendered, Vec<&'a Packed<CiteGroup>>) {
    static LOCALES: LazyLock<Vec<citationberg::Locale>> =
        LazyLock::new(hayagriva::archive::locales);

    let database = &bibliography.sources.derived;

    // Process all citation groups.
    let mut driver = BibliographyDriver::new();
    let mut successes = Vec::new();

    for elem in groups {
        let group = elem.to_packed::<CiteGroup>().unwrap();
        let location = elem.location().unwrap();

        // Groups should never be empty.
        let Some(first) = group.children.first() else { continue };

        let mut items = Vec::with_capacity(group.children.len());
        let mut errors = EcoVec::new();

        // Create infos and items for each child in the group.
        for child in &group.children {
            match database.get(child.key) {
                Some(entry) => items.push(citation_item(entry, child)),
                _ => errors.push(error!(
                    child.span(),
                    "key `{}` does not exist in the bibliography",
                    child.key.resolve(),
                )),
            }
        }

        if !errors.is_empty() {
            failures.insert(location, Err(errors));
            continue;
        }

        let style = resolve_style(group, bibliography).get();
        let locale = locale(first.lang.unwrap_or(Lang::ENGLISH), first.region.flatten());

        driver.citation(CitationRequest::new(items, style, Some(locale), &LOCALES, None));
        successes.push(group);
    }

    let bib_style = &bibliography.style.get_ref(StyleChain::default()).derived;
    let locale =
        locale(bibliography.lang.unwrap_or(Lang::ENGLISH), bibliography.region.flatten());

    // Add hidden items for everything if we should print the whole
    // bibliography.
    if bibliography.full.get(StyleChain::default()) {
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

    (rendered, successes)
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

/// Turns a bibliography rendered with hayagriva into a final
/// [`RenderedBibliography`].
fn show_bibliography(
    world: Tracked<dyn World + '_>,
    bibliography: &Packed<BibliographyElem>,
    groups: &[Content],
    rendered: &hayagriva::RenderedBibliography,
) -> StrResult<RenderedBibliography> {
    let to_citations = links_to_citations(groups);

    let mut entries = vec![];
    for (k, item) in rendered.items.iter().enumerate() {
        let ctx = ShowCtx {
            world,
            span: bibliography.span(),
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

        entries.push(RenderedEntry {
            prefix,
            body,
            backlink: entry_location(bibliography, k),
        });
    }

    Ok(RenderedBibliography { entries, hanging_indent: rendered.hanging_indent })
}

/// Produces the content for a [`CiteGroup`].
fn show_cite_group(
    world: Tracked<dyn World + '_>,
    bibliography: &Packed<BibliographyElem>,
    group: &Packed<CiteGroup>,
    citation: &hayagriva::RenderedCitation,
    to_entries: &FxHashMap<&str, Location>,
) -> SourceResult<Content> {
    if group
        .children
        .iter()
        .all(|sub| sub.form.get(StyleChain::default()).is_none())
    {
        return Ok(Content::empty());
    }

    let supplement =
        |i: usize| group.children.get(i)?.supplement.get_cloned(StyleChain::default());
    let link =
        |i: usize| to_entries.get(group.children.get(i)?.key.resolve().as_str()).copied();

    let ctx = ShowCtx {
        world,
        span: group.span(),
        supplement: &supplement,
        link: &link,
    };

    let mut realized =
        show_elem_children(&ctx, &citation.citation, None, true).at(group.span())?;

    if resolve_style(group, bibliography).get().settings.class
        == citationberg::StyleClass::Note
        && group.children.iter().all(|sub| {
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

/// Determines the CSL style to use for a citation group.
fn resolve_style<'a>(
    group: &'a Packed<CiteGroup>,
    bibliography: &'a Packed<BibliographyElem>,
) -> &'a CslStyle {
    if let Some(first) = group.children.first()
        && let Smart::Custom(style) = first.style.get_ref(StyleChain::default())
    {
        &style.derived
    } else {
        &bibliography.style.get_ref(StyleChain::default()).derived
    }
}

/// Creates a map that links from citation keys to the first citation group
/// that contains the key.
///
/// This is used by bibliography entries to link back to the first citation that
/// references them.
fn links_to_citations(groups: &[Content]) -> FxHashMap<ResolvedPicoStr, Location> {
    let mut map = FxHashMap::default();
    for group in groups {
        let group = group.to_packed::<CiteGroup>().unwrap();
        let loc = group.location().unwrap();
        for child in &group.children {
            let key = child.key.resolve();
            map.entry(key).or_insert(loc);
        }
    }
    map
}

/// Creates a map that links from citation keys to the corresponding entry in
/// the bibliography.
fn links_to_entries<'a>(
    bibliography: &Packed<BibliographyElem>,
    rendered: &'a hayagriva::RenderedBibliography,
) -> FxHashMap<&'a str, Location> {
    let mut links = FxHashMap::default();
    for (k, item) in rendered.items.iter().enumerate() {
        links.insert(item.key.as_str(), entry_location(bibliography, k));
    }
    links
}

/// Each reference is assigned a manually created well-known location that is
/// derived from the bibliography's location. This way, citations can link to
/// them without having to query for them (which would incur an extra layout
/// iteration).
fn entry_location(bibliography: &Packed<BibliographyElem>, k: usize) -> Location {
    bibliography.location().unwrap().variant(k + 1)
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

/// Retrieves all citation groups in the document.
///
/// This is separate from `QueryIntrospection` so that we can customize the
/// diagnostic as the `CiteGroup` is internal. The default query message is also
/// not that helpful in this case.
#[derive(Debug, Clone, PartialEq, Hash)]
struct CiteGroupIntrospection(Span);

impl Introspect for CiteGroupIntrospection {
    type Output = EcoVec<Content>;

    fn introspect(
        &self,
        _: &mut Engine,
        introspector: Tracked<dyn Introspector + '_>,
    ) -> Self::Output {
        introspector.query(&CiteGroup::ELEM.select())
    }

    fn diagnose(&self, _: &History<Self::Output>) -> SourceDiagnostic {
        warning!(
            self.0, "citation grouping did not stabilize";
            hint: "this can happen if the citations and bibliographies in the \
                   document did not stabilize by the end of the third layout iteration";
        )
    }
}

/// The diagnostic when a citation wasn't found in the pre-formatted list.
fn failed_to_format_citation() -> HintedString {
    error!(
        "cannot format citation in isolation";
        hint: "check whether this citation is measured \
               without being inserted into the document";
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
