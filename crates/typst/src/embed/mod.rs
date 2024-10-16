use crate::diag::{At, SourceResult};
use crate::engine::Engine;
use crate::foundations::{Bytes, Content, Packed, Scope, Show, Smart, StyleChain};
use crate::introspection::Locator;
use crate::layout::{BlockElem, Frame, FrameItem, Point, Region, Rel, Size, Sizing};
use crate::loading::Readable;
use crate::text::LocalName;
use crate::World;
use ecow::EcoString;
use std::sync::Arc;
use typst::foundations::NativeElement;
use typst_macros::{elem, func, scope};
use typst_syntax::Spanned;
use typst_utils::LazyHash;

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

    /// The name of the attached file
    ///
    /// If no name is given, the path is used instead
    #[borrowed]
    pub name: Option<EcoString>,

    /// A description for the attached file
    #[borrowed]
    pub description: Option<EcoString>,
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
    let mut frame = Frame::hard(Size::zero());
    frame.push(Point::zero(), FrameItem::Embed(Embed::from_element(elem)));

    Ok(frame)
}

/// A loaded file to be embedded.
///
/// Values of this type are cheap to clone and hash.
#[derive(Clone, Hash, Eq, PartialEq)]
pub struct Embed(Arc<LazyHash<Repr>>);

/// The internal representation of a file embedding
#[derive(Hash)]
struct Repr {
    /// The raw file data.
    data: Bytes,
    /// Path of this embedding
    path: EcoString,
    /// Name of this embedding
    name: EcoString,
    /// Name of this embedding
    description: Option<EcoString>,
}

impl Embed {
    fn from_element(element: &Packed<EmbedElem>) -> Self {
        let repr = Repr {
            data: element.data.clone().into(),
            path: element.path.clone(),
            name: if let Some(Some(name)) = element.name.as_ref() {
                name.clone()
            } else {
                element.path.clone()
            },
            description: if let Some(Some(description)) = element.description.as_ref() {
                Some(description.clone())
            } else {
                None
            },
        };

        Embed(Arc::new(LazyHash::new(repr)))
    }

    /// The raw file data.
    pub fn data(&self) -> &Bytes {
        &self.0.data
    }

    /// The name of the file embedding
    pub fn name(&self) -> &EcoString {
        &self.0.name
    }

    /// The path of the file embedding
    pub fn path(&self) -> &EcoString {
        &self.0.path
    }

    /// The description of the file embedding
    pub fn description(&self) -> Option<&str> {
        self.0.description.as_deref()
    }
}
