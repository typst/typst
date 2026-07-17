use std::num::NonZeroUsize;

use crate::foundations::{Content, elem};
use crate::layout::{Length, Ratio, Rel};

/// Separates a region into multiple equally sized columns.
///
/// The `column` function lets you separate the contents of any container into
/// multiple columns. By default, columns take up the height of their container
/// or the remaining height on the page and are filled up one after another.
/// Use @colbreak to explicitly end a column and continue in the next one.
/// Use the @columns.balanced[`balanced` parameter] to automatically equalize
/// the height of the columns.
///
/// = Example <example>
/// ```example
/// #columns(2, gutter: 8pt)[
///   This text is in the
///   first column.
///
///   #colbreak()
///
///   This text is in the
///   second column.
/// ]
/// ```
///
/// = #short-or-long[Page Level][Page-level columns] <page-level>
/// If you need to insert columns across your whole document, use the `{page}`
/// function's @page.columns[`columns` parameter] instead. This will create the
/// columns directly at the page-level rather than wrapping all of your content
/// in a layout container. As a result, things like @pagebreak[pagebreaks],
/// @footnote[footnotes], and @par.line[line numbers] will continue to work as
/// expected. For more information, also read the
/// @guides:page-setup:columns[relevant part of the page setup guide].
///
/// = #short-or-long[Breaking Out][Breaking out of columns] <breaking-out>
/// To temporarily break out of columns (e.g. for a paper's title), use
/// parent-scoped floating placement:
///
/// #example(
///   single: true,
///   ```
///   #set page(columns: 2, height: 150pt)
///
///   #place(
///     top + center,
///     scope: "parent",
///     float: true,
///     text(1.4em, weight: "bold")[
///       My document
///     ],
///   )
///
///   #lorem(40)
///   ```
/// )
#[elem]
pub struct ColumnsElem {
    /// The number of columns.
    #[positional]
    #[default(NonZeroUsize::new(2).unwrap())]
    pub count: NonZeroUsize,

    /// The size of the gutter space between each column.
    ///
    /// #example(
    /// ```
    /// #set page(columns: 2, height: 5cm)
    /// #set par(justify: true)
    /// #set columns(gutter: 15mm)
    /// #lorem(30)
    /// ```
    /// )
    #[default(Ratio::new(0.04).into())]
    pub gutter: Rel<Length>,

    /// Whether to equalize the height of columns by breaking columns early.
    ///
    /// #example(
    /// ```
    /// #set page(columns: 2, height: 5cm)
    /// #set par(justify: true)
    /// >>> #set columns(gutter: 15pt)
    /// #set columns(balanced: true)
    /// #lorem(30)
    /// ```
    /// )
    #[default(false)]
    pub balanced: bool,

    /// The content that should be layouted into the columns.
    #[required]
    pub body: Content,
}

/// Forces a column break.
///
/// The function will behave like a @pagebreak[page break] when used in a single
/// column layout or the last column on a page. Otherwise, content after the
/// column break will be placed in the next column.
///
/// = Example <example>
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
