use crate::prelude::*;
use crate::text::{Hyphenate, TextNode};

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
/// ## Parameters
/// - dest: `Destination` (positional, required)
///   The destination the link points to.
///
///   - To link to web pages, `dest` should be a valid URL string. If the URL is
///     in the `mailto:` or `tel:` scheme and the `body` parameter is omitted,
///     the email address or phone number will be the link's body, without the
///     scheme.
///
///   - To link to another part of the document, `dest` must contain a
///     dictionary with a `page` key of type `integer` and `x` and `y`
///     coordinates of type `length`. Pages are counted from one, and the
///     coordinates are relative to the page's top left corner.
///
///   ```example
///   #link("mailto:hello@typst.app") \
///   #link((page: 1, x: 0pt, y: 0pt))[
///     Go to top
///   ]
///   ```
///
/// - body: `Content` (positional)
///
///   The content that should become a link. If `dest` is an URL string, the
///   parameter can be omitted. In this case, the URL will be shown as the link.
///
/// Display: Link
/// Category: meta
#[node(Construct, Show, Finalize)]
pub struct LinkNode {
    /// The destination the link points to.
    #[positional]
    #[required]
    pub dest: Destination,

    /// How the link is represented.
    #[positional]
    #[default]
    pub body: Content,
}

impl LinkNode {
    /// Create a link node from a URL with its bare text.
    pub fn from_url(url: EcoString) -> Self {
        let body = body_from_url(&url);
        Self::new(Destination::Url(url)).with_body(body)
    }
}

impl Construct for LinkNode {
    fn construct(_: &Vm, args: &mut Args) -> SourceResult<Content> {
        let dest = args.expect::<Destination>("destination")?;
        let body = match &dest {
            Destination::Url(url) => match args.eat()? {
                Some(body) => body,
                None => body_from_url(url),
            },
            Destination::Internal(_) => args.expect("body")?,
        };
        Ok(Self::new(dest).with_body(body).pack())
    }
}

impl Show for LinkNode {
    fn show(&self, _: &mut Vt, _: &Content, _: StyleChain) -> SourceResult<Content> {
        Ok(self.body())
    }
}

impl Finalize for LinkNode {
    fn finalize(&self, realized: Content) -> Content {
        realized
            .styled(MetaNode::DATA, vec![Meta::Link(self.dest())])
            .styled(TextNode::HYPHENATE, Hyphenate(Smart::Custom(false)))
    }
}

fn body_from_url(url: &EcoString) -> Content {
    let mut text = url.as_str();
    for prefix in ["mailto:", "tel:"] {
        text = text.trim_start_matches(prefix);
    }
    let shorter = text.len() < url.len();
    TextNode::packed(if shorter { text.into() } else { url.clone() })
}
