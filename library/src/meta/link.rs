use crate::prelude::*;
use crate::text::{Hyphenate, TextElem};

/// Link to a URL or a location in the document.
///
/// The link function makes its positional `body` argument clickable and links
/// it to the destination specified by the `dest` argument. By default, links
/// are not styled any different from normal text. However, you can easily apply
/// a style of your choice with a show rule.
///
/// ## Example
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
/// ## Syntax
/// This function also has dedicated syntax: Text that starts with `http://` or
/// `https://` is automatically turned into a link.
///
/// Display: Link
/// Category: meta
#[element(Show)]
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
    ///   - A [label]($func/label) attached to an element. If you also want
    ///     automatic text for the link based on the element, consider using
    ///     a [reference]($func/ref) instead.
    ///
    ///   - A [location]($func/locate) resulting from a [`locate`]($func/locate)
    ///     call or [`query`]($func/query).
    ///
    ///   - A dictionary with a `page` key of type [integer]($type/integer) and
    ///     `x` and `y` coordinates of type [length]($type/length). Pages are
    ///     counted from one, and the coordinates are relative to the page's top
    ///     left corner.
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
}

impl LinkElem {
    /// Create a link element from a URL with its bare text.
    pub fn from_url(url: EcoString) -> Self {
        let body = body_from_url(&url);
        Self::new(LinkTarget::Dest(Destination::Url(url)), body)
    }
}

impl Show for LinkElem {
    fn show(&self, vt: &mut Vt, _: StyleChain) -> SourceResult<Content> {
        let body = self.body();
        let dest = match self.dest() {
            LinkTarget::Dest(dest) => dest,
            LinkTarget::Label(label) => {
                if !vt.introspector.init() {
                    return Ok(body);
                }

                let elem = vt.introspector.query_label(&label).at(self.span())?;
                Destination::Location(elem.location().unwrap())
            }
        };

        Ok(body
            .linked(dest)
            .styled(TextElem::set_hyphenate(Hyphenate(Smart::Custom(false)))))
    }
}

fn body_from_url(url: &EcoString) -> Content {
    let mut text = url.as_str();
    for prefix in ["mailto:", "tel:"] {
        text = text.trim_start_matches(prefix);
    }
    let shorter = text.len() < url.len();
    TextElem::packed(if shorter { text.into() } else { url.clone() })
}

/// A target where a link can go.
#[derive(Debug, Clone)]
pub enum LinkTarget {
    Dest(Destination),
    Label(Label),
}

cast_from_value! {
    LinkTarget,
    v: Destination => Self::Dest(v),
    v: Label => Self::Label(v),
}

cast_to_value! {
    v: LinkTarget => match v {
        LinkTarget::Dest(v) => v.into(),
        LinkTarget::Label(v) => v.into(),
    }
}

impl From<Destination> for LinkTarget {
    fn from(dest: Destination) -> Self {
        Self::Dest(dest)
    }
}
