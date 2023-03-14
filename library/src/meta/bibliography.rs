use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::Path;
use std::sync::Arc;

use ecow::EcoVec;
use hayagriva::io::{BibLaTeXError, YamlBibliographyError};
use hayagriva::style::{self, Citation, Database, DisplayString, Formatting};
use typst::font::{FontStyle, FontWeight};

use super::LocalName;
use crate::layout::{GridNode, ParNode, Sizing, TrackSizings, VNode};
use crate::meta::HeadingNode;
use crate::prelude::*;
use crate::text::{Hyphenate, TextNode};

/// A bibliography / reference listing.
///
/// Display: Bibliography
/// Category: meta
#[node(Locatable, Synthesize, Show, LocalName)]
pub struct BibliographyNode {
    /// Path to a Hayagriva `.yml` or BibLaTeX `.bib` file.
    #[required]
    #[parse(
        let Spanned { v: path, span } =
            args.expect::<Spanned<EcoString>>("path to bibliography file")?;
        let path: EcoString = vm.locate(&path).at(span)?.to_string_lossy().into();
        let _ = load(vm.world(), &path).at(span)?;
        path
    )]
    pub path: EcoString,

    /// The title of the bibliography.
    ///
    /// - When set to `{auto}`, an appropriate title for the [text
    ///   language]($func/text.lang) will be used. This is the default.
    /// - When set to `{none}`, the bibliography will not have a title.
    /// - A custom title can be set by passing content.
    #[default(Some(Smart::Auto))]
    pub title: Option<Smart<Content>>,

    /// The bibliography style.
    #[default(BibliographyStyle::Ieee)]
    pub style: BibliographyStyle,
}

impl BibliographyNode {
    /// Find the document's bibliography.
    pub fn find(introspector: Tracked<Introspector>) -> StrResult<Self> {
        let mut iter = introspector.locate(Selector::node::<Self>()).into_iter();
        let Some((_, node)) = iter.next() else {
            return Err("the document does not contain a bibliography".into());
        };

        if iter.next().is_some() {
            Err("multiple bibliographies are not supported")?;
        }

        Ok(node.to::<Self>().unwrap().clone())
    }

    /// Whether the bibliography contains the given key.
    pub fn has(vt: &Vt, key: &str) -> bool {
        vt.introspector
            .locate(Selector::node::<Self>())
            .into_iter()
            .flat_map(|(_, node)| load(vt.world(), &node.to::<Self>().unwrap().path()))
            .flatten()
            .any(|entry| entry.key() == key)
    }

    /// Find all bibliography keys.
    pub fn keys(
        world: Tracked<dyn World>,
        introspector: Tracked<Introspector>,
    ) -> Vec<(EcoString, Option<EcoString>)> {
        Self::find(introspector)
            .and_then(|node| load(world, &node.path()))
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

impl Synthesize for BibliographyNode {
    fn synthesize(&mut self, _: &Vt, styles: StyleChain) {
        self.push_style(self.style(styles));
    }
}

impl Show for BibliographyNode {
    fn show(&self, vt: &mut Vt, styles: StyleChain) -> SourceResult<Content> {
        const COLUMN_GUTTER: Em = Em::new(0.65);
        const ROW_GUTTER: Em = Em::new(1.0);
        const INDENT: Em = Em::new(1.5);

        let works = match Works::new(vt) {
            Ok(works) => works,
            Err(error) => {
                if vt.locatable() {
                    bail!(self.span(), error)
                } else {
                    return Ok(TextNode::packed("bibliography"));
                }
            }
        };

        let mut seq = vec![];
        if let Some(title) = self.title(styles) {
            let title = title.clone().unwrap_or_else(|| {
                TextNode::packed(self.local_name(TextNode::lang_in(styles)))
            });

            seq.push(
                HeadingNode::new(title)
                    .with_level(NonZeroUsize::new(1).unwrap())
                    .with_numbering(None)
                    .pack(),
            );
        }

        if works.references.iter().any(|(prefix, _)| prefix.is_some()) {
            let mut cells = vec![];
            for (prefix, reference) in &works.references {
                cells.push(prefix.clone().unwrap_or_default());
                cells.push(reference.clone());
            }

            seq.push(
                GridNode::new(cells)
                    .with_columns(TrackSizings(vec![Sizing::Auto; 2]))
                    .with_column_gutter(TrackSizings(vec![COLUMN_GUTTER.into()]))
                    .with_row_gutter(TrackSizings(vec![ROW_GUTTER.into()]))
                    .pack(),
            );
        } else {
            let mut entries = vec![];
            for (i, (_, reference)) in works.references.iter().enumerate() {
                if i > 0 {
                    entries.push(VNode::new(ROW_GUTTER.into()).with_weakness(1).pack());
                }
                entries.push(reference.clone());
            }

            seq.push(
                Content::sequence(entries)
                    .styled(ParNode::set_hanging_indent(INDENT.into())),
            );
        }

        Ok(Content::sequence(seq))
    }
}

impl LocalName for BibliographyNode {
    fn local_name(&self, lang: Lang) -> &'static str {
        match lang {
            Lang::GERMAN => "Bibliographie",
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
    AuthorDate,
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
            Self::Apa => CitationStyle::AuthorDate,
            Self::AuthorDate => CitationStyle::AuthorDate,
            Self::Ieee => CitationStyle::Numerical,
            Self::Mla => CitationStyle::AuthorDate,
        }
    }
}

/// A citation of another work.
///
/// Display: Citation
/// Category: meta
#[node(Locatable, Synthesize, Show)]
pub struct CiteNode {
    /// The citation key.
    #[required]
    pub key: EcoString,

    /// A supplement for the citation such as page or chapter number.
    #[positional]
    pub supplement: Option<Content>,

    /// The citation style.
    ///
    /// When set to `{auto}`, automatically picks the preferred citation style
    /// for the bibliography's style.
    pub style: Smart<CitationStyle>,
}

impl Synthesize for CiteNode {
    fn synthesize(&mut self, _: &Vt, styles: StyleChain) {
        self.push_supplement(self.supplement(styles));
        self.push_style(self.style(styles));
    }
}

impl Show for CiteNode {
    fn show(&self, vt: &mut Vt, _: StyleChain) -> SourceResult<Content> {
        let id = self.0.stable_id().unwrap();
        let works = match Works::new(vt) {
            Ok(works) => works,
            Err(error) => {
                if vt.locatable() {
                    bail!(self.span(), error)
                } else {
                    return Ok(TextNode::packed("citation"));
                }
            }
        };

        let Some(citation) = works.citations.get(&id).cloned() else {
            return Ok(TextNode::packed("citation"));
        };

        citation
            .ok_or("bibliography does not contain this key")
            .at(self.span())
    }
}

/// A citation style.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Cast)]
pub enum CitationStyle {
    /// IEEE-style numerical reference markers.
    Numerical,
    /// A simple alphanumerical style. For example, the output could be Rass97
    /// or MKG+21.
    Alphanumerical,
    /// The Chicago Author Date style. Based on the 17th edition of the Chicago
    /// Manual of Style, Chapter 15.
    AuthorDate,
    /// A Chicago-like author-title format. Results could look like this:
    /// Prokopov, “It Is Fast or It Is Wrong”.
    AuthorTitle,
    /// Citations that just consist of the entry keys.
    Keys,
}

/// Fully formatted citations and references.
pub struct Works {
    citations: HashMap<StableId, Option<Content>>,
    references: Vec<(Option<Content>, Content)>,
}

impl Works {
    /// Prepare all things need to cite a work or format a bibliography.
    pub fn new(vt: &Vt) -> StrResult<Arc<Self>> {
        let bibliography = BibliographyNode::find(vt.introspector)?;
        let style = bibliography.style(StyleChain::default());
        let citations = vt
            .locate_node::<CiteNode>()
            .map(|(id, node)| {
                (
                    id,
                    node.key(),
                    node.supplement(StyleChain::default()),
                    node.style(StyleChain::default())
                        .unwrap_or(style.default_citation_style()),
                )
            })
            .collect();
        Ok(create(vt.world(), &bibliography.path(), style, citations))
    }
}

/// Generate all citations and the whole bibliography.
#[comemo::memoize]
fn create(
    world: Tracked<dyn World>,
    path: &str,
    style: BibliographyStyle,
    citations: Vec<(StableId, EcoString, Option<Content>, CitationStyle)>,
) -> Arc<Works> {
    let entries = load(world, path).unwrap();

    let mut db = Database::new();
    let mut preliminary = vec![];

    for (id, key, supplement, style) in citations {
        let entry = entries.iter().find(|entry| entry.key() == key);
        if let Some(entry) = &entry {
            db.push(entry);
        }
        preliminary.push((id, entry, supplement, style));
    }

    let mut current = CitationStyle::Numerical;
    let mut citation_style: Box<dyn style::CitationStyle> =
        Box::new(style::Numerical::new());

    let citations = preliminary
        .into_iter()
        .map(|(id, result, supplement, style)| {
            let formatted = result.map(|entry| {
                if style != current {
                    current = style;
                    citation_style = match style {
                        CitationStyle::Numerical => Box::new(style::Numerical::new()),
                        CitationStyle::Alphanumerical => {
                            Box::new(style::Alphanumerical::new())
                        }
                        CitationStyle::AuthorDate => {
                            Box::new(style::ChicagoAuthorDate::new())
                        }
                        CitationStyle::AuthorTitle => Box::new(style::AuthorTitle::new()),
                        CitationStyle::Keys => Box::new(style::Keys::new()),
                    };
                }

                let citation = db.citation(
                    &mut *citation_style,
                    &[Citation {
                        entry,
                        supplement: supplement.is_some().then(|| SUPPLEMENT),
                    }],
                );
                let bracketed = citation.display.with_default_brackets(&*citation_style);
                format_display_string(&bracketed, supplement)
            });
            (id, formatted)
        })
        .collect();

    let bibliography_style: Box<dyn style::BibliographyStyle> = match style {
        BibliographyStyle::Apa => Box::new(style::Apa::new()),
        BibliographyStyle::AuthorDate => Box::new(style::ChicagoAuthorDate::new()),
        BibliographyStyle::Ieee => Box::new(style::Ieee::new()),
        BibliographyStyle::Mla => Box::new(style::Mla::new()),
    };

    let references = db
        .bibliography(&*bibliography_style, None)
        .into_iter()
        .map(|reference| {
            let prefix = reference.prefix.map(|prefix| {
                let bracketed = prefix.with_default_brackets(&*citation_style);
                format_display_string(&bracketed, None)
            });
            let reference = format_display_string(&reference.display, None);
            (prefix, reference)
        })
        .collect();

    Arc::new(Works { citations, references })
}

/// Load bibliography entries from a path.
#[comemo::memoize]
fn load(world: Tracked<dyn World>, path: &str) -> StrResult<EcoVec<hayagriva::Entry>> {
    let path = Path::new(path);
    let buffer = world.file(path)?;
    let src = std::str::from_utf8(&buffer).map_err(|_| "file is not valid utf-8")?;
    let ext = path.extension().and_then(OsStr::to_str).unwrap_or_default();
    let entries = match ext.to_lowercase().as_str() {
        "yml" => hayagriva::io::from_yaml_str(src).map_err(format_hayagriva_error)?,
        "bib" => hayagriva::io::from_biblatex_str(src).map_err(|err| {
            err.into_iter()
                .next()
                .map(|error| format_biblatex_error(src, error))
                .unwrap_or_else(|| "failed to parse biblatex file".into())
        })?,
        _ => return Err("unknown bibliography format".into()),
    };
    Ok(entries.into_iter().collect())
}

/// Format a Hayagriva loading error.
fn format_hayagriva_error(error: YamlBibliographyError) -> EcoString {
    eco_format!("{error}")
}

/// Format a BibLaTeX loading error.
fn format_biblatex_error(src: &str, error: BibLaTeXError) -> EcoString {
    let (span, msg) = match error {
        BibLaTeXError::Parse(error) => (error.span, error.kind.to_string()),
        BibLaTeXError::Type(error) => (error.span, error.kind.to_string()),
    };
    let line = src.get(..span.start).unwrap_or_default().lines().count();
    eco_format!("failed to parse biblatex file: {msg} in line {line}")
}

/// Hayagriva only supports strings, but we have a content supplement. To deal
/// with this, we pass this string to hayagriva instead of our content, find it
/// in the output and replace it with the content.
const SUPPLEMENT: &str = "cdc579c45cf3d648905c142c7082683f";

/// Format a display string into content.
fn format_display_string(
    string: &DisplayString,
    mut supplement: Option<Content>,
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

        let mut styles = StyleMap::new();
        for (range, fmt) in &string.formatting {
            if !range.contains(&start) {
                continue;
            }

            styles.set(match fmt {
                Formatting::Bold => TextNode::set_weight(FontWeight::BOLD),
                Formatting::Italic => TextNode::set_style(FontStyle::Italic),
                Formatting::NoHyphenation => {
                    TextNode::set_hyphenate(Hyphenate(Smart::Custom(false)))
                }
            });
        }

        let content = if segment == SUPPLEMENT && supplement.is_some() {
            supplement.take().unwrap_or_default()
        } else {
            TextNode::packed(segment)
        };

        seq.push(content.styled_with_map(styles));
        start = stop;
    }

    Content::sequence(seq)
}
