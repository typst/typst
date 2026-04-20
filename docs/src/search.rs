use std::collections::BTreeMap;

use ecow::{EcoString, eco_format};
use serde::Serialize;
use typst::diag::{At, Hint, HintedStrResult, SourceResult, bail};
use typst::foundations::{Dict, Label, Value, cast};
use typst::introspection::{Introspector, Location, MetadataElem, Tag};
use typst_bundle::{Bundle, BundleDocument, BundleFile};
use typst_html::{HtmlElement, HtmlNode};
use typst_utils::PicoStr;

/// A search index that is encoded as `search.json`, emitted as an asset, and
/// then fetched by the frontend to power the site-global search in the top
/// left.
#[derive(Debug, Serialize)]
pub struct SearchIndex {
    /// Items that can be potential search results.
    pub items: Vec<IndexItem>,
    /// A sorted list of words that occur in the docs. Can be binary searched
    /// to find prefix matches.
    pub words: Vec<String>,
    /// Parallel to `words`. Stores the indices of the `items` which contain
    /// the parallel word.
    pub hits: Vec<Vec<usize>>,
}

/// One item in the search index.
///
/// Corresponds to one possible match during search. Here are two examples:
/// ```json
/// {
///   "kind": "Function",
///   "title": "Heading",
///   "route": "/reference/model/heading/"
/// },
/// {
///   "kind": "Parameter of caption",
///   "title": "Separator",
///   "route": "/reference/model/figure/#definitions-caption-separator"
/// },
/// ```
#[derive(Debug, Clone, Serialize)]
pub struct IndexItem {
    /// The category of item. Shown next to the match.
    pub kind: EcoString,
    /// The title-case name of the item. Shown as the match.
    pub title: EcoString,
    /// The full route to the matching page. May include a fragment.
    pub route: EcoString,
    /// Keywords with which the page can be found. The keywords are stored in
    /// the item to aid with ranking.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub keywords: Vec<EcoString>,
}

cast! {
    IndexItem,
    mut v: Dict => Self {
        kind: v.take("kind")?.cast()?,
        title: v.take("title")?.cast()?,
        route: v.take("route")?.cast()?,
        keywords: v.take("keywords")?.cast()?,
    }
}

/// Walks through the bundle and indexes all text and items in it. Items
/// are `metadata` elements labelled `<metadata-index-item>`. Text in between
/// items is considered belonging to the preceding item.
pub fn build_search_index(bundle: &Bundle) -> SourceResult<SearchIndex> {
    let mut indexer = Indexer::new();
    for file in bundle.files.values() {
        let BundleFile::Document(BundleDocument::Html(doc)) = file else { continue };
        walk_html(&mut indexer, doc.root(), bundle.introspector.as_ref())?;
    }
    Ok(indexer.finish())
}

/// Indexes the contents of a single HTML element, recursively.
fn walk_html(
    indexer: &mut Indexer,
    elem: &HtmlElement,
    introspector: &dyn Introspector,
) -> SourceResult<()> {
    for node in &elem.children {
        match node {
            HtmlNode::Tag(tag) => {
                if let Tag::Start(it, _) = tag
                    && it.label() == Some(indexer.item_label)
                    && let Some(metadata) = it.to_packed::<MetadataElem>()
                {
                    let item = parse_item_metadata(metadata.value.clone(), introspector)
                        .hint("search index metadata is not well-formed")
                        .at(metadata.span())?;
                    indexer.enter(item.clone());
                    indexer.index_str(&item.title);
                    for keyword in &item.keywords {
                        indexer.index_str(keyword);
                    }
                }
            }
            HtmlNode::Text(text, _) => indexer.index_str(text),
            HtmlNode::Element(elem) => walk_html(indexer, elem, introspector)?,
            HtmlNode::Frame(_) => {}
        }
    }
    Ok(())
}

/// Takes a search index metadata element's dynamic value and turns it into
/// strongly typed index item.
fn parse_item_metadata(
    value: Value,
    introspector: &dyn Introspector,
) -> HintedStrResult<IndexItem> {
    let mut dict = value.cast::<Dict>()?;
    let kind = dict.take("kind")?.cast()?;
    let title = dict.take("title")?.cast()?;
    let dest = dict.take("dest")?.cast::<ItemDestination>()?;
    let keywords = dict.take("keywords")?.cast()?;
    let route = match dest {
        ItemDestination::Route(route) => {
            if !route.ends_with('/') {
                bail!("routes should end with a slash: {route:?}");
            }
            route
        }
        ItemDestination::Location(loc) => {
            let path = introspector.path(loc).ok_or("found no path for index item")?;
            let path = path
                .get_with_slash()
                .strip_suffix("index.html")
                .ok_or("expected consistent usage of index.html")?;
            let anchor = introspector
                .anchor(loc)
                .ok_or("found no HTML anchor for index item")?;
            if anchor.is_empty() { path.into() } else { eco_format!("{path}#{anchor}") }
        }
    };
    Ok(IndexItem { kind, title, route, keywords })
}

/// The two kinds of destinations that can be stored in an index item's
/// `dest` field.
enum ItemDestination {
    /// A raw route to an HTML file. Must end with a slash.
    Route(EcoString),
    /// A location that should be resolved to a route.
    Location(Location),
}

cast! {
   ItemDestination,
   v: EcoString => Self::Route(v),
   v: Location => Self::Location(v),
}

/// Associates text with index items.
struct Indexer {
    words: BTreeMap<String, Vec<usize>>,
    items: Vec<IndexItem>,
    item_label: Label,
}

impl Indexer {
    fn new() -> Self {
        Self {
            words: BTreeMap::new(),
            items: vec![],
            item_label: Label::new(PicoStr::intern("metadata-index-item")).unwrap(),
        }
    }

    fn finish(self) -> SearchIndex {
        let mut words = vec![];
        let mut hits = vec![];
        for (word, word_hits) in self.words {
            words.push(word);
            hits.push(word_hits);
        }
        SearchIndex { items: self.items, words, hits }
    }

    fn enter(&mut self, item: IndexItem) {
        self.items.push(item);
    }

    fn index_str(&mut self, s: &str) {
        if self.items.is_empty() {
            return;
        }
        let n = self.items.len() - 1;
        for word in s
            .to_lowercase()
            .split(|c: char| c.is_ascii_whitespace() || c.is_ascii_punctuation())
            .filter(|w| !w.is_empty())
            .filter(|w| STOP_WORDS.binary_search(w).is_err())
        {
            let vec = self.words.entry(word.into()).or_default();
            if !vec.contains(&n) {
                vec.push(n);
            }
        }
    }
}

/// These words are ignored. Taken from
/// <https://gist.github.com/sebleier/554280>.
#[rustfmt::skip]
const STOP_WORDS: &[&str] = &[
    "a", "about", "am", "an", "and", "are", "as", "at", "be", "because", "been",
    "being", "but", "by", "can", "did", "do", "does", "doing", "don", "few",
    "from", "had", "has", "have", "he", "her", "hers", "herself", "him",
    "himself", "his", "i", "into", "is", "it", "its", "itself", "just", "me",
    "more", "most", "my", "myself", "no", "nor", "not", "now", "or", "our",
    "ours", "ourselves", "out", "s", "she", "should", "so", "some", "such", "t",
    "than", "that", "the", "their", "theirs", "them", "themselves", "then",
    "there", "these", "they", "this", "those", "to", "too", "very", "was", "we",
    "were", "what", "who", "whom", "why", "will", "you", "your", "yours",
    "yourself", "yourselves",
];
