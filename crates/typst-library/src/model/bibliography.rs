use std::any::TypeId;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::fmt::{self, Debug, Formatter};
use std::num::NonZeroUsize;
use std::path::Path;
use std::sync::{Arc, LazyLock};

use comemo::Tracked;
use ecow::{eco_format, EcoString, EcoVec};
use hayagriva::archive::ArchivedStyle;
use hayagriva::io::BibLaTeXError;
use hayagriva::{
    citationberg, BibliographyDriver, BibliographyRequest, CitationItem, CitationRequest,
    Library, SpecificLocator,
};
use indexmap::IndexMap;
use smallvec::{smallvec, SmallVec};
use typst_syntax::{Span, Spanned};
use typst_utils::{ManuallyHash, NonZeroExt, PicoStr};

use crate::diag::{bail, error, At, FileError, HintedStrResult, SourceResult, StrResult};
use crate::engine::Engine;
use crate::foundations::{
    elem, Bytes, CastInfo, Content, Derived, FromValue, IntoValue, Label, NativeElement,
    OneOrMultiple, Packed, Reflect, Scope, Show, ShowSet, Smart, StyleChain, Styles,
    Synthesize, Value,
};
use crate::introspection::{Introspector, Locatable, Location};
use crate::layout::{
    BlockBody, BlockElem, Em, GridCell, GridChild, GridElem, GridItem, HElem, PadElem,
    Sizing, TrackSizings, VElem,
};
use crate::loading::{DataSource, Load};
use crate::model::{
    CitationForm, CiteGroup, Destination, FootnoteElem, HeadingElem, LinkElem, ParElem,
    Url,
};
use crate::routines::{EvalMode, Routines};
use crate::text::{
    FontStyle, Lang, LocalName, Region, SubElem, SuperElem, TextElem, WeightDelta,
};
use crate::World;

/// A bibliography / reference listing.
///
/// You can create a new bibliography by calling this function with a path
/// to a bibliography file in either one of two formats:
///
/// - A Hayagriva `.yml` file. Hayagriva is a new bibliography file format
///   designed for use with Typst. Visit its
///   [documentation](https://github.com/typst/hayagriva/blob/main/docs/file-format.md)
///   for more details.
/// - A BibLaTeX `.bib` file.
///
/// As soon as you add a bibliography somewhere in your document, you can start
/// citing things with reference syntax (`[@key]`) or explicit calls to the
/// [citation]($cite) function (`[#cite(<key>)]`). The bibliography will only
/// show entries for works that were referenced in the document.
///
/// # Styles
/// Typst offers a wide selection of built-in
/// [citation and bibliography styles]($bibliography.style). Beyond those, you
/// can add and use custom [CSL](https://citationstyles.org/) (Citation Style
/// Language) files. Wondering which style to use? Here are some good defaults
/// based on what discipline you're working in:
///
/// | Fields          | Typical Styles                                         |
/// |-----------------|--------------------------------------------------------|
/// | Engineering, IT | `{"ieee"}`                                             |
/// | Psychology, Life Sciences | `{"apa"}`                                    |
/// | Social sciences | `{"chicago-author-date"}`                              |
/// | Humanities      | `{"mla"}`, `{"chicago-notes"}`, `{"harvard-cite-them-right"}` |
/// | Economics       | `{"harvard-cite-them-right"}`                          |
/// | Physics         | `{"american-physics-society"}`                         |
///
/// # Example
/// ```example
/// This was already noted by
/// pirates long ago. @arrgh
///
/// Multiple sources say ...
/// @arrgh @netwok.
///
/// #bibliography("works.bib")
/// ```
#[elem(Locatable, Synthesize, Show, ShowSet, LocalName)]
pub struct BibliographyElem {
    /// One or multiple paths to or raw bytes for Hayagriva `.yml` and/or
    /// BibLaTeX `.bib` files.
    ///
    /// This can be a:
    /// - A path string to load a bibliography file from the given path. For
    ///   more details about paths, see the [Paths section]($syntax/#paths).
    /// - Raw bytes from which the bibliography should be decoded.
    /// - An array where each item is one the above.
    #[required]
    #[parse(
        let sources = args.expect("sources")?;
        Bibliography::load(engine.world, sources)?
    )]
    pub sources: Derived<OneOrMultiple<DataSource>, Bibliography>,

    /// The title of the bibliography.
    ///
    /// - When set to `{auto}`, an appropriate title for the
    ///   [text language]($text.lang) will be used. This is the default.
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
    /// also use the `cite` function with [`form`]($cite.form) set to `{none}`.
    #[default(false)]
    pub full: bool,

    /// The bibliography style.
    ///
    /// This can be:
    /// - A string with the name of one of the built-in styles (see below). Some
    ///   of the styles listed below appear twice, once with their full name and
    ///   once with a short alias.
    /// - A path string to a [CSL file](https://citationstyles.org/). For more
    ///   details about paths, see the [Paths section]($syntax/#paths).
    /// - Raw bytes from which a CSL style should be decoded.
    #[parse(match args.named::<Spanned<CslSource>>("style")? {
        Some(source) => Some(CslStyle::load(engine.world, source)?),
        None => None,
    })]
    #[default({
        let default = ArchivedStyle::InstituteOfElectricalAndElectronicsEngineers;
        Derived::new(CslSource::Named(default), CslStyle::from_archived(default))
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
    pub fn find(introspector: Tracked<Introspector>) -> StrResult<Packed<Self>> {
        let query = introspector.query(&Self::elem().select());
        let mut iter = query.iter();
        let Some(elem) = iter.next() else {
            bail!("the document does not contain a bibliography");
        };

        if iter.next().is_some() {
            bail!("multiple bibliographies are not yet supported");
        }

        Ok(elem.to_packed::<Self>().unwrap().clone())
    }

    /// Whether the bibliography contains the given key.
    pub fn has(engine: &Engine, key: Label) -> bool {
        engine
            .introspector
            .query(&Self::elem().select())
            .iter()
            .any(|elem| elem.to_packed::<Self>().unwrap().sources.derived.has(key))
    }

    /// Find all bibliography keys.
    pub fn keys(introspector: Tracked<Introspector>) -> Vec<(Label, Option<EcoString>)> {
        let mut vec = vec![];
        for elem in introspector.query(&Self::elem().select()).iter() {
            let this = elem.to_packed::<Self>().unwrap();
            for (key, entry) in this.sources.derived.iter() {
                let detail = entry.title().map(|title| title.value.to_str().into());
                vec.push((key, detail))
            }
        }
        vec
    }
}

impl Synthesize for Packed<BibliographyElem> {
    fn synthesize(&mut self, _: &mut Engine, styles: StyleChain) -> SourceResult<()> {
        let elem = self.as_mut();
        elem.push_lang(TextElem::lang_in(styles));
        elem.push_region(TextElem::region_in(styles));
        Ok(())
    }
}

impl Show for Packed<BibliographyElem> {
    #[typst_macros::time(name = "bibliography", span = self.span())]
    fn show(&self, engine: &mut Engine, styles: StyleChain) -> SourceResult<Content> {
        const COLUMN_GUTTER: Em = Em::new(0.65);
        const INDENT: Em = Em::new(1.5);

        let mut seq = vec![];
        if let Some(title) = self.title(styles).unwrap_or_else(|| {
            Some(TextElem::packed(Self::local_name_in(styles)).spanned(self.span()))
        }) {
            seq.push(
                HeadingElem::new(title)
                    .with_depth(NonZeroUsize::ONE)
                    .pack()
                    .spanned(self.span()),
            );
        }

        let span = self.span();
        let works = Works::generate(engine).at(span)?;
        let references = works
            .references
            .as_ref()
            .ok_or("CSL style is not suitable for bibliographies")
            .at(span)?;

        let row_gutter = ParElem::spacing_in(styles);
        let row_gutter_elem = VElem::new(row_gutter.into()).with_weak(true).pack();

        if references.iter().any(|(prefix, _)| prefix.is_some()) {
            let mut cells = vec![];
            for (prefix, reference) in references {
                cells.push(GridChild::Item(GridItem::Cell(
                    Packed::new(GridCell::new(prefix.clone().unwrap_or_default()))
                        .spanned(span),
                )));
                cells.push(GridChild::Item(GridItem::Cell(
                    Packed::new(GridCell::new(reference.clone())).spanned(span),
                )));
            }
            seq.push(
                GridElem::new(cells)
                    .with_columns(TrackSizings(smallvec![Sizing::Auto; 2]))
                    .with_column_gutter(TrackSizings(smallvec![COLUMN_GUTTER.into()]))
                    .with_row_gutter(TrackSizings(smallvec![row_gutter.into()]))
                    .pack()
                    .spanned(self.span()),
            );
        } else {
            for (i, (_, reference)) in references.iter().enumerate() {
                if i > 0 {
                    seq.push(row_gutter_elem.clone());
                }
                seq.push(reference.clone());
            }
        }

        let mut content = Content::sequence(seq);
        if works.hanging_indent {
            content = content.styled(ParElem::set_hanging_indent(INDENT.into()));
        }

        Ok(content)
    }
}

impl ShowSet for Packed<BibliographyElem> {
    fn show_set(&self, _: StyleChain) -> Styles {
        const INDENT: Em = Em::new(1.0);
        let mut out = Styles::new();
        out.set(HeadingElem::set_numbering(None));
        out.set(PadElem::set_left(INDENT.into()));
        out
    }
}

impl LocalName for Packed<BibliographyElem> {
    const KEY: &'static str = "bibliography";
}

/// A loaded bibliography.
#[derive(Clone, PartialEq, Hash)]
pub struct Bibliography(Arc<ManuallyHash<IndexMap<Label, hayagriva::Entry>>>);

impl Bibliography {
    /// Load a bibliography from data sources.
    fn load(
        world: Tracked<dyn World + '_>,
        sources: Spanned<OneOrMultiple<DataSource>>,
    ) -> SourceResult<Derived<OneOrMultiple<DataSource>, Self>> {
        let data = sources.load(world)?;
        let bibliography = Self::decode(&sources.v, &data).at(sources.span)?;
        Ok(Derived::new(sources.v, bibliography))
    }

    /// Decode a bibliography from loaded data sources.
    #[comemo::memoize]
    #[typst_macros::time(name = "load bibliography")]
    fn decode(
        sources: &OneOrMultiple<DataSource>,
        data: &[Bytes],
    ) -> StrResult<Bibliography> {
        let mut map = IndexMap::new();
        let mut duplicates = Vec::<EcoString>::new();

        // We might have multiple bib/yaml files
        for (source, data) in sources.0.iter().zip(data) {
            let library = decode_library(source, data)?;
            for entry in library {
                match map.entry(Label::new(PicoStr::intern(entry.key()))) {
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
            bail!("duplicate bibliography keys: {}", duplicates.join(", "));
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
fn decode_library(source: &DataSource, data: &Bytes) -> StrResult<Library> {
    let src = data.as_str().map_err(FileError::from)?;

    if let DataSource::Path(path) = source {
        // If we got a path, use the extension to determine whether it is
        // YAML or BibLaTeX.
        let ext = Path::new(path.as_str())
            .extension()
            .and_then(OsStr::to_str)
            .unwrap_or_default();

        match ext.to_lowercase().as_str() {
            "yml" | "yaml" => hayagriva::io::from_yaml_str(src)
                .map_err(|err| eco_format!("failed to parse YAML ({err})")),
            "bib" => hayagriva::io::from_biblatex_str(src)
                .map_err(|errors| format_biblatex_error(src, Some(path), errors)),
            _ => bail!("unknown bibliography format (must be .yml/.yaml or .bib)"),
        }
    } else {
        // If we just got bytes, we need to guess. If it can be decoded as
        // hayagriva YAML, we'll use that.
        let haya_err = match hayagriva::io::from_yaml_str(src) {
            Ok(library) => return Ok(library),
            Err(err) => err,
        };

        // If it can be decoded as BibLaTeX, we use that isntead.
        let bib_errs = match hayagriva::io::from_biblatex_str(src) {
            Ok(library) => return Ok(library),
            Err(err) => err,
        };

        // If neither decoded correctly, check whether `:` or `{` appears
        // more often to guess whether it's more likely to be YAML or BibLaTeX
        // and emit the more appropriate error.
        let mut yaml = 0;
        let mut biblatex = 0;
        for c in src.chars() {
            match c {
                ':' => yaml += 1,
                '{' => biblatex += 1,
                _ => {}
            }
        }

        if yaml > biblatex {
            bail!("failed to parse YAML ({haya_err})")
        } else {
            Err(format_biblatex_error(src, None, bib_errs))
        }
    }
}

/// Format a BibLaTeX loading error.
fn format_biblatex_error(
    src: &str,
    path: Option<&str>,
    errors: Vec<BibLaTeXError>,
) -> EcoString {
    let Some(error) = errors.first() else {
        return match path {
            Some(path) => eco_format!("failed to parse BibLaTeX file ({path})"),
            None => eco_format!("failed to parse BibLaTeX"),
        };
    };

    let (span, msg) = match error {
        BibLaTeXError::Parse(error) => (&error.span, error.kind.to_string()),
        BibLaTeXError::Type(error) => (&error.span, error.kind.to_string()),
    };

    let line = src.get(..span.start).unwrap_or_default().lines().count();
    match path {
        Some(path) => eco_format!("failed to parse BibLaTeX file ({path}:{line}: {msg})"),
        None => eco_format!("failed to parse BibLaTeX ({line}: {msg})"),
    }
}

/// A loaded CSL style.
#[derive(Debug, Clone, PartialEq, Hash)]
pub struct CslStyle(Arc<ManuallyHash<citationberg::IndependentStyle>>);

impl CslStyle {
    /// Load a CSL style from a data source.
    pub fn load(
        world: Tracked<dyn World + '_>,
        Spanned { v: source, span }: Spanned<CslSource>,
    ) -> SourceResult<Derived<CslSource, Self>> {
        let style = match &source {
            CslSource::Named(style) => Self::from_archived(*style),
            CslSource::Normal(source) => {
                let data = Spanned::new(source, span).load(world)?;
                Self::from_data(data).at(span)?
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
            _ => unreachable!("archive should not contain dependant styles"),
        }
    }

    /// Load a CSL style from file contents.
    #[comemo::memoize]
    pub fn from_data(data: Bytes) -> StrResult<CslStyle> {
        let text = data.as_str().map_err(FileError::from)?;
        citationberg::IndependentStyle::from_xml(text)
            .map(|style| {
                Self(Arc::new(ManuallyHash::new(
                    style,
                    typst_utils::hash128(&(TypeId::of::<Bytes>(), data)),
                )))
            })
            .map_err(|err| eco_format!("failed to load CSL style ({err})"))
    }

    /// Get the underlying independent style.
    pub fn get(&self) -> &citationberg::IndependentStyle {
        self.0.as_ref()
    }
}

/// Source for a CSL style.
#[derive(Debug, Clone, PartialEq, Hash)]
pub enum CslSource {
    /// A predefined named style.
    Named(ArchivedStyle),
    /// A normal data source.
    Normal(DataSource),
}

impl Reflect for CslSource {
    #[comemo::memoize]
    fn input() -> CastInfo {
        let source = std::iter::once(DataSource::input());
        let names = ArchivedStyle::all().iter().map(|name| {
            CastInfo::Value(name.names()[0].into_value(), name.display_name())
        });
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
                let style = ArchivedStyle::by_name(&string)
                    .ok_or_else(|| eco_format!("unknown style: {}", string))?;
                return Ok(CslSource::Named(style));
            }
        }

        DataSource::from_value(value).map(CslSource::Normal)
    }
}

impl IntoValue for CslSource {
    fn into_value(self) -> Value {
        match self {
            // We prefer the shorter names which are at the back of the array.
            Self::Named(v) => v.names().last().unwrap().into_value(),
            Self::Normal(v) => v.into_value(),
        }
    }
}

/// Fully formatted citations and references, generated once (through
/// memoization) for the whole document. This setup is necessary because
/// citation formatting is inherently stateful and we need access to all
/// citations to do it.
pub(super) struct Works {
    /// Maps from the location of a citation group to its rendered content.
    pub citations: HashMap<Location, SourceResult<Content>>,
    /// Lists all references in the bibliography, with optional prefix, or
    /// `None` if the citation style can't be used for bibliographies.
    pub references: Option<Vec<(Option<Content>, Content)>>,
    /// Whether the bibliography should have hanging indent.
    pub hanging_indent: bool,
}

impl Works {
    /// Generate all citations and the whole bibliography.
    pub fn generate(engine: &Engine) -> StrResult<Arc<Works>> {
        Self::generate_impl(engine.routines, engine.world, engine.introspector)
    }

    /// The internal implementation of [`Works::generate`].
    #[comemo::memoize]
    fn generate_impl(
        routines: &Routines,
        world: Tracked<dyn World + '_>,
        introspector: Tracked<Introspector>,
    ) -> StrResult<Arc<Works>> {
        let mut generator = Generator::new(routines, world, introspector)?;
        let rendered = generator.drive();
        let works = generator.display(&rendered)?;
        Ok(Arc::new(works))
    }
}

/// Context for generating the bibliography.
struct Generator<'a> {
    /// The routines that is used to evaluate mathematical material in citations.
    routines: &'a Routines,
    /// The world that is used to evaluate mathematical material in citations.
    world: Tracked<'a, dyn World + 'a>,
    /// The document's bibliography.
    bibliography: Packed<BibliographyElem>,
    /// The document's citation groups.
    groups: EcoVec<Content>,
    /// Details about each group that are accumulated while driving hayagriva's
    /// bibliography driver and needed when processing hayagriva's output.
    infos: Vec<GroupInfo>,
    /// Citations with unresolved keys.
    failures: HashMap<Location, SourceResult<Content>>,
}

/// Details about a group of merged citations. All citations are put into groups
/// of adjacent ones (e.g., `@foo @bar` will merge into a group of length two).
/// Even single citations will be put into groups of length ones.
struct GroupInfo {
    /// The group's location.
    location: Location,
    /// The group's span.
    span: Span,
    /// Whether the group should be displayed in a footnote.
    footnote: bool,
    /// Details about the groups citations.
    subinfos: SmallVec<[CiteInfo; 1]>,
}

/// Details about a citation item in a request.
struct CiteInfo {
    /// The citation's key.
    key: Label,
    /// The citation's supplement.
    supplement: Option<Content>,
    /// Whether this citation was hidden.
    hidden: bool,
}

impl<'a> Generator<'a> {
    /// Create a new generator.
    fn new(
        routines: &'a Routines,
        world: Tracked<'a, dyn World + 'a>,
        introspector: Tracked<Introspector>,
    ) -> StrResult<Self> {
        let bibliography = BibliographyElem::find(introspector)?;
        let groups = introspector.query(&CiteGroup::elem().select());
        let infos = Vec::with_capacity(groups.len());
        Ok(Self {
            routines,
            world,
            bibliography,
            groups,
            infos,
            failures: HashMap::new(),
        })
    }

    /// Drives hayagriva's citation driver.
    fn drive(&mut self) -> hayagriva::Rendered {
        static LOCALES: LazyLock<Vec<citationberg::Locale>> =
            LazyLock::new(hayagriva::archive::locales);

        let database = &self.bibliography.sources.derived;
        let bibliography_style = &self.bibliography.style(StyleChain::default()).derived;

        // Process all citation groups.
        let mut driver = BibliographyDriver::new();
        for elem in &self.groups {
            let group = elem.to_packed::<CiteGroup>().unwrap();
            let location = elem.location().unwrap();
            let children = group.children();

            // Groups should never be empty.
            let Some(first) = children.first() else { continue };

            let mut subinfos = SmallVec::with_capacity(children.len());
            let mut items = Vec::with_capacity(children.len());
            let mut errors = EcoVec::new();
            let mut normal = true;

            // Create infos and items for each child in the group.
            for child in children {
                let key = *child.key();
                let Some(entry) = database.get(key) else {
                    errors.push(error!(
                        child.span(),
                        "key `{}` does not exist in the bibliography",
                        key.resolve()
                    ));
                    continue;
                };

                let supplement = child.supplement(StyleChain::default());
                let locator = supplement.as_ref().map(|_| {
                    SpecificLocator(
                        citationberg::taxonomy::Locator::Custom,
                        hayagriva::LocatorPayload::Transparent,
                    )
                });

                let mut hidden = false;
                let special_form = match child.form(StyleChain::default()) {
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

                normal &= special_form.is_none();
                subinfos.push(CiteInfo { key, supplement, hidden });
                items.push(CitationItem::new(entry, locator, None, hidden, special_form));
            }

            if !errors.is_empty() {
                self.failures.insert(location, Err(errors));
                continue;
            }

            let style = match first.style(StyleChain::default()) {
                Smart::Auto => bibliography_style.get(),
                Smart::Custom(style) => style.derived.get(),
            };

            self.infos.push(GroupInfo {
                location,
                subinfos,
                span: first.span(),
                footnote: normal
                    && style.settings.class == citationberg::StyleClass::Note,
            });

            driver.citation(CitationRequest::new(
                items,
                style,
                Some(locale(
                    first.lang().copied().unwrap_or(Lang::ENGLISH),
                    first.region().copied().flatten(),
                )),
                &LOCALES,
                None,
            ));
        }

        let locale = locale(
            self.bibliography.lang().copied().unwrap_or(Lang::ENGLISH),
            self.bibliography.region().copied().flatten(),
        );

        // Add hidden items for everything if we should print the whole
        // bibliography.
        if self.bibliography.full(StyleChain::default()) {
            for (_, entry) in database.iter() {
                driver.citation(CitationRequest::new(
                    vec![CitationItem::new(entry, None, None, true, None)],
                    bibliography_style.get(),
                    Some(locale.clone()),
                    &LOCALES,
                    None,
                ));
            }
        }

        driver.finish(BibliographyRequest {
            style: bibliography_style.get(),
            locale: Some(locale),
            locale_files: &LOCALES,
        })
    }

    /// Displays hayagriva's output as content for the citations and references.
    fn display(&mut self, rendered: &hayagriva::Rendered) -> StrResult<Works> {
        let citations = self.display_citations(rendered)?;
        let references = self.display_references(rendered)?;
        let hanging_indent =
            rendered.bibliography.as_ref().is_some_and(|b| b.hanging_indent);
        Ok(Works { citations, references, hanging_indent })
    }

    /// Display the citation groups.
    fn display_citations(
        &mut self,
        rendered: &hayagriva::Rendered,
    ) -> StrResult<HashMap<Location, SourceResult<Content>>> {
        // Determine for each citation key where in the bibliography it is,
        // so that we can link there.
        let mut links = HashMap::new();
        if let Some(bibliography) = &rendered.bibliography {
            let location = self.bibliography.location().unwrap();
            for (k, item) in bibliography.items.iter().enumerate() {
                links.insert(item.key.as_str(), location.variant(k + 1));
            }
        }

        let mut output = std::mem::take(&mut self.failures);
        for (info, citation) in self.infos.iter().zip(&rendered.citations) {
            let supplement = |i: usize| info.subinfos.get(i)?.supplement.clone();
            let link = |i: usize| {
                links.get(info.subinfos.get(i)?.key.resolve().as_str()).copied()
            };

            let renderer = ElemRenderer {
                routines: self.routines,
                world: self.world,
                span: info.span,
                supplement: &supplement,
                link: &link,
            };

            let content = if info.subinfos.iter().all(|sub| sub.hidden) {
                Content::empty()
            } else {
                let mut content = renderer.display_elem_children(
                    &citation.citation,
                    &mut None,
                    true,
                )?;

                if info.footnote {
                    content = FootnoteElem::with_content(content).pack();
                }

                content
            };

            output.insert(info.location, Ok(content));
        }

        Ok(output)
    }

    /// Display the bibliography references.
    #[allow(clippy::type_complexity)]
    fn display_references(
        &self,
        rendered: &hayagriva::Rendered,
    ) -> StrResult<Option<Vec<(Option<Content>, Content)>>> {
        let Some(rendered) = &rendered.bibliography else { return Ok(None) };

        // Determine for each citation key where it first occurred, so that we
        // can link there.
        let mut first_occurrences = HashMap::new();
        for info in &self.infos {
            for subinfo in &info.subinfos {
                let key = subinfo.key.resolve();
                first_occurrences.entry(key).or_insert(info.location);
            }
        }

        // The location of the bibliography.
        let location = self.bibliography.location().unwrap();

        let mut output = vec![];
        for (k, item) in rendered.items.iter().enumerate() {
            let renderer = ElemRenderer {
                routines: self.routines,
                world: self.world,
                span: self.bibliography.span(),
                supplement: &|_| None,
                link: &|_| None,
            };

            // Each reference is assigned a manually created well-known location
            // that is derived from the bibliography's location. This way,
            // citations can link to them.
            let backlink = location.variant(k + 1);

            // Render the first field.
            let mut prefix = item
                .first_field
                .as_ref()
                .map(|elem| {
                    let mut content =
                        renderer.display_elem_child(elem, &mut None, false)?;
                    if let Some(location) = first_occurrences.get(item.key.as_str()) {
                        let dest = Destination::Location(*location);
                        content = content.linked(dest);
                    }
                    StrResult::Ok(content)
                })
                .transpose()?;

            // Render the main reference content.
            let mut reference =
                renderer.display_elem_children(&item.content, &mut prefix, false)?;

            // Attach a backlink to either the prefix or the reference so that
            // we can link to the bibliography entry.
            prefix.as_mut().unwrap_or(&mut reference).set_location(backlink);

            output.push((prefix, reference));
        }

        Ok(Some(output))
    }
}

/// Renders hayagriva elements into content.
struct ElemRenderer<'a> {
    /// The routines that is used to evaluate mathematical material in citations.
    routines: &'a Routines,
    /// The world that is used to evaluate mathematical material.
    world: Tracked<'a, dyn World + 'a>,
    /// The span that is attached to all of the resulting content.
    span: Span,
    /// Resolves the supplement of i-th citation in the request.
    supplement: &'a dyn Fn(usize) -> Option<Content>,
    /// Resolves where the i-th citation in the request should link to.
    link: &'a dyn Fn(usize) -> Option<Location>,
}

impl ElemRenderer<'_> {
    /// Display rendered hayagriva elements.
    ///
    /// The `prefix` can be a separate content storage where `left-margin`
    /// elements will be accumulated into.
    ///
    /// `is_citation` dictates whether whitespace at the start of the citation
    /// will be eliminated. Some CSL styles yield whitespace at the start of
    /// their citations, which should instead be handled by Typst.
    fn display_elem_children(
        &self,
        elems: &hayagriva::ElemChildren,
        prefix: &mut Option<Content>,
        is_citation: bool,
    ) -> StrResult<Content> {
        Ok(Content::sequence(
            elems
                .0
                .iter()
                .enumerate()
                .map(|(i, elem)| {
                    self.display_elem_child(elem, prefix, is_citation && i == 0)
                })
                .collect::<StrResult<Vec<_>>>()?,
        ))
    }

    /// Display a rendered hayagriva element.
    fn display_elem_child(
        &self,
        elem: &hayagriva::ElemChild,
        prefix: &mut Option<Content>,
        trim_start: bool,
    ) -> StrResult<Content> {
        Ok(match elem {
            hayagriva::ElemChild::Text(formatted) => {
                self.display_formatted(formatted, trim_start)
            }
            hayagriva::ElemChild::Elem(elem) => self.display_elem(elem, prefix)?,
            hayagriva::ElemChild::Markup(markup) => self.display_math(markup),
            hayagriva::ElemChild::Link { text, url } => self.display_link(text, url)?,
            hayagriva::ElemChild::Transparent { cite_idx, format } => {
                self.display_transparent(*cite_idx, format)
            }
        })
    }

    /// Display a block-level element.
    fn display_elem(
        &self,
        elem: &hayagriva::Elem,
        prefix: &mut Option<Content>,
    ) -> StrResult<Content> {
        use citationberg::Display;

        let block_level = matches!(elem.display, Some(Display::Block | Display::Indent));

        let mut suf_prefix = None;
        let mut content = self.display_elem_children(
            &elem.children,
            if block_level { &mut suf_prefix } else { prefix },
            false,
        )?;

        if let Some(prefix) = suf_prefix {
            const COLUMN_GUTTER: Em = Em::new(0.65);
            content = GridElem::new(vec![
                GridChild::Item(GridItem::Cell(
                    Packed::new(GridCell::new(prefix)).spanned(self.span),
                )),
                GridChild::Item(GridItem::Cell(
                    Packed::new(GridCell::new(content)).spanned(self.span),
                )),
            ])
            .with_columns(TrackSizings(smallvec![Sizing::Auto; 2]))
            .with_column_gutter(TrackSizings(smallvec![COLUMN_GUTTER.into()]))
            .pack()
            .spanned(self.span);
        }

        match elem.display {
            Some(Display::Block) => {
                content = BlockElem::new()
                    .with_body(Some(BlockBody::Content(content)))
                    .pack()
                    .spanned(self.span);
            }
            Some(Display::Indent) => {
                content = PadElem::new(content).pack().spanned(self.span);
            }
            Some(Display::LeftMargin) => {
                *prefix.get_or_insert_with(Default::default) += content;
                return Ok(Content::empty());
            }
            _ => {}
        }

        if let Some(hayagriva::ElemMeta::Entry(i)) = elem.meta {
            if let Some(location) = (self.link)(i) {
                let dest = Destination::Location(location);
                content = content.linked(dest);
            }
        }

        Ok(content)
    }

    /// Display math.
    fn display_math(&self, math: &str) -> Content {
        (self.routines.eval_string)(
            self.routines,
            self.world,
            math,
            self.span,
            EvalMode::Math,
            Scope::new(),
        )
        .map(Value::display)
        .unwrap_or_else(|_| TextElem::packed(math).spanned(self.span))
    }

    /// Display a link.
    fn display_link(&self, text: &hayagriva::Formatted, url: &str) -> StrResult<Content> {
        let dest = Destination::Url(Url::new(url)?);
        Ok(LinkElem::new(dest.into(), self.display_formatted(text, false))
            .pack()
            .spanned(self.span))
    }

    /// Display transparent pass-through content.
    fn display_transparent(&self, i: usize, format: &hayagriva::Formatting) -> Content {
        let content = (self.supplement)(i).unwrap_or_default();
        apply_formatting(content, format)
    }

    /// Display formatted hayagriva text as content.
    fn display_formatted(
        &self,
        formatted: &hayagriva::Formatted,
        trim_start: bool,
    ) -> Content {
        let formatted_text = if trim_start {
            formatted.text.trim_start()
        } else {
            formatted.text.as_str()
        };

        let content = TextElem::packed(formatted_text).spanned(self.span);
        apply_formatting(content, &formatted.formatting)
    }
}

/// Applies formatting to content.
fn apply_formatting(mut content: Content, format: &hayagriva::Formatting) -> Content {
    match format.font_style {
        citationberg::FontStyle::Normal => {}
        citationberg::FontStyle::Italic => {
            content = content.styled(TextElem::set_style(FontStyle::Italic));
        }
    }

    match format.font_variant {
        citationberg::FontVariant::Normal => {}
        citationberg::FontVariant::SmallCaps => {
            content = content.styled(TextElem::set_smallcaps(true));
        }
    }

    match format.font_weight {
        citationberg::FontWeight::Normal => {}
        citationberg::FontWeight::Bold => {
            content = content.styled(TextElem::set_delta(WeightDelta(300)));
        }
        citationberg::FontWeight::Light => {
            content = content.styled(TextElem::set_delta(WeightDelta(-100)));
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
            content = HElem::hole().pack() + SuperElem::new(content).pack().spanned(span);
        }
        citationberg::VerticalAlign::Sub => {
            content = HElem::hole().pack() + SubElem::new(content).pack().spanned(span);
        }
    }

    content
}

/// Create a locale code from language and optionally region.
fn locale(lang: Lang, region: Option<Region>) -> citationberg::LocaleCode {
    let mut value = String::with_capacity(5);
    value.push_str(lang.as_str());
    if let Some(region) = region {
        value.push('-');
        value.push_str(region.as_str())
    }
    citationberg::LocaleCode(value)
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
}
