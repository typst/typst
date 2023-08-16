use crate::prelude::*;

/// Display: Split
/// Category: layout
#[element(Layout)]
#[scope(
    scope.define("item", SplitItem::func());
    scope
)]
pub struct SplitElem {
    /// The contents to be broken.
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

/// Display: Split Item
/// Category: layout
#[element(Layout)]
pub struct SplitItem {
    pub index: usize,
    pub count: usize,
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

/// Display: Pre-Layouted content
/// Category: layout
#[element(Layout)]
pub struct LayoutedContent {
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
