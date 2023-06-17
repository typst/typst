use std::{collections::HashMap, sync::Arc};

#[cfg(feature = "rkyv")]
use rkyv::{Archive, Deserialize as rDeser, Serialize as rSer};

use crate::export::svg::{
    geom::{Abs, Point, Size},
    ir::{
        AbsoulteRef, DefId, Fingerprint, FingerprintBuilder, GlyphItem, GlyphMapping,
        ImageGlyphItem, ImageItem, ImmutStr, LinkItem, OutlineGlyphItem, PathItem,
        SvgItem, TextShape, TransformItem,
    },
};

/// Flatten svg item.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "rkyv", derive(Archive, rDeser, rSer))]
#[cfg_attr(feature = "rkyv-validation", archive(check_bytes))]
pub enum FlatSvgItem {
    None,
    Image(ImageItem),
    Link(LinkItem),
    Path(PathItem),
    Text(FlatTextItem),
    Item(TransformedRef),
    Group(GroupRef),
}

/// Flatten text item.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "rkyv", derive(Archive, rDeser, rSer))]
#[cfg_attr(feature = "rkyv-validation", archive(check_bytes))]
pub struct FlatTextItem {
    pub content: Arc<FlatTextItemContent>,
    pub shape: Arc<TextShape>,
}

/// The content metadata of a [`FlatTextItem`].
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "rkyv", derive(Archive, rDeser, rSer))]
#[cfg_attr(feature = "rkyv-validation", archive(check_bytes))]
pub struct FlatTextItemContent {
    pub content: ImmutStr,
    pub glyphs: Arc<[(Abs, Abs, AbsoulteRef)]>,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "rkyv", derive(Archive, rDeser, rSer))]
#[cfg_attr(feature = "rkyv-validation", archive(check_bytes))]
pub enum FlatGlyphItem {
    Image(Arc<ImageGlyphItem>),
    Outline(Arc<OutlineGlyphItem>),
}

impl From<FlatGlyphItem> for GlyphItem {
    fn from(item: FlatGlyphItem) -> Self {
        match item {
            FlatGlyphItem::Image(item) => GlyphItem::Image(item),
            FlatGlyphItem::Outline(item) => GlyphItem::Outline(item),
        }
    }
}

/// Flatten transform item.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "rkyv", derive(Archive, rDeser, rSer))]
#[cfg_attr(feature = "rkyv-validation", archive(check_bytes))]
pub struct TransformedRef(pub TransformItem, pub AbsoulteRef);

/// Flatten group item.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "rkyv", derive(Archive, rDeser, rSer))]
#[cfg_attr(feature = "rkyv-validation", archive(check_bytes))]
pub struct GroupRef(pub Arc<[(Point, AbsoulteRef)]>);

#[derive(Debug, Default)]
#[cfg_attr(feature = "rkyv", derive(Archive, rDeser, rSer))]
#[cfg_attr(feature = "rkyv-validation", archive(check_bytes))]
pub struct ItemPack(pub Vec<(Fingerprint, FlatSvgItem)>);

/// A finished module that stores all the svg items.
/// The svg items shares the underlying data.
/// The svg items are flattened and ready to be serialized.
#[derive(Debug, Default)]
pub struct Module {
    pub glyphs: Vec<(AbsoulteRef, GlyphItem)>,
    pub item_pack: ItemPack,
}

impl Module {
    /// Get a glyph item by its stable ref.
    pub fn get_glyph(&self, id: &AbsoulteRef) -> Option<&GlyphItem> {
        self.glyphs.get(id.id.0 as usize).map(|(_, item)| item)
    }

    /// Get a svg item by its stable ref.
    pub fn get_item(&self, id: &AbsoulteRef) -> Option<&FlatSvgItem> {
        self.item_pack.0.get(id.id.0 as usize).map(|(_, item)| item)
    }
}

pub type Pages = Vec<(AbsoulteRef, Size)>;
pub type LayoutElem = (Abs, Pages);

/// Module with page references of a [`typst::doc::Document`].
pub struct SvgDocument {
    pub module: Module,
    /// References to the page frames.
    /// Use [`Module::get_item`] to get the actual item.
    pub pages: Pages,
}

/// Module with multiple documents in a module [`typst::doc::Document`].
pub struct MultiSvgDocument {
    pub module: Module,
    /// References to the page frames.
    /// Use [`Module::get_item`] to get the actual item.
    pub layouts: Vec<(Abs, Pages)>,
}

impl MultiSvgDocument {
    #[cfg(feature = "rkyv")]
    pub fn from_slice(v: &[u8]) -> Self {
        use rkyv::de::deserializers::SharedDeserializeMap;

        let mut aligned = rkyv::AlignedVec::default();
        let v = if (v.as_ptr() as usize) % rkyv::AlignedVec::ALIGNMENT != 0 {
            aligned.extend_from_slice(v);
            aligned.as_slice()
        } else {
            v
        };

        let archived = rkyv::check_archived_root::<SerializedModule>(v).unwrap();

        let item_pack: ItemPack = {
            let mut dmap = SharedDeserializeMap::default();
            archived.item_pack.deserialize(&mut dmap).unwrap()
        };

        let layouts = {
            let mut infallible = rkyv::Infallible::default();
            archived.layouts.deserialize(&mut infallible).unwrap()
        };

        let glyphs = {
            let mut dmap = SharedDeserializeMap::default();
            let glyphs: Vec<(AbsoulteRef, FlatGlyphItem)> =
                archived.glyphs.deserialize(&mut dmap).unwrap();
            glyphs
                .into_iter()
                .map(|(abs_ref, glyph)| (abs_ref, glyph.into()))
                .collect()
        };

        MultiSvgDocument { module: Module { glyphs, item_pack }, layouts }
    }
}

/// Intermediate representation of a incompleted svg item.
#[derive(Default)]
pub struct ModuleBuilder {
    pub glyphs: GlyphMapping,
    pub items: Vec<(Fingerprint, FlatSvgItem)>,
    pub item_pos: HashMap<Fingerprint, DefId>,

    fingerprint_builder: FingerprintBuilder,
}

impl ModuleBuilder {
    pub fn finalize_ref(&self) -> (Module, GlyphMapping) {
        let mut glyphs = self.glyphs.clone().into_iter().collect::<Vec<_>>();
        glyphs.sort_by(|(_, a), (_, b)| a.id.0.cmp(&b.id.0));
        (
            Module {
                glyphs: glyphs.into_iter().map(|(a, b)| (b, a)).collect(),
                item_pack: ItemPack(self.items.clone()),
            },
            self.glyphs.clone(),
        )
    }

    pub fn finalize(self) -> (Module, GlyphMapping) {
        let mut glyphs = self.glyphs.clone().into_iter().collect::<Vec<_>>();
        glyphs.sort_by(|(_, a), (_, b)| a.id.0.cmp(&b.id.0));
        (
            Module {
                glyphs: glyphs.into_iter().map(|(a, b)| (b, a)).collect(),
                item_pack: ItemPack(self.items),
            },
            self.glyphs,
        )
    }

    pub fn build_glyph(&mut self, glyph: &GlyphItem) -> AbsoulteRef {
        if let Some(id) = self.glyphs.get(glyph) {
            return id.clone();
        }

        let id = DefId(self.glyphs.len() as u64);

        let fingerprint = self.fingerprint_builder.resolve(glyph);
        let abs_ref = AbsoulteRef { fingerprint, id };
        self.glyphs.insert(glyph.clone(), abs_ref.clone());
        abs_ref
    }

    pub fn build(&mut self, item: SvgItem) -> AbsoulteRef {
        let resolved_item = match item {
            SvgItem::Image(image) => FlatSvgItem::Image(image),
            SvgItem::Path(path) => FlatSvgItem::Path(path),
            SvgItem::Link(link) => FlatSvgItem::Link(link),
            SvgItem::Text(text) => {
                let glyphs = text
                    .content
                    .glyphs
                    .iter()
                    .cloned()
                    .map(|(offset, advance, glyph)| {
                        (offset, advance, self.build_glyph(&glyph))
                    })
                    .collect::<Arc<_>>();
                let shape = text.shape.clone();
                let content = text.content.content.clone();
                FlatSvgItem::Text(FlatTextItem {
                    content: Arc::new(FlatTextItemContent { content, glyphs }),
                    shape,
                })
            }
            SvgItem::Transformed(transformed) => {
                let item = &transformed.1;
                let item_id = self.build(*item.clone());
                let transform = transformed.0.clone();

                FlatSvgItem::Item(TransformedRef(transform, item_id))
            }
            SvgItem::Group(group) => {
                let items = group
                    .0
                    .iter()
                    .map(|(point, item)| (*point, self.build(item.clone())))
                    .collect::<Vec<_>>();
                FlatSvgItem::Group(GroupRef(items.into()))
            }
        };

        let fingerprint = self.fingerprint_builder.resolve(&resolved_item);

        if let Some(pos) = self.item_pos.get(&fingerprint) {
            return AbsoulteRef { fingerprint, id: *pos };
        }

        let id = DefId(self.items.len() as u64);
        self.items.push((fingerprint, resolved_item));
        self.item_pos.insert(fingerprint, id);
        AbsoulteRef { fingerprint, id }
    }
}

/// Flatten transform item.
#[derive(Debug)]
#[cfg_attr(feature = "rkyv", derive(Archive, rDeser, rSer))]
#[cfg_attr(feature = "rkyv-validation", archive(check_bytes))]
pub struct SerializedModule {
    pub item_pack: ItemPack,
    pub glyphs: Vec<(AbsoulteRef, FlatGlyphItem)>,
    pub layouts: Vec<(Abs, Vec<(AbsoulteRef, Size)>)>,
}
