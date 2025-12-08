use std::ops::Deref;
use std::str::FromStr;

use ecow::{EcoString, eco_format};
use typst_syntax::Span;

use crate::diag::{At, SourceResult, StrResult, bail};
use crate::engine::Engine;
use crate::foundations::{
    Args, Construct, Content, Label, Packed, Repr, Selector, ShowSet, Smart, StyleChain,
    Styles, cast, elem,
};
use crate::introspection::{
    Counter, CounterKey, Introspector, Locatable, Location, QueryFirstIntrospection,
    QueryLabelIntrospection, Tagged,
};
use crate::layout::{PageElem, Position};
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
/// In HTML export, a link to a [label] or [location] will be turned into a
/// fragment link to a named anchor point. To support this, targets without an
/// existing ID will automatically receive an ID in the DOM. How this works
/// varies by which kind of HTML node(s) the link target turned into:
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
    pub fn resolve(&self, engine: &mut Engine, span: Span) -> SourceResult<Destination> {
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
    pub fn resolve_with_introspector(
        &self,
        introspector: &Introspector,
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
    Position(Position),
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
    v: Position => Self::Position(v),
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
