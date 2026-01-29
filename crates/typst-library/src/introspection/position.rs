use std::num::NonZeroUsize;

use ecow::EcoVec;
use typst_utils::NonZeroExt;

use crate::foundations::{Dict, Value, cast, dict};
use crate::layout::{Length, Point};

/// Physical position in a document, be it paged or HTML.
///
/// This type exists to make it possible to write functions that are generic
/// over the document target.
#[derive(Clone, Debug, Hash)]
pub enum DocumentPosition {
    /// If the document is paged, the position is expressed as coordinates
    /// inside of a page.
    Paged(PagedPosition),
    /// If the document is an HTML document, the position points to a specific
    /// node in the DOM tree.
    Html(HtmlPosition),
}

impl DocumentPosition {
    /// Returns the [`PagedPosition`] if this is one.
    pub fn as_paged(self) -> Option<PagedPosition> {
        match self {
            DocumentPosition::Paged(position) => Some(position),
            _ => None,
        }
    }

    /// Returns the [`PagedPosition`] or a position at page 1, point `(0, 0)` if
    /// this is not a paged position.
    pub fn as_paged_or_default(self) -> PagedPosition {
        self.as_paged().unwrap_or(PagedPosition::ORIGIN)
    }

    /// Returns the [`HtmlPosition`] if available.
    pub fn as_html(self) -> Option<HtmlPosition> {
        match self {
            DocumentPosition::Html(position) => Some(position),
            _ => None,
        }
    }
}

impl From<PagedPosition> for DocumentPosition {
    fn from(value: PagedPosition) -> Self {
        Self::Paged(value)
    }
}

impl From<HtmlPosition> for DocumentPosition {
    fn from(value: HtmlPosition) -> Self {
        Self::Html(value)
    }
}

/// A physical position in a paged document.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct PagedPosition {
    /// The page, starting at 1.
    pub page: NonZeroUsize,
    /// The exact coordinates on the page (from the top left, as usual).
    pub point: Point,
}

impl PagedPosition {
    /// A position at the origin of the first page.
    pub const ORIGIN: PagedPosition =
        PagedPosition { page: NonZeroUsize::ONE, point: Point::zero() };
}

cast! {
    PagedPosition,
    self => Value::Dict(self.into()),
    mut dict: Dict => {
        let page = dict.take("page")?.cast()?;
        let x: Length = dict.take("x")?.cast()?;
        let y: Length = dict.take("y")?.cast()?;
        dict.finish(&["page", "x", "y"])?;
        Self { page, point: Point::new(x.abs, y.abs) }
    },
}

impl From<PagedPosition> for Dict {
    fn from(pos: PagedPosition) -> Self {
        dict! {
            "page" => pos.page,
            "x" => pos.point.x,
            "y" => pos.point.y,
        }
    }
}

/// A position in an HTML tree.
#[derive(Clone, Debug, Hash)]
pub struct HtmlPosition {
    /// Indices that can be used to traverse the tree from the root.
    element: EcoVec<usize>,
    /// The precise position inside of the specified element.
    inner: Option<InnerHtmlPosition>,
}

impl HtmlPosition {
    /// A position in an HTML document pointing to a specific node as a whole.
    ///
    /// The items of the vector corresponds to indices that can be used to
    /// traverse the DOM tree from the root to reach the node. In practice, this
    /// means that the first item of the vector will often be `1` for the
    /// `<body>` tag (`0` being the `<head>` tag in a typical HTML document).
    ///
    /// Consecutive text nodes in Typst's HTML representation are grouped for
    /// the purpose of this indexing as the segmentation is not observable in
    /// the resulting DOM.
    pub fn new(element: EcoVec<usize>) -> Self {
        Self { element, inner: None }
    }

    /// Specifies a character offset inside of the node, to build a position
    /// pointing to a specific point in text.
    ///
    /// This only makes sense if the node is a text node, not an element or a
    /// frame.
    ///
    /// The offset is expressed in codepoints, not in bytes, to be
    /// encoding-independent.
    pub fn at_char(self, offset: usize) -> Self {
        Self {
            element: self.element,
            inner: Some(InnerHtmlPosition::Character(offset)),
        }
    }

    /// Specifies a point in a frame, to build a more precise position.
    ///
    /// This only makes sense if the node is a frame.
    pub fn in_frame(self, point: Point) -> Self {
        Self {
            element: self.element,
            inner: Some(InnerHtmlPosition::Frame(point)),
        }
    }

    /// Extra-information for a more precise location inside of the node
    /// designated by [`HtmlPosition::element`].
    pub fn details(&self) -> Option<&InnerHtmlPosition> {
        self.inner.as_ref()
    }

    /// Indices for traversing an HTML tree to reach the node corresponding to
    /// this position.
    ///
    /// See [`HtmlPosition::new`] for more details.
    pub fn element(&self) -> impl Iterator<Item = &usize> {
        self.element.iter()
    }
}

/// A precise position inside of an HTML node.
#[derive(Clone, Debug, Hash)]
pub enum InnerHtmlPosition {
    /// If the node is a frame, the coordinates of the position.
    Frame(Point),
    /// If the node is a text node, the index of the codepoint at the position.
    Character(usize),
}
