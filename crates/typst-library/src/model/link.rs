use std::ops::Deref;

use comemo::Tracked;
use ecow::{eco_format, EcoString};

use crate::diag::{bail, StrResult};
use crate::foundations::{
    cast, elem, Content, Label, Packed, Repr, ShowSet, Smart, StyleChain, Styles,
};
use crate::introspection::{Introspector, Locatable, Location};
use crate::layout::Position;
use crate::text::TextElem;

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

fn body_from_url(url: &Url) -> Content {
    let text = ["mailto:", "tel:"]
        .into_iter()
        .find_map(|prefix| url.strip_prefix(prefix))
        .unwrap_or(url);
    let shorter = text.len() < url.len();
    TextElem::packed(if shorter { text.into() } else { (**url).clone() })
}

/// A target where a link can go.
#[derive(Debug, Clone, PartialEq, Hash)]
pub enum LinkTarget {
    Dest(Destination),
    Label(Label),
}

impl LinkTarget {
    /// Resolves the destination.
    pub fn resolve(&self, introspector: Tracked<Introspector>) -> StrResult<Destination> {
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

impl Destination {}

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
        }
        Ok(Self(url))
    }

    /// Extract the underlying [`EcoString`].
    pub fn into_inner(self) -> EcoString {
        self.0
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
