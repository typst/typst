use crate::diag::{At, SourceResult};
use crate::engine::Engine;
use crate::foundations::{Content, Packed, Scope, Show, Smart, StyleChain};
use crate::introspection::Locator;
use crate::layout::{BlockElem, Frame, FrameItem, Point, Region, Rel, Size, Sizing};
use crate::loading::Readable;
use crate::text::LocalName;
use crate::World;
use ecow::EcoString;
use typst::foundations::NativeElement;
use typst_macros::{elem, func, scope};
use typst_syntax::Spanned;

/// Hook up the embed definition.
pub(super) fn define(global: &mut Scope) {
    global.define_elem::<EmbedElem>();
}

#[elem(scope, Show, LocalName)]
pub struct EmbedElem {
    /// Path to a file to be embedded
    ///
    /// For more details, see the [Paths section]($syntax/#paths).
    #[required]
    #[parse(
        let Spanned { v: path, span } =
        args.expect::<Spanned<EcoString>>("path to image file")?;
        let id = span.resolve_path(&path).at(span)?;
        let data = engine.world.file(id).at(span)?;
        path
    )]
    #[borrowed]
    pub path: EcoString,

    /// The raw file data.
    #[internal]
    #[required]
    #[parse(Readable::Bytes(data))]
    pub data: Readable,
}

#[scope]
impl EmbedElem {
    #[func(title = "Embed the given file")]
    fn file(
        /// The engine.
        engine: &mut Engine,
        /// Path to a file.
        ///
        /// For more details, see the [Paths section]($syntax/#paths).
        path: Spanned<EcoString>,
    ) -> SourceResult<Content> {
        let Spanned { v: path, span } = path;
        let id = span.resolve_path(&path).at(span)?;
        let data = engine.world.file(id).at(span)?;
        let elem = EmbedElem::new(path, Readable::Bytes(data));

        Ok(elem.pack().spanned(span))
    }
}

impl LocalName for Packed<EmbedElem> {
    const KEY: &'static str = "embedding";
}

impl Show for Packed<EmbedElem> {
    fn show(&self, _: &mut Engine, _: StyleChain) -> SourceResult<Content> {
        Ok(BlockElem::single_layouter(self.clone(), layout_embedding)
            .with_width(Smart::Custom(Rel::zero()))
            .with_height(Sizing::Rel(Rel::zero()))
            .pack()
            .spanned(self.span()))
    }
}

/// Layout the embedding.
#[typst_macros::time(span = elem.span())]
fn layout_embedding(
    elem: &Packed<EmbedElem>,
    _: &mut Engine,
    _: Locator,
    _: StyleChain,
    _: Region,
) -> SourceResult<Frame> {
    let mut frame = Frame::soft(Size::zero());
    frame.push(Point::zero(), FrameItem::Embed(elem.clone().unpack()));

    Ok(frame)
}
