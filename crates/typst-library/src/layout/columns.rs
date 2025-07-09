use std::num::NonZeroUsize;

use crate::foundations::{elem, Content};
use crate::layout::{Length, Ratio, Rel};

/// Separates a region into multiple equally sized columns.
///
/// The `column` function lets you separate the interior of any container into
/// multiple columns. It will currently not balance the height of the columns.
/// Instead, the columns will take up the height of their container or the
/// remaining height on the page. Support for balanced columns is planned for
/// the future.
///
/// # Page-level columns { #page-level }
/// If you need to insert columns across your whole document, use the `{page}`
/// function's [`columns` parameter]($page.columns) instead. This will create
/// the columns directly at the page-level rather than wrapping all of your
/// content in a layout container. As a result, things like
/// [pagebreaks]($pagebreak), [footnotes]($footnote), and [line
/// numbers]($par.line) will continue to work as expected. For more information,
/// also read the [relevant part of the page setup
/// guide]($guides/page-setup-guide/#columns).
///
/// # Breaking out of columns { #breaking-out }
/// To temporarily break out of columns (e.g. for a paper's title), use
/// parent-scoped floating placement:
///
/// ```example:single
/// #set page(columns: 2, height: 150pt)
///
/// #place(
///   top + center,
///   scope: "parent",
///   float: true,
///   text(1.4em, weight: "bold")[
///     My document
///   ],
/// )
///
/// #lorem(40)
/// ```
#[elem]
pub struct ColumnsElem {
    /// The number of columns.
    #[positional]
    #[default(NonZeroUsize::new(2).unwrap())]
    pub count: NonZeroUsize,

    /// The size of the gutter space between each column.
    #[default(Ratio::new(0.04).into())]
    pub gutter: Rel<Length>,

    /// The content that should be layouted into the columns.
    #[required]
    pub body: Content,
}

/// Forces a column break.
///
/// The function will behave like a [page break]($pagebreak) when used in a
/// single column layout or the last column on a page. Otherwise, content after
/// the column break will be placed in the next column.
///
/// # Example
/// ```example
/// #set page(columns: 2)
/// Preliminary findings from our
/// ongoing research project have
/// revealed a hitherto unknown
/// phenomenon of extraordinary
/// significance.
///
/// #colbreak()
/// Through rigorous experimentation
/// and analysis, we have discovered
/// a hitherto uncharacterized process
/// that defies our current
/// understanding of the fundamental
/// laws of nature.
/// ```
#[elem(title = "Column Break")]
pub struct ColbreakElem {
    /// If `{true}`, the column break is skipped if the current column is
    /// already empty.
    #[default(false)]
    pub weak: bool,
}
