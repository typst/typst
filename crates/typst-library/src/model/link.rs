use std::ops::Deref;
use std::str::FromStr;

use comemo::Tracked;
use ecow::{EcoString, eco_format};
use rustc_hash::FxHashMap;
use typst_syntax::{Span, VirtualPath};
use typst_utils::PicoStr;

use crate::diag::{At, SourceDiagnostic, SourceResult, StrResult, bail, warning};
use crate::engine::Engine;
use crate::foundations::{
    Args, Construct, Content, Label, NativeElement, Packed, Repr, Selector, ShowSet,
    Smart, StyleChain, Styles, cast, elem,
};
use crate::introspection::{
    Counter, CounterKey, History, Introspect, Introspector, Locatable, Location,
    PagedPosition, PathIntrospection, QueryFirstIntrospection, QueryLabelIntrospection,
    Tagged,
};
use crate::layout::PageElem;
use crate::model::{NumberingPattern, Refable};
use crate::text::{LocalName, TextElem};

/// Links to a URL or a location in the document.
///
/// By default, links do not look any different from normal text. However,
/// you can easily apply a style of your choice with a show rule.
///
/// # Example
/// ```example
/// #show link: underline
///
/// https://example.com \
///
/// #link("https://example.com") \
/// #link("https://example.com")[
///   See example.com
/// ]
/// ```
///
/// # Syntax
/// This function also has dedicated syntax: Text that starts with `http://` or
/// `https://` is automatically turned into a link.
///
/// # Hyphenation
/// If you enable hyphenation or justification, by default, it will not apply to
/// links to prevent unwanted hyphenation in URLs. You can opt out of this
/// default via `{show link: set text(hyphenate: true)}`.
///
/// # Accessibility
/// The destination of a link should be clear from the link text itself, or at
/// least from the text immediately surrounding it. In PDF export, Typst will
/// automatically generate a tooltip description for links based on their
/// destination. For links to URLs, the URL itself will be used as the tooltip.
///
/// # Links in HTML export
/// In [HTML export]($html), a link to a [label] or [location] will be turned
/// into a fragment link to a named anchor point. To support this, targets
/// without an existing ID will automatically receive an ID in the DOM. How this
/// works varies by which kind of HTML node(s) the link target turned into:
///
/// - If the link target turned into a single HTML element, that element will
///   receive the ID. This is, for instance, typically the case when linking to
///   a top-level heading (which turns into a single `<h2>` element).
///
/// - If the link target turned into a single text node, the node will be
///   wrapped in a `<span>`, which will then receive the ID.
///
/// - If the link target turned into multiple nodes, the first node will receive
///   the ID.
///
/// - If the link target turned into no nodes at all, an empty span will be
///   generated to serve as a link target.
///
/// If you rely on a specific DOM structure, you should ensure that the link
/// target turns into one or multiple elements, as the compiler makes no
/// guarantees on the precise segmentation of text into text nodes.
///
/// If present, the automatic ID generation tries to reuse the link target's
/// label to create a human-readable ID. A label can be reused if:
///
/// - All characters are alphabetic or numeric according to Unicode, or a
///   hyphen, or an underscore.
///
/// - The label does not start with a digit or hyphen.
///
/// These rules ensure that the label is both a valid CSS identifier and a valid
/// URL fragment for linking.
///
/// As IDs must be unique in the DOM, duplicate labels might need disambiguation
/// when reusing them as IDs. The precise rules for this are as follows:
///
/// - If a label can be reused and is unique in the document, it will directly
///   be used as the ID.
///
/// - If it's reusable, but not unique, a suffix consisting of a hyphen and an
///   integer will be added. For instance, if the label `<mylabel>` exists
///   twice, it would turn into `mylabel-1` and `mylabel-2`.
///
/// - Otherwise, a unique ID of the form `loc-` followed by an integer will be
///   generated.
///
/// # Links in bundle export
/// In [bundle export]($bundle), linking still works as usual. For instance, if
/// you attach a label to an element in one document, links in other documents
/// can reference that label. In addition, documents and assets are also
/// directly linkable. To link to a full document or asset, you can attach a
/// label to it or [query] for it and extract its [location].
///
/// ```typ
/// #document("index.html")[
///   // Link to document.
///   #link(<appendix>)[To appendix]
///
///   // Link into document.
///   See the #link(<glossary>)[Glossary]
///   for more information.
/// ]
///
/// #document("appendix.html")[
///   = Definitions
///   ...
///
///   = Glossary <glossary>
///   ...
/// ] <appendix>
/// ```
///
/// Cross-document links are emitted as relative paths (potentially with
/// fragments). Typst automatically assigns anchor names per document based on
/// the same rules as in HTML export. In HTML and SVG documents, these are
/// emitted as `id` attributes on elements. In PDF documents, they are emitted
/// as _named destinations._ PNG documents do not support linking.
///
/// Note that links always use full relative paths. In some scenarios (primarily
/// for multi-page web sites), this may not be desirable. For instance, you may
/// want to generate a `/blog/index.html` document while wanting to link to it
/// as just `/blog`. Furthermore, your web server might treat `/blog` and
/// `/blog/` as interchangeable and serve `/blog/index.html` for both. If a user
/// then navigates to `/blog`, relative links to other pages generated by Typst
/// will no longer work. Currently, Typst does not have a way to directly hook
/// into the built-in link handling. That said, in HTML export, depending on
/// your use case, it may be possible to adjust the built-in link handling with
/// a show rule on `{html.elem.where(tag: "a")}`.
#[elem(Locatable)]
pub struct LinkElem {
    /// The destination the link points to.
    ///
    /// - To link to web pages, `dest` should be a valid URL string. If the URL
    ///   is in the `mailto:` or `tel:` scheme and the `body` parameter is
    ///   omitted, the email address or phone number will be the link's body,
    ///   without the scheme.
    ///
    /// - To link to another part of the document, `dest` can take one of three
    ///   forms:
    ///   - A [label] attached to an element. If you also want automatic text
    ///     for the link based on the element, consider using a
    ///     [reference]($ref) instead.
    ///
    ///   - A [`location`] (typically retrieved from [`here`], [`locate`] or
    ///     [`query`]).
    ///
    ///   - A dictionary with a `page` key of type [integer]($int) and `x` and
    ///     `y` coordinates of type [length]. Pages are counted from one, and
    ///     the coordinates are relative to the page's top left corner.
    ///
    /// ```example
    /// = Introduction <intro>
    /// #link("mailto:hello@typst.app") \
    /// #link(<intro>)[Go to intro] \
    /// #link((page: 1, x: 0pt, y: 0pt))[
    ///   Go to top
    /// ]
    /// ```
    #[required]
    #[parse(
        let dest = args.expect::<LinkTarget>("destination")?;
        dest.clone()
    )]
    pub dest: LinkTarget,

    /// The content that should become a link.
    ///
    /// If `dest` is an URL string, the parameter can be omitted. In this case,
    /// the URL will be shown as the link.
    #[required]
    #[parse(match &dest {
        LinkTarget::Dest(Destination::Url(url)) => match args.eat()? {
            Some(body) => body,
            None => body_from_url(url),
        },
        _ => args.expect("body")?,
    })]
    pub body: Content,

    /// A destination style that should be applied to elements.
    #[internal]
    #[ghost]
    pub current: Option<Destination>,
}

impl LinkElem {
    /// Create a link element from a URL with its bare text.
    pub fn from_url(url: Url) -> Self {
        let body = body_from_url(&url);
        Self::new(LinkTarget::Dest(Destination::Url(url)), body)
    }

    /// Finds all linked-to locations referenced in an introspector.
    pub fn find_destinations(
        introspector: &dyn Introspector,
    ) -> impl Iterator<Item = Location> {
        introspector
            .query(&Self::ELEM.select())
            .into_iter()
            .map(|elem| elem.into_packed::<Self>().unwrap())
            .filter_map(|elem| match elem.dest.resolve_late(introspector) {
                Ok(Destination::Location(loc)) => Some(loc),
                _ => None,
            })
    }
}

impl ShowSet for Packed<LinkElem> {
    fn show_set(&self, _: StyleChain) -> Styles {
        let mut out = Styles::new();
        out.set(TextElem::hyphenate, Smart::Custom(false));
        out
    }
}

pub(crate) fn body_from_url(url: &Url) -> Content {
    let stripped = url.strip_contact_scheme().map(|(_, s)| s.into());
    TextElem::packed(stripped.unwrap_or_else(|| url.clone().into_inner()))
}

/// A target where a link can go.
#[derive(Debug, Clone, PartialEq, Hash)]
pub enum LinkTarget {
    Dest(Destination),
    Label(Label),
}

impl LinkTarget {
    /// Resolves the destination.
    pub fn resolve_early(
        &self,
        engine: &mut Engine,
        span: Span,
    ) -> SourceResult<Destination> {
        Ok(match self {
            LinkTarget::Dest(dest) => dest.clone(),
            LinkTarget::Label(label) => {
                let elem =
                    engine.introspect(QueryLabelIntrospection(*label, span)).at(span)?;
                Destination::Location(elem.location().unwrap())
            }
        })
    }

    /// Resolves the destination without an engine.
    pub fn resolve_late(
        &self,
        introspector: &dyn Introspector,
    ) -> StrResult<Destination> {
        Ok(match self {
            LinkTarget::Dest(dest) => dest.clone(),
            LinkTarget::Label(label) => {
                let elem = introspector.query_label(*label)?;
                Destination::Location(elem.location().unwrap())
            }
        })
    }
}

cast! {
    LinkTarget,
    self => match self {
        Self::Dest(v) => v.into_value(),
        Self::Label(v) => v.into_value(),
    },
    v: Destination => Self::Dest(v),
    v: Label => Self::Label(v),
}

impl From<Destination> for LinkTarget {
    fn from(dest: Destination) -> Self {
        Self::Dest(dest)
    }
}

/// A link destination.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum Destination {
    /// A link to a URL.
    Url(Url),
    /// A link to a point on a page.
    Position(PagedPosition),
    /// An unresolved link to a location in the document.
    Location(Location),
}

impl Destination {
    pub fn alt_text(
        &self,
        engine: &mut Engine,
        styles: StyleChain,
        span: Span,
    ) -> SourceResult<EcoString> {
        match self {
            Destination::Url(url) => {
                let contact = url.strip_contact_scheme().map(|(scheme, stripped)| {
                    eco_format!("{} {stripped}", scheme.local_name_in(styles))
                });
                Ok(contact.unwrap_or_else(|| url.clone().into_inner()))
            }
            Destination::Position(pos) => {
                let page_nr = eco_format!("{}", pos.page.get());
                let page_str = PageElem::local_name_in(styles);
                Ok(eco_format!("{page_str} {page_nr}"))
            }
            &Destination::Location(loc) => {
                let fallback = |engine: &mut Engine| {
                    // Fall back to a generating a page reference.
                    let numbering =
                        loc.page_numbering(engine, span).unwrap_or_else(|| {
                            NumberingPattern::from_str("1").unwrap().into()
                        });
                    let page_nr = Counter::new(CounterKey::Page)
                        .display_at(engine, loc, styles, &numbering, span)?
                        .plain_text();
                    let page_str = PageElem::local_name_in(styles);
                    Ok(eco_format!("{page_str} {page_nr}"))
                };

                // Try to generate more meaningful alt text if the location is a
                // refable element.
                if let Some(elem) = engine
                    .introspect(QueryFirstIntrospection(Selector::Location(loc), span))
                    && let Some(refable) = elem.with::<dyn Refable>()
                {
                    let counter = refable.counter();
                    let supplement = refable.supplement().plain_text();

                    if let Some(numbering) = refable.numbering() {
                        let numbers = counter.display_at(
                            engine,
                            loc,
                            styles,
                            &numbering.clone().trimmed(),
                            span,
                        )?;
                        return Ok(eco_format!("{supplement} {}", numbers.plain_text()));
                    } else {
                        let page_ref = fallback(engine)?;
                        return Ok(eco_format!("{supplement}, {page_ref}"));
                    }
                }

                fallback(engine)
            }
        }
    }
}

impl Repr for Destination {
    fn repr(&self) -> EcoString {
        eco_format!("{self:?}")
    }
}

cast! {
    Destination,
    self => match self {
        Self::Url(v) => v.into_value(),
        Self::Position(v) => v.into_value(),
        Self::Location(v) => v.into_value(),
    },
    v: Url => Self::Url(v),
    v: PagedPosition => Self::Position(v),
    v: Location => Self::Location(v),
}

/// A uniform resource locator with a maximum length.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Url(EcoString);

impl Url {
    /// Create a URL from a string, checking the maximum length.
    pub fn new(url: impl Into<EcoString>) -> StrResult<Self> {
        let url = url.into();
        if url.len() > 8000 {
            bail!("URL is too long")
        } else if url.is_empty() {
            bail!("URL must not be empty")
        }
        Ok(Self(url))
    }

    /// Extract the underlying [`EcoString`].
    pub fn into_inner(self) -> EcoString {
        self.0
    }

    pub fn strip_contact_scheme(&self) -> Option<(UrlContactScheme, &str)> {
        [UrlContactScheme::Mailto, UrlContactScheme::Tel]
            .into_iter()
            .find_map(|scheme| {
                let stripped = self.strip_prefix(scheme.as_str())?;
                Some((scheme, stripped))
            })
    }
}

impl Deref for Url {
    type Target = EcoString;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

cast! {
    Url,
    self => self.0.into_value(),
    v: EcoString => Self::new(v)?,
}

/// This is a temporary hack to dispatch to
/// - a raw link that does not go through `LinkElem` in paged
/// - `LinkElem` in HTML (there is no equivalent to a direct link)
///
/// We'll want to dispatch all kinds of links to `LinkElem` in the future, but
/// this is a visually breaking change in paged export as e.g.
/// `show link: underline` will suddenly also affect references, bibliography
/// back references, footnote references, etc. We'll want to do this change
/// carefully and in a way where we provide a good way to keep styling only URL
/// links, which is a bit too complicated to achieve right now for such a basic
/// requirement.
#[elem(Construct)]
pub struct DirectLinkElem {
    #[required]
    #[internal]
    pub loc: Location,
    #[required]
    #[internal]
    pub body: Content,
    #[required]
    #[internal]
    pub alt: Option<EcoString>,
}

impl Construct for DirectLinkElem {
    fn construct(_: &mut Engine, args: &mut Args) -> SourceResult<Content> {
        bail!(args.span, "cannot be constructed manually");
    }
}

/// An element that wraps all content that is [`Content::linked`] to a
/// destination.
#[elem(Tagged, Construct)]
pub struct LinkMarker {
    /// The content.
    #[internal]
    #[required]
    pub body: Content,
    #[internal]
    #[required]
    pub alt: Option<EcoString>,
}

impl Construct for LinkMarker {
    fn construct(_: &mut Engine, args: &mut Args) -> SourceResult<Content> {
        bail!(args.span, "cannot be constructed manually");
    }
}

#[derive(Copy, Clone)]
pub enum UrlContactScheme {
    /// The `mailto:` prefix.
    Mailto,
    /// The `tel:` prefix.
    Tel,
}

impl UrlContactScheme {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Mailto => "mailto:",
            Self::Tel => "tel:",
        }
    }

    pub fn local_name_in(self, styles: StyleChain) -> &'static str {
        match self {
            UrlContactScheme::Mailto => Email::local_name_in(styles),
            UrlContactScheme::Tel => Telephone::local_name_in(styles),
        }
    }
}

#[derive(Copy, Clone)]
pub struct Email;
impl LocalName for Email {
    const KEY: &'static str = "email";
}

#[derive(Copy, Clone)]
pub struct Telephone;
impl LocalName for Telephone {
    const KEY: &'static str = "telephone";
}

/// Creates unique IDs for elements.
pub struct AnchorGenerator<'a> {
    introspector: &'a dyn Introspector,
    loc_counter: usize,
    label_counter: FxHashMap<Label, usize>,
}

impl<'a> AnchorGenerator<'a> {
    /// Creates a new identificator.
    pub fn new(introspector: &'a dyn Introspector) -> Self {
        Self {
            introspector,
            loc_counter: 0,
            label_counter: FxHashMap::default(),
        }
    }

    /// Returns a reference to the underlying introspector.
    pub fn introspector(&self) -> &'a dyn Introspector {
        self.introspector
    }

    /// Generates an ID, potentially based on a label.
    pub fn identify(&mut self, label: Option<Label>) -> EcoString {
        if let Some(label) = label {
            let resolved = label.resolve();
            let text = resolved.as_str();
            if can_use_label_as_id(text) {
                if self.introspector.label_count(label) == 1 {
                    return text.into();
                }

                let counter = self.label_counter.entry(label).or_insert(0);
                *counter += 1;
                return disambiguate(self.introspector, text, counter);
            }
        }

        self.loc_counter += 1;
        disambiguate(self.introspector, "loc", &mut self.loc_counter)
    }
}

/// Whether the label is both a valid CSS identifier and a valid URL fragment
/// for linking.
///
/// This is slightly more restrictive than HTML and CSS, but easier to
/// understand and explain.
fn can_use_label_as_id(label: &str) -> bool {
    !label.is_empty()
        && label.chars().all(|c| c.is_alphanumeric() || matches!(c, '-' | '_'))
        && !label.starts_with(|c: char| c.is_numeric() || c == '-')
}

/// Disambiguates `text` with the suffix `-{counter}`, while ensuring that this
/// does not result in a collision with an existing label.
fn disambiguate(
    introspector: &dyn Introspector,
    text: &str,
    counter: &mut usize,
) -> EcoString {
    loop {
        let disambiguated = eco_format!("{text}-{counter}");
        if PicoStr::get(&disambiguated)
            .and_then(Label::new)
            .is_some_and(|label| introspector.label_count(label) > 0)
        {
            *counter += 1;
        } else {
            break disambiguated;
        }
    }
}

/// Resolves location links during compilation.
///
/// This is used in HTML export as there isn't a dedicated export stage that
/// could make use of the [`LateLinkResolver`]. There is the HTML serialization,
/// but that's not an appropriate stage for meaningful DOM manipulation.
pub struct EarlyLinkResolver {
    base: Location,
    span: Span,
}

impl EarlyLinkResolver {
    /// Creates a resolver that resolves links relatively to the element with
    /// the given location.
    pub fn new(base: Location, span: Span) -> Self {
        Self { base, span }
    }

    /// Resolves a link to the given location.
    pub fn resolve(
        &self,
        engine: &mut Engine,
        location: Location,
    ) -> StrResult<ResolvedLink> {
        let from = engine.introspect(PathIntrospection(self.base, self.span));
        let to = engine.introspect(PathIntrospection(location, self.span));
        let anchor = engine
            .introspect(LinkAnchorIntrospection(location, self.span))
            .ok_or("failed to determine link anchor")?;

        Ok(match (&from, &to) {
            // This is the normal case in single file export.
            (None, None) => ResolvedLink::Local { anchor },
            // This is the normal case in bundle export.
            (Some(from), Some(to)) => {
                if from == to {
                    ResolvedLink::Local { anchor }
                } else if let Some(parent) = from.parent() {
                    let relative_path = to.relative_from(&parent);
                    ResolvedLink::Cross { relative_path, anchor }
                } else {
                    // For this to happen, `src` would have to be `/`, which
                    // is not allowed.
                    bail!("containing document has invalid path")
                }
            }
            // This can, for instance, happen when trying to link to
            // metadata that is not within a document (top-level in the
            // bundle).
            (Some(_), None) => {
                bail!("link destination is not within a document")
            }
            // This is rather unlikely because we can't resolve a link rule
            // in a non-file. It could happen in a non-convergent case.
            (None, Some(_)) => bail!("failed to resolve cross-link"),
        })
    }
}

/// Resolves location links during export.
///
/// This is used in paged exports. Compared to the [`EarlyLinkResolver`], this
/// one can save an introspection iteration as links don't need to be fully
/// resolved during compilation. Keeping the location link unresolved will also
/// be useful for tagging links in PDF 2.0 (linking to an element and not just a
/// position).
///
/// The downside is that links could be silently broken in a non-converging
/// scenario where HTML would instead generate an error, so it's a bit of a
/// trade-off and not entirely clear whether this is the best way to do it.
pub struct LateLinkResolver<'a> {
    base: Option<&'a VirtualPath>,
    introspector: &'a dyn Introspector,
}

impl<'a> LateLinkResolver<'a> {
    /// Creates a resolver.
    ///
    /// - In single-document export, `base` should be `None`.
    /// - In bundle export, `base` should be the path of the document relative
    ///   to which links shall be resolved.
    pub fn new(
        base: Option<&'a VirtualPath>,
        introspector: &'a dyn Introspector,
    ) -> Self {
        Self { base, introspector }
    }
}

/// Resolves a link to the given location.
#[comemo::track]
impl<'a> LateLinkResolver<'a> {
    pub fn resolve(&self, location: Location) -> Option<ResolvedLink> {
        let from = self.base;
        let to = self.introspector.path(location);
        let anchor = self.introspector.anchor(location)?.clone();

        // See `EarlyLinkResolver::resolve` for more details.
        Some(match (from, to) {
            (None, None) => ResolvedLink::Local { anchor },
            (Some(from), Some(to)) => {
                if from == to {
                    ResolvedLink::Local { anchor }
                } else if let Some(parent) = from.parent() {
                    let relative_path = to.relative_from(&parent);
                    ResolvedLink::Cross { relative_path, anchor }
                } else {
                    return None;
                }
            }
            (Some(_), None) => return None,
            (None, Some(_)) => return None,
        })
    }
}

/// A resolved internal link.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum ResolvedLink {
    /// Should link to an anchor in the same document.
    Local {
        /// The anchor to link to. If empty, should link to the full current
        /// document.
        anchor: EcoString,
    },
    /// Should link to an anchor in another document.
    Cross {
        /// The relative path that navigates from the document containing the link
        /// to the linked-to document containing the anchor.
        relative_path: EcoString,
        /// The anchor to link to. If empty, should link to the full document.
        anchor: EcoString,
    },
}

impl ResolvedLink {
    /// Turns the link into a URI, potentially with an `#` anchor fragment.
    pub fn into_uri(self) -> EcoString {
        match self {
            // Still write the empty anchor if linking to the document itself
            // because `#` doesn't trigger a reload unlike an empty href.
            Self::Local { anchor } => eco_format!("#{anchor}"),
            Self::Cross { relative_path, anchor } => {
                if anchor.is_empty() {
                    // Don't write a trailing `#` if linking to a full document.
                    relative_path
                } else {
                    eco_format!("{relative_path}#{anchor}")
                }
            }
        }
    }
}

/// Resolves the anchor to reach the linked-to element with the given location.
#[derive(Debug, Clone, PartialEq, Hash)]
struct LinkAnchorIntrospection(Location, Span);

impl Introspect for LinkAnchorIntrospection {
    type Output = Option<EcoString>;

    fn introspect(
        &self,
        _: &mut Engine,
        introspector: Tracked<dyn Introspector + '_>,
    ) -> Self::Output {
        introspector.anchor(self.0).cloned()
    }

    fn diagnose(&self, history: &History<Self::Output>) -> SourceDiagnostic {
        let introspector = history.final_introspector();
        let what = match introspector.query_first(&Selector::Location(self.0)) {
            Some(content) => content.elem().name(),
            None => "element",
        };
        warning!(
            self.1,
            "link anchor assigned to the destination {what} did not stabilize",
        )
        .with_hint(history.hint("anchors", |id| match id {
            Some(id) => id.clone(),
            None => "(no anchor)".into(),
        }))
    }
}
