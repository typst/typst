use std::collections::HashMap;
use std::ffi::OsStr;
use std::fmt::{self, Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::num::NonZeroUsize;
use std::path::Path;
use std::sync::{Arc, LazyLock};

use comemo::Tracked;
use ecow::{eco_format, EcoString, EcoVec};
use hayagriva::archive::ArchivedStyle;
use hayagriva::io::BibLaTeXError;
use hayagriva::{
    citationberg, BibliographyDriver, BibliographyRequest, CitationItem, CitationRequest,
    SpecificLocator,
};
use indexmap::IndexMap;
use smallvec::{smallvec, SmallVec};
use typed_arena::Arena;
use typst_syntax::{Span, Spanned};
use typst_utils::{LazyHash, NonZeroExt, PicoStr};

use crate::diag::{bail, error, At, FileError, HintedStrResult, SourceResult, StrResult};
use crate::engine::Engine;
use crate::foundations::{
    cast, elem, ty, Args, Array, Bytes, CastInfo, Content, FromValue, IntoValue, Label,
    NativeElement, Packed, Reflect, Repr, Scope, Show, ShowSet, Smart, Str, StyleChain,
    Styles, Synthesize, Type, Value,
};
use crate::introspection::{Introspector, Locatable, Location};
use crate::layout::{
    BlockBody, BlockElem, Em, GridCell, GridChild, GridElem, GridItem, HElem, PadElem,
    Sizing, TrackSizings, VElem,
};
use crate::model::{
    CitationForm, CiteGroup, Destination, FootnoteElem, HeadingElem, LinkElem,
    LinkTarget, ParElem, Url,
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
    /// Path(s) to Hayagriva `.yml` and/or BibLaTeX `.bib` files.
    #[required]
    #[parse(
        let (paths, bibliography) = Bibliography::parse(engine, args)?;
        paths
    )]
    pub path: BibliographyPaths,

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
    /// Should be either one of the built-in styles (see below) or a path to
    /// a [CSL file](https://citationstyles.org/). Some of the styles listed
    /// below appear twice, once with their full name and once with a short
    /// alias.
    #[parse(CslStyle::parse(engine, args)?)]
    #[default(CslStyle::from_name("ieee").unwrap())]
    pub style: CslStyle,

    /// The loaded bibliography.
    #[internal]
    #[required]
    #[parse(bibliography)]
    pub bibliography: Bibliography,

    /// The language setting where the bibliography is.
    #[internal]
    #[synthesized]
    pub lang: Lang,

    /// The region setting where the bibliography is.
    #[internal]
    #[synthesized]
    pub region: Option<Region>,
}

/// A list of bibliography file paths.
#[derive(Debug, Default, Clone, Eq, PartialEq, Hash)]
pub struct BibliographyPaths(Vec<EcoString>);

cast! {
    BibliographyPaths,
    self => self.0.into_value(),
    v: EcoString => Self(vec![v]),
    v: Array => Self(v.into_iter().map(Value::cast).collect::<HintedStrResult<_>>()?),
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
    pub fn has(engine: &Engine, key: impl Into<PicoStr>) -> bool {
        let key = key.into();
        engine
            .introspector
            .query(&Self::elem().select())
            .iter()
            .any(|elem| elem.to_packed::<Self>().unwrap().bibliography().has(key))
    }

    /// Find all bibliography keys.
    pub fn keys(introspector: Tracked<Introspector>) -> Vec<(Label, Option<EcoString>)> {
        let mut vec = vec![];
        for elem in introspector.query(&Self::elem().select()).iter() {
            let this = elem.to_packed::<Self>().unwrap();
            for (key, entry) in this.bibliography().iter() {
                let detail = entry.title().map(|title| title.value.to_str().into());
                vec.push((Label::new(key), detail))
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
#[derive(Clone, PartialEq)]
pub struct Bibliography {
    map: Arc<IndexMap<PicoStr, hayagriva::Entry>>,
    hash: u128,
}

impl Bibliography {
    /// Parse the bibliography argument.
    fn parse(
        engine: &mut Engine,
        args: &mut Args,
    ) -> SourceResult<(BibliographyPaths, Bibliography)> {
        let Spanned { v: paths, span } =
            args.expect::<Spanned<BibliographyPaths>>("path to bibliography file")?;

        // Load bibliography files.
        let data = paths
            .0
            .iter()
            .map(|path| {
                let id = span.resolve_path(path).at(span)?;
                engine.world.file(id).at(span)
            })
            .collect::<SourceResult<Vec<Bytes>>>()?;

        // Parse.
        let bibliography = Self::load(&paths, &data).at(span)?;

        Ok((paths, bibliography))
    }

    /// Load bibliography entries from paths.
    #[comemo::memoize]
    #[typst_macros::time(name = "load bibliography")]
    fn load(paths: &BibliographyPaths, data: &[Bytes]) -> StrResult<Bibliography> {
        let mut map = IndexMap::new();
        let mut duplicates = Vec::<EcoString>::new();

        // We might have multiple bib/yaml files
        for (path, bytes) in paths.0.iter().zip(data) {
            let src = std::str::from_utf8(bytes).map_err(FileError::from)?;

            let ext = Path::new(path.as_str())
                .extension()
                .and_then(OsStr::to_str)
                .unwrap_or_default();

            let library = match ext.to_lowercase().as_str() {
                "yml" | "yaml" => hayagriva::io::from_yaml_str(src)
                    .map_err(|err| eco_format!("failed to parse YAML ({err})"))?,
                "bib" => hayagriva::io::from_biblatex_str(src)
                    .map_err(|errors| format_biblatex_error(path, src, errors))?,
                _ => bail!("unknown bibliography format (must be .yml/.yaml or .bib)"),
            };

            for entry in library {
                match map.entry(PicoStr::intern(entry.key())) {
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

        Ok(Bibliography {
            map: Arc::new(map),
            hash: typst_utils::hash128(data),
        })
    }

    fn has(&self, key: impl Into<PicoStr>) -> bool {
        self.map.contains_key(&key.into())
    }

    fn iter(&self) -> impl Iterator<Item = (PicoStr, &hayagriva::Entry)> {
        self.map.iter().map(|(&k, v)| (k, v))
    }
}

impl Debug for Bibliography {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_set().entries(self.map.keys()).finish()
    }
}

impl Hash for Bibliography {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.hash.hash(state);
    }
}

/// Format a BibLaTeX loading error.
fn format_biblatex_error(path: &str, src: &str, errors: Vec<BibLaTeXError>) -> EcoString {
    let Some(error) = errors.first() else {
        return eco_format!("failed to parse BibLaTeX file ({path})");
    };

    let (span, msg) = match error {
        BibLaTeXError::Parse(error) => (&error.span, error.kind.to_string()),
        BibLaTeXError::Type(error) => (&error.span, error.kind.to_string()),
    };
    let line = src.get(..span.start).unwrap_or_default().lines().count();
    eco_format!("failed to parse BibLaTeX file ({path}:{line}: {msg})")
}

/// A loaded CSL style.
#[ty(cast)]
#[derive(Debug, Clone, PartialEq, Hash)]
pub struct CslStyle {
    name: Option<EcoString>,
    style: Arc<LazyHash<citationberg::IndependentStyle>>,
}

impl CslStyle {
    /// Parse the style argument.
    pub fn parse(engine: &mut Engine, args: &mut Args) -> SourceResult<Option<CslStyle>> {
        let Some(Spanned { v: string, span }) =
            args.named::<Spanned<EcoString>>("style")?
        else {
            return Ok(None);
        };

        Ok(Some(Self::parse_impl(engine, &string, span).at(span)?))
    }

    /// Parse the style argument with `Smart`.
    pub fn parse_smart(
        engine: &mut Engine,
        args: &mut Args,
    ) -> SourceResult<Option<Smart<CslStyle>>> {
        let Some(Spanned { v: smart, span }) =
            args.named::<Spanned<Smart<EcoString>>>("style")?
        else {
            return Ok(None);
        };

        Ok(Some(match smart {
            Smart::Auto => Smart::Auto,
            Smart::Custom(string) => {
                Smart::Custom(Self::parse_impl(engine, &string, span).at(span)?)
            }
        }))
    }

    /// Parse internally.
    fn parse_impl(engine: &mut Engine, string: &str, span: Span) -> StrResult<CslStyle> {
        let ext = Path::new(string)
            .extension()
            .and_then(OsStr::to_str)
            .unwrap_or_default()
            .to_lowercase();

        if ext == "csl" {
            let id = span.resolve_path(string)?;
            let data = engine.world.file(id)?;
            CslStyle::from_data(&data)
        } else {
            CslStyle::from_name(string)
        }
    }

    /// Load a built-in CSL style.
    #[comemo::memoize]
    pub fn from_name(name: &str) -> StrResult<CslStyle> {
        match hayagriva::archive::ArchivedStyle::by_name(name).map(ArchivedStyle::get) {
            Some(citationberg::Style::Independent(style)) => Ok(Self {
                name: Some(name.into()),
                style: Arc::new(LazyHash::new(style)),
            }),
            _ => bail!("unknown style: `{name}`"),
        }
    }

    /// Load a CSL style from file contents.
    #[comemo::memoize]
    pub fn from_data(data: &Bytes) -> StrResult<CslStyle> {
        let text = std::str::from_utf8(data.as_slice()).map_err(FileError::from)?;
        citationberg::IndependentStyle::from_xml(text)
            .map(|style| Self { name: None, style: Arc::new(LazyHash::new(style)) })
            .map_err(|err| eco_format!("failed to load CSL style ({err})"))
    }

    /// Get the underlying independent style.
    pub fn get(&self) -> &citationberg::IndependentStyle {
        self.style.as_ref()
    }
}

// This Reflect impl is technically a bit wrong because it doesn't say what
// FromValue and IntoValue really do. Instead, it says what the `style` argument
// on `bibliography` and `cite` expect (through manual parsing).
impl Reflect for CslStyle {
    #[comemo::memoize]
    fn input() -> CastInfo {
        let ty = std::iter::once(CastInfo::Type(Type::of::<Str>()));
        let options = hayagriva::archive::ArchivedStyle::all().iter().map(|name| {
            CastInfo::Value(name.names()[0].into_value(), name.display_name())
        });
        CastInfo::Union(ty.chain(options).collect())
    }

    fn output() -> CastInfo {
        EcoString::output()
    }

    fn castable(value: &Value) -> bool {
        if let Value::Dyn(dynamic) = &value {
            if dynamic.is::<Self>() {
                return true;
            }
        }

        false
    }
}

impl FromValue for CslStyle {
    fn from_value(value: Value) -> HintedStrResult<Self> {
        if let Value::Dyn(dynamic) = &value {
            if let Some(concrete) = dynamic.downcast::<Self>() {
                return Ok(concrete.clone());
            }
        }

        Err(<Self as Reflect>::error(&value))
    }
}

impl IntoValue for CslStyle {
    fn into_value(self) -> Value {
        Value::dynamic(self)
    }
}

impl Repr for CslStyle {
    fn repr(&self) -> EcoString {
        self.name
            .as_ref()
            .map(|name| name.repr())
            .unwrap_or_else(|| "..".into())
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

        let database = self.bibliography.bibliography();
        let bibliography_style = self.bibliography.style(StyleChain::default());
        let styles = Arena::new();

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
                let Some(entry) = database.map.get(&key.into_inner()) else {
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
                Smart::Auto => &bibliography_style.style,
                Smart::Custom(style) => styles.alloc(style.style),
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
            for entry in database.map.values() {
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
                        content = LinkElem::new(LinkTarget::Dest(dest), content)
                            .pack()
                            .spanned(renderer.span);
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
                content = LinkElem::new(LinkTarget::Dest(dest), content)
                    .pack()
                    .spanned(self.span);
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
