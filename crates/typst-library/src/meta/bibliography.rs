use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::Path;
use std::sync::Arc;

use ecow::{eco_vec, EcoVec};
use hayagriva::io::{BibLaTeXError, YamlBibliographyError};
use hayagriva::style::{self, Brackets, Citation, Database, DisplayString, Formatting};
use hayagriva::Entry;
use typst::diag::FileError;
use typst::eval::Bytes;
use typst::util::option_eq;

use super::{LinkElem, LocalName, RefElem};
use crate::layout::{BlockElem, GridElem, ParElem, Sizing, TrackSizings, VElem};
use crate::meta::{FootnoteElem, HeadingElem};
use crate::prelude::*;
use crate::text::TextElem;

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
/// [citation]($func/cite) function (`[#cite("key")]`). The bibliography will
/// only show entries for works that were referenced in the document.
///
/// # Example
/// ```example
/// This was already noted by
/// pirates long ago. @arrgh
///
/// Multiple sources say ...
/// #cite("arrgh", "netwok").
///
/// #bibliography("works.bib")
/// ```
///
/// Display: Bibliography
/// Category: meta
#[element(Locatable, Synthesize, Show, Finalize, LocalName)]
pub struct BibliographyElem {
    /// Path to a Hayagriva `.yml` or BibLaTeX `.bib` file.
    #[required]
    #[parse(
        let Spanned { v:  paths, span } =
            args.expect::<Spanned<BibPaths>>("path to bibliography file")?;

        // Load bibliography files.
        let data = paths.0
            .iter()
            .map(|path| {
                let id = vm.location().join(path).at(span)?;
                vm.world().file(id).at(span)
            })
            .collect::<SourceResult<Vec<Bytes>>>()?;

        // Check that parsing works.
        let _ = load(&paths, &data).at(span)?;

        paths
    )]
    pub path: BibPaths,

    /// The raw file buffers.
    #[internal]
    #[required]
    #[parse(data)]
    pub data: Vec<Bytes>,

    /// The title of the bibliography.
    ///
    /// - When set to `{auto}`, an appropriate title for the [text
    ///   language]($func/text.lang) will be used. This is the default.
    /// - When set to `{none}`, the bibliography will not have a title.
    /// - A custom title can be set by passing content.
    ///
    /// The bibliography's heading will not be numbered by default, but you can
    /// force it to be with a show-set rule:
    /// `{show bibliography: set heading(numbering: "1.")}`
    /// ```
    #[default(Some(Smart::Auto))]
    pub title: Option<Smart<Content>>,

    /// The bibliography style.
    #[default(BibliographyStyle::Ieee)]
    pub style: BibliographyStyle,
}

/// A list of bibliography file paths.
#[derive(Debug, Default, Clone, Hash)]
pub struct BibPaths(Vec<EcoString>);

cast! {
    BibPaths,
    self => self.0.into_value(),
    v: EcoString => Self(vec![v]),
    v: Array => Self(v.into_iter().map(Value::cast).collect::<StrResult<_>>()?),
}

impl BibliographyElem {
    /// Find the document's bibliography.
    pub fn find(introspector: Tracked<Introspector>) -> StrResult<Self> {
        let mut iter = introspector.query(&Self::func().select()).into_iter();
        let Some(elem) = iter.next() else {
            bail!("the document does not contain a bibliography");
        };

        if iter.next().is_some() {
            bail!("multiple bibliographies are not supported");
        }

        Ok(elem.to::<Self>().unwrap().clone())
    }

    /// Whether the bibliography contains the given key.
    pub fn has(vt: &Vt, key: &str) -> bool {
        vt.introspector
            .query(&Self::func().select())
            .into_iter()
            .flat_map(|elem| {
                let elem = elem.to::<Self>().unwrap();
                load(&elem.path(), &elem.data())
            })
            .flatten()
            .any(|entry| entry.key() == key)
    }

    /// Find all bibliography keys.
    pub fn keys(
        introspector: Tracked<Introspector>,
    ) -> Vec<(EcoString, Option<EcoString>)> {
        Self::find(introspector)
            .and_then(|elem| load(&elem.path(), &elem.data()))
            .into_iter()
            .flatten()
            .map(|entry| {
                let key = entry.key().into();
                let detail =
                    entry.title().map(|title| title.canonical.value.as_str().into());
                (key, detail)
            })
            .collect()
    }
}

impl Synthesize for BibliographyElem {
    fn synthesize(&mut self, _vt: &mut Vt, styles: StyleChain) -> SourceResult<()> {
        self.push_style(self.style(styles));
        Ok(())
    }
}

impl Show for BibliographyElem {
    #[tracing::instrument(name = "BibliographyElem::show", skip_all)]
    fn show(&self, vt: &mut Vt, styles: StyleChain) -> SourceResult<Content> {
        const COLUMN_GUTTER: Em = Em::new(0.65);
        const INDENT: Em = Em::new(1.5);

        let mut seq = vec![];
        if let Some(title) = self.title(styles) {
            let title =
                title.unwrap_or_else(|| {
                    TextElem::packed(self.local_name(
                        TextElem::lang_in(styles),
                        TextElem::region_in(styles),
                    ))
                    .spanned(self.span())
                });

            seq.push(HeadingElem::new(title).with_level(NonZeroUsize::ONE).pack());
        }

        Ok(vt.delayed(|vt| {
            let works = Works::new(vt).at(self.span())?;

            let row_gutter = BlockElem::below_in(styles).amount();
            if works.references.iter().any(|(prefix, _)| prefix.is_some()) {
                let mut cells = vec![];
                for (prefix, reference) in &works.references {
                    cells.push(prefix.clone().unwrap_or_default());
                    cells.push(reference.clone());
                }

                seq.push(VElem::new(row_gutter).with_weakness(3).pack());
                seq.push(
                    GridElem::new(cells)
                        .with_columns(TrackSizings(vec![Sizing::Auto; 2]))
                        .with_column_gutter(TrackSizings(vec![COLUMN_GUTTER.into()]))
                        .with_row_gutter(TrackSizings(vec![row_gutter.into()]))
                        .pack(),
                );
            } else {
                let mut entries = vec![];
                for (_, reference) in &works.references {
                    entries.push(VElem::new(row_gutter).with_weakness(3).pack());
                    entries.push(reference.clone());
                }

                seq.push(
                    Content::sequence(entries)
                        .styled(ParElem::set_hanging_indent(INDENT.into())),
                );
            }

            Ok(Content::sequence(seq))
        }))
    }
}

impl Finalize for BibliographyElem {
    fn finalize(&self, realized: Content, _: StyleChain) -> Content {
        realized.styled(HeadingElem::set_numbering(None))
    }
}

impl LocalName for BibliographyElem {
    fn local_name(&self, lang: Lang, region: Option<Region>) -> &'static str {
        match lang {
            Lang::ALBANIAN => "Bibliografi",
            Lang::ARABIC => "المراجع",
            Lang::BOKMÅL => "Bibliografi",
            Lang::CHINESE if option_eq(region, "TW") => "書目",
            Lang::CHINESE => "参考文献",
            Lang::CZECH => "Bibliografie",
            Lang::DANISH => "Bibliografi",
            Lang::DUTCH => "Bibliografie",
            Lang::FILIPINO => "Bibliograpiya",
            Lang::FRENCH => "Bibliographie",
            Lang::GERMAN => "Bibliographie",
            Lang::ITALIAN => "Bibliografia",
            Lang::NYNORSK => "Bibliografi",
            Lang::POLISH => "Bibliografia",
            Lang::PORTUGUESE => "Bibliografia",
            Lang::RUSSIAN => "Библиография",
            Lang::SLOVENIAN => "Literatura",
            Lang::SPANISH => "Bibliografía",
            Lang::SWEDISH => "Bibliografi",
            Lang::TURKISH => "Kaynakça",
            Lang::UKRAINIAN => "Бібліографія",
            Lang::VIETNAMESE => "Tài liệu tham khảo",
            Lang::JAPANESE => "参考文献",
            Lang::ENGLISH | _ => "Bibliography",
        }
    }
}

/// A bibliography style.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Cast)]
pub enum BibliographyStyle {
    /// Follows guidance of the American Psychological Association. Based on the
    /// 7th edition of the APA Publication Manual.
    Apa,
    /// The Chicago Author Date style. Based on the 17th edition of the Chicago
    /// Manual of Style, Chapter 15.
    ChicagoAuthorDate,
    /// The Chicago Notes style. Based on the 17th edition of the Chicago
    /// Manual of Style, Chapter 14.
    ChicagoNotes,
    /// The style of the Institute of Electrical and Electronics Engineers.
    /// Based on the 2018 IEEE Reference Guide.
    Ieee,
    /// Follows guidance of the Modern Language Association. Based on the 8th
    /// edition of the MLA Handbook.
    Mla,
}

impl BibliographyStyle {
    /// The default citation style for this bibliography style.
    pub fn default_citation_style(self) -> CitationStyle {
        match self {
            Self::Apa => CitationStyle::ChicagoAuthorDate,
            Self::ChicagoAuthorDate => CitationStyle::ChicagoAuthorDate,
            Self::ChicagoNotes => CitationStyle::ChicagoNotes,
            Self::Ieee => CitationStyle::Numerical,
            Self::Mla => CitationStyle::ChicagoAuthorDate,
        }
    }
}

/// Cite a work from the bibliography.
///
/// Before you starting citing, you need to add a
/// [bibliography]($func/bibliography) somewhere in your document.
///
/// # Example
/// ```example
/// This was already noted by
/// pirates long ago. @arrgh
///
/// Multiple sources say ...
/// #cite("arrgh", "netwok").
///
/// #bibliography("works.bib")
/// ```
///
/// # Syntax
/// This function indirectly has dedicated syntax. [References]($func/ref)
/// can be used to cite works from the bibliography. The label then
/// corresponds to the citation key.
///
/// Display: Citation
/// Category: meta
#[element(Locatable, Synthesize, Show)]
pub struct CiteElem {
    /// The citation keys that identify the elements that shall be cited in
    /// the bibliography.
    ///
    /// Reference syntax supports only a single key.
    #[variadic]
    pub keys: Vec<EcoString>,

    /// A supplement for the citation such as page or chapter number.
    ///
    /// In reference syntax, the supplement can be added in square brackets:
    ///
    /// ```example
    /// This has been proven over and
    /// over again. @distress[p.~7]
    ///
    /// #bibliography("works.bib")
    /// ```
    #[positional]
    pub supplement: Option<Content>,

    /// Whether the citation should include brackets.
    ///
    /// ```example
    /// #set cite(brackets: false)
    ///
    /// @netwok follow these methods
    /// in their work ...
    ///
    /// #bibliography(
    ///   "works.bib",
    ///   style: "chicago-author-date",
    /// )
    /// ```
    #[default(true)]
    pub brackets: bool,

    /// The citation style.
    ///
    /// When set to `{auto}`, automatically picks the preferred citation style
    /// for the bibliography's style.
    ///
    /// ```example
    /// #set cite(style: "alphanumerical")
    /// Alphanumerical references.
    /// @netwok
    ///
    /// #bibliography("works.bib")
    /// ```
    pub style: Smart<CitationStyle>,
}

impl Synthesize for CiteElem {
    fn synthesize(&mut self, _vt: &mut Vt, styles: StyleChain) -> SourceResult<()> {
        self.push_supplement(self.supplement(styles));
        self.push_brackets(self.brackets(styles));
        self.push_style(self.style(styles));
        Ok(())
    }
}

impl Show for CiteElem {
    #[tracing::instrument(name = "CiteElem::show", skip(self, vt))]
    fn show(&self, vt: &mut Vt, _: StyleChain) -> SourceResult<Content> {
        Ok(vt.delayed(|vt| {
            let works = Works::new(vt).at(self.span())?;
            let location = self.0.location().unwrap();
            works
                .citations
                .get(&location)
                .cloned()
                .flatten()
                .ok_or("bibliography does not contain this key")
                .at(self.span())
        }))
    }
}

cast! {
    CiteElem,
    v: Content => v.to::<Self>().cloned().ok_or("expected citation")?,
}

/// A citation style.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Cast)]
pub enum CitationStyle {
    /// IEEE-style numerical reference markers.
    Numerical,
    /// A simple alphanumerical style. For example, the output could be Rass97
    /// or MKG+21.
    Alphanumerical,
    /// Citations that just consist of the entry keys.
    Keys,
    /// The Chicago Author Date style. Based on the 17th edition of the Chicago
    /// Manual of Style, Chapter 15.
    ChicagoAuthorDate,
    /// The Chicago Notes style. Based on the 17th edition of the Chicago
    /// Manual of Style, Chapter 14.
    ChicagoNotes,
    /// A Chicago-like author-title format. Results could look like this:
    /// Prokopov, “It Is Fast or It Is Wrong”.
    ChicagoAuthorTitle,
}

impl CitationStyle {
    fn is_short(self) -> bool {
        matches!(self, Self::Numerical | Self::Alphanumerical | Self::Keys)
    }
}

/// Fully formatted citations and references.
#[derive(Default)]
struct Works {
    citations: HashMap<Location, Option<Content>>,
    references: Vec<(Option<Content>, Content)>,
}

impl Works {
    /// Prepare all things need to cite a work or format a bibliography.
    fn new(vt: &Vt) -> StrResult<Arc<Self>> {
        let bibliography = BibliographyElem::find(vt.introspector)?;
        let citations = vt
            .introspector
            .query(&Selector::Or(eco_vec![
                RefElem::func().select(),
                CiteElem::func().select(),
            ]))
            .into_iter()
            .map(|elem| match elem.to::<RefElem>() {
                Some(reference) => reference.citation().unwrap(),
                _ => elem.to::<CiteElem>().unwrap().clone(),
            })
            .collect();
        Ok(create(bibliography, citations))
    }
}

/// Generate all citations and the whole bibliography.
#[comemo::memoize]
fn create(bibliography: BibliographyElem, citations: Vec<CiteElem>) -> Arc<Works> {
    let span = bibliography.span();
    let entries = load(&bibliography.path(), &bibliography.data()).unwrap();
    let style = bibliography.style(StyleChain::default());
    let bib_location = bibliography.0.location().unwrap();
    let ref_location = |target: &Entry| {
        let i = entries
            .iter()
            .position(|entry| entry.key() == target.key())
            .unwrap_or_default();
        bib_location.variant(i)
    };

    let mut db = Database::new();
    let mut ids = HashMap::new();
    let mut preliminary = vec![];

    for citation in citations {
        let cite_id = citation.0.location().unwrap();
        let entries = citation
            .keys()
            .into_iter()
            .map(|key| {
                let entry = entries.iter().find(|entry| entry.key() == key)?;
                ids.entry(entry.key()).or_insert(cite_id);
                db.push(entry);
                Some(entry)
            })
            .collect::<Option<Vec<_>>>();
        preliminary.push((citation, entries));
    }

    let mut current = CitationStyle::Numerical;
    let mut citation_style: Box<dyn style::CitationStyle> =
        Box::new(style::Numerical::new());

    let citations = preliminary
        .into_iter()
        .map(|(citation, cited)| {
            let location = citation.0.location().unwrap();
            let Some(cited) = cited else { return (location, None) };

            let mut supplement = citation.supplement(StyleChain::default());
            let brackets = citation.brackets(StyleChain::default());
            let style = citation
                .style(StyleChain::default())
                .unwrap_or(style.default_citation_style());

            if style != current {
                current = style;
                citation_style = match style {
                    CitationStyle::Numerical => Box::new(style::Numerical::new()),
                    CitationStyle::Alphanumerical => {
                        Box::new(style::Alphanumerical::new())
                    }
                    CitationStyle::ChicagoAuthorDate => {
                        Box::new(style::ChicagoAuthorDate::new())
                    }
                    CitationStyle::ChicagoNotes => Box::new(style::ChicagoNotes::new()),
                    CitationStyle::ChicagoAuthorTitle => {
                        Box::new(style::AuthorTitle::new())
                    }
                    CitationStyle::Keys => Box::new(style::Keys::new()),
                };
            }

            let len = cited.len();
            let mut content = Content::empty();
            for (i, entry) in cited.into_iter().enumerate() {
                let supplement = if i + 1 == len { supplement.take() } else { None };
                let mut display = db
                    .citation(
                        &mut *citation_style,
                        &[Citation {
                            entry,
                            supplement: supplement.is_some().then_some(SUPPLEMENT),
                        }],
                    )
                    .display;

                if style.is_short() {
                    display.value = display.value.replace(' ', "\u{a0}");
                }

                if brackets && len == 1 {
                    display = display.with_default_brackets(&*citation_style);
                }

                if i > 0 {
                    content += TextElem::packed(",\u{a0}");
                }

                // Format and link to the reference entry.
                content += format_display_string(&display, supplement, citation.span())
                    .linked(Destination::Location(ref_location(entry)));
            }

            if brackets && len > 1 {
                content = match citation_style.brackets() {
                    Brackets::None => content,
                    Brackets::Round => {
                        TextElem::packed('(') + content + TextElem::packed(')')
                    }
                    Brackets::Square => {
                        TextElem::packed('[') + content + TextElem::packed(']')
                    }
                };
            }

            if style == CitationStyle::ChicagoNotes {
                content = FootnoteElem::with_content(content).pack();
            }

            (location, Some(content))
        })
        .collect();

    let bibliography_style: Box<dyn style::BibliographyStyle> = match style {
        BibliographyStyle::Apa => Box::new(style::Apa::new()),
        BibliographyStyle::ChicagoAuthorDate => Box::new(style::ChicagoAuthorDate::new()),
        BibliographyStyle::ChicagoNotes => Box::new(style::ChicagoNotes::new()),
        BibliographyStyle::Ieee => Box::new(style::Ieee::new()),
        BibliographyStyle::Mla => Box::new(style::Mla::new()),
    };

    let references = db
        .bibliography(&*bibliography_style, None)
        .into_iter()
        .map(|reference| {
            let backlink = ref_location(reference.entry);
            let prefix = reference.prefix.map(|prefix| {
                // Format and link to first citation.
                let bracketed = prefix.with_default_brackets(&*citation_style);
                format_display_string(&bracketed, None, span)
                    .linked(Destination::Location(ids[reference.entry.key()]))
                    .backlinked(backlink)
            });

            let mut reference = format_display_string(&reference.display, None, span);
            if prefix.is_none() {
                reference = reference.backlinked(backlink);
            }

            (prefix, reference)
        })
        .collect();

    Arc::new(Works { citations, references })
}

/// Load bibliography entries from a path.
#[comemo::memoize]
fn load(paths: &BibPaths, data: &[Bytes]) -> StrResult<EcoVec<hayagriva::Entry>> {
    let mut result = EcoVec::new();

    // We might have multiple bib/yaml files
    for (path, bytes) in paths.0.iter().zip(data) {
        let src = std::str::from_utf8(bytes).map_err(|_| FileError::InvalidUtf8)?;
        let entries = parse_bib(path, src)?;
        result.extend(entries);
    }

    // Biblatex only checks for duplicate keys within files
    // -> We have to do this between files again
    let mut keys = result.iter().map(|r| r.key()).collect::<Vec<_>>();
    keys.sort_unstable();
    // Waiting for `slice_partition_dedup` #54279
    let mut duplicates = Vec::new();
    for pair in keys.windows(2) {
        if pair[0] == pair[1] {
            duplicates.push(pair[0]);
        }
    }

    if !duplicates.is_empty() {
        Err(eco_format!("duplicate bibliography keys: {}", duplicates.join(", ")))
    } else {
        Ok(result)
    }
}

/// Parse a bibliography file (bib/yml/yaml)
fn parse_bib(path_str: &str, src: &str) -> StrResult<Vec<hayagriva::Entry>> {
    let path = Path::new(path_str);
    let ext = path.extension().and_then(OsStr::to_str).unwrap_or_default();
    match ext.to_lowercase().as_str() {
        "yml" | "yaml" => {
            hayagriva::io::from_yaml_str(src).map_err(format_hayagriva_error)
        }
        "bib" => hayagriva::io::from_biblatex_str(src).map_err(|err| {
            err.into_iter()
                .next()
                .map(|error| format_biblatex_error(path_str, src, error))
                .unwrap_or_else(|| eco_format!("failed to parse {path_str}"))
        }),
        _ => bail!("unknown bibliography format (must be .yml/.yaml or .bib)"),
    }
}

/// Format a Hayagriva loading error.
fn format_hayagriva_error(error: YamlBibliographyError) -> EcoString {
    eco_format!("{error}")
}

/// Format a BibLaTeX loading error.
fn format_biblatex_error(path: &str, src: &str, error: BibLaTeXError) -> EcoString {
    let (span, msg) = match error {
        BibLaTeXError::Parse(error) => (error.span, error.kind.to_string()),
        BibLaTeXError::Type(error) => (error.span, error.kind.to_string()),
    };
    let line = src.get(..span.start).unwrap_or_default().lines().count();
    eco_format!("parsing failed at {path}:{line}: {msg}")
}

/// Hayagriva only supports strings, but we have a content supplement. To deal
/// with this, we pass this string to hayagriva instead of our content, find it
/// in the output and replace it with the content.
const SUPPLEMENT: &str = "cdc579c45cf3d648905c142c7082683f";

/// Format a display string into content.
fn format_display_string(
    string: &DisplayString,
    mut supplement: Option<Content>,
    span: Span,
) -> Content {
    let mut stops: Vec<_> = string
        .formatting
        .iter()
        .flat_map(|(range, _)| [range.start, range.end])
        .collect();

    if let Some(i) = string.value.find(SUPPLEMENT) {
        stops.push(i);
        stops.push(i + SUPPLEMENT.len());
    }

    stops.sort();
    stops.dedup();
    stops.push(string.value.len());

    let mut start = 0;
    let mut seq = vec![];
    for stop in stops {
        let segment = string.value.get(start..stop).unwrap_or_default();
        if segment.is_empty() {
            continue;
        }

        let mut content = if segment == SUPPLEMENT && supplement.is_some() {
            supplement.take().unwrap_or_default()
        } else {
            TextElem::packed(segment).spanned(span)
        };

        for (range, fmt) in &string.formatting {
            if !range.contains(&start) {
                continue;
            }

            content = match fmt {
                Formatting::Bold => content.strong(),
                Formatting::Italic => content.emph(),
                Formatting::Link(link) => {
                    LinkElem::new(Destination::Url(link.as_str().into()).into(), content)
                        .pack()
                }
            };
        }

        seq.push(content);
        start = stop;
    }

    Content::sequence(seq)
}
