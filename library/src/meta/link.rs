use crate::prelude::*;
use crate::text::{Hyphenate, TextElem};

/// Link to a URL or another location in the document.
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
#[element(Show, Finalize)]
pub struct LinkElem {
    /// The destination the link points to.
    ///
    /// - To link to web pages, `dest` should be a valid URL string. If the URL
    ///   is in the `mailto:` or `tel:` scheme and the `body` parameter is
    ///   omitted, the email address or phone number will be the link's body,
    ///   without the scheme.
    ///
    /// - To link to another part of the document, `dest` can take one of two
    ///   forms: A [`location`]($func/locate) or a dictionary with a `page` key
    ///   of type `integer` and `x` and `y` coordinates of type `length`. Pages
    ///   are counted from one, and the coordinates are relative to the page's
    ///   top left corner.
    ///
    /// ```example
    /// #link("mailto:hello@typst.app") \
    /// #link((page: 1, x: 0pt, y: 0pt))[
    ///   Go to top
    /// ]
    /// ```
    #[required]
    #[parse(
        let dest = args.expect::<Destination>("destination")?;
        dest.clone()
    )]
    pub dest: Destination,

    /// How the link is represented.
    ///
    /// The content that should become a link. If `dest` is an URL string, the
    /// parameter can be omitted. In this case, the URL will be shown as the
    /// link.
    #[required]
    #[parse(match &dest {
        Destination::Url(url) => match args.eat()? {
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
        Self::new(Destination::Url(url), body)
    }
}

impl Show for LinkElem {
    fn show(&self, _: &mut Vt, _: StyleChain) -> SourceResult<Content> {
        Ok(self.body())
    }
}

impl Finalize for LinkElem {
    fn finalize(&self, realized: Content, _: StyleChain) -> Content {
        realized
            .linked(self.dest())
            .styled(TextElem::set_hyphenate(Hyphenate(Smart::Custom(false))))
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
