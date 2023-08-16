use crate::prelude::*;

/// Allows you to separate content into its different parts (according to the usual layouting)
/// and work with them as separate elements.
///
/// # Example
///
/// ```example
/// #show split.item: it => {
///     let color = if it.index == 0 {
///         red
///     } else if it.index == it.count - 1 {
///         blue
/// 	} else {
///         green
///     }
///     rect(stroke: 1pt + color, it.body)
/// }
///
/// #columns(3, split(lorem(100)))
/// ```
///
/// Display: Split
/// Category: layout
#[element(Layout)]
#[scope(
    scope.define("item", SplitItem::func());
    scope
)]
pub struct SplitElem {
    /// The content to be split.
    #[positional]
    pub body: Content,
}

impl Layout for SplitElem {
    fn layout(
        &self,
        vt: &mut Vt,
        styles: StyleChain,
        regions: Regions,
    ) -> SourceResult<Fragment> {
        let body = self.body(styles);
        let body_frames = body.layout(vt, styles, regions)?.into_frames();
        let n = body_frames.len();
        let res = Content::sequence(body_frames.iter().enumerate().map(|(i, frame)| {
            let body = LayoutedContent::new().with_index(i).with_size(frame.size().map(Rel::from));
            SplitItem::new()
                .with_index(i)
                .with_count(n)
                .with_body(body.pack())
                .pack()
        }))
        .layout(vt, styles, regions)?;
        let mut out_frames = res.into_frames();
        for frame in &mut out_frames {
            frame.deep_replace_placeholders(&body_frames);
        }
        Ok(Fragment::frames(out_frames))
    }
}

/// A single part of split content.
///
/// On its own, this does nothing; It is intended to be used in a `show` rule.
///
/// Display: Split Item
/// Category: layout
#[element(Layout)]
pub struct SplitItem {
    /// The index of this part
    pub index: usize,
    /// The number of total parts
    pub count: usize,
    /// (A stand-in for) the content for this part
    pub body: Content,
}

impl Layout for SplitItem {
    fn layout(
        &self,
        vt: &mut Vt,
        styles: StyleChain,
        regions: Regions,
    ) -> SourceResult<Fragment> {
        self.body(styles).layout(vt, styles, regions)
    }
}

/// A stand-in for content that has already been fully layouted.
///
/// This is conceptually an opaque rectangle with already layouted content inside
/// (which can't be accessed anymore). It is only used in conjunction with `split`.
///
/// Display: Pre-Layouted content
/// Category: layout
#[element(Layout)]
pub struct LayoutedContent {
    /// The index of the part this is a stand-in for.
    index: usize,
    #[internal]
    size: Axes<Rel<Length>>,
}

impl Layout for LayoutedContent {
    fn layout(
        &self,
        _vt: &mut Vt,
        styles: StyleChain,
        regions: Regions,
    ) -> SourceResult<Fragment> {
        let size = self
            .size(styles)
            .resolve(styles)
            .zip(regions.base())
            .map(|(len, b)| len.relative_to(b));
        let mut res = Frame::new(size);
        res.push(Point::zero(), FrameItem::Placeholder(self.index(styles)));

        Ok(Fragment::frame(res))
    }
}
