use std::num::NonZeroUsize;

use crate::diag::SourceResult;
use crate::engine::Engine;
use crate::foundations::{elem, Content, NativeElement, Packed, Show, StyleChain};
use crate::introspection::Locator;
use crate::layout::{
    layout_fragment_with_columns, BlockElem, Fragment, Length, Ratio, Regions, Rel,
};

/// Separates a region into multiple equally sized columns.
///
/// The `column` function allows to separate the interior of any container into
/// multiple columns. It will not equalize the height of the columns, instead,
/// the columns will take up the height of their container or the remaining
/// height on the page. The columns function can break across pages if
/// necessary.
///
/// If you need to insert columns across your whole document, you can use the
/// [`{page}` function's `columns` parameter]($page.columns) instead.
///
/// # Example
/// ```example
/// = Towards Advanced Deep Learning
///
/// #box(height: 68pt,
///  columns(2, gutter: 11pt)[
///    #set par(justify: true)
///    This research was funded by the
///    National Academy of Sciences.
///    NAoS provided support for field
///    tests and interviews with a
///    grant of up to USD 40.000 for a
///    period of 6 months.
///  ]
/// )
///
/// In recent years, deep learning has
/// increasingly been used to solve a
/// variety of problems.
/// ```
#[elem(Show)]
pub struct ColumnsElem {
    /// The number of columns.
    #[positional]
    #[default(NonZeroUsize::new(2).unwrap())]
    pub count: NonZeroUsize,

    /// The size of the gutter space between each column.
    #[resolve]
    #[default(Ratio::new(0.04).into())]
    pub gutter: Rel<Length>,

    /// The content that should be layouted into the columns.
    #[required]
    pub body: Content,
}

impl Show for Packed<ColumnsElem> {
    fn show(&self, _: &mut Engine, _: StyleChain) -> SourceResult<Content> {
        Ok(BlockElem::multi_layouter(self.clone(), layout_columns)
            .with_rootable(true)
            .pack()
            .spanned(self.span()))
    }
}

/// Layout the columns.
#[typst_macros::time(span = elem.span())]
fn layout_columns(
    elem: &Packed<ColumnsElem>,
    engine: &mut Engine,
    locator: Locator,
    styles: StyleChain,
    regions: Regions,
) -> SourceResult<Fragment> {
    layout_fragment_with_columns(
        engine,
        &elem.body,
        locator,
        styles,
        regions,
        elem.count(styles),
        elem.gutter(styles),
    )
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
