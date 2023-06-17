use std::any::Any;
use std::collections::HashMap;
use std::hash::Hash;
use std::sync::Arc;

use crate::export::svg::{hash::typst_affinite_hash, hash::StaticHash128};
use base64::Engine;
use siphasher::sip128::{Hasher128, SipHasher13};
use ttf_parser::GlyphId;
use typst::{
    font::Font,
    image::{ImageFormat, RasterFormat, VectorFormat},
};

#[cfg(feature = "rkyv")]
use rkyv::{Archive, Deserialize as rDeser, Serialize as rSer};

pub type ImmutStr = Arc<str>;

pub use super::geom::*;

/// See <https://github.com/rust-lang/rust/blob/master/compiler/rustc_hir/src/stable_hash_impls.rs#L22>
/// The fingerprint conflicts should be very rare and should be handled by the compiler.
///
/// > That being said, given a high quality hash function, the collision
/// > probabilities in question are very small. For example, for a big crate like
/// > `rustc_middle` (with ~50000 `LocalDefId`s as of the time of writing) there
/// > is a probability of roughly 1 in 14,750,000,000 of a crate-internal
/// > collision occurring. For a big crate graph with 1000 crates in it, there is
/// > a probability of 1 in 36,890,000,000,000 of a `StableCrateId` collision.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "rkyv", derive(Archive, rDeser, rSer))]
#[cfg_attr(feature = "rkyv-validation", archive(check_bytes))]
pub struct Fingerprint(u64, u64);

/// A fingerprint hasher that extends the [`std::hash::Hasher`] trait.
pub trait FingerprintHasher: std::hash::Hasher {
    /// Finish the fingerprint and return the fingerprint and the data.
    /// The data is used to resolve the conflict.
    fn finish_fingerprint(&self) -> (Fingerprint, Vec<u8>);
}

/// A fingerprint hasher that uses the [`SipHasher13`] algorithm.
struct FingerprintSipHasher {
    /// The underlying data passed to the hasher.
    data: Vec<u8>,
}

impl std::hash::Hasher for FingerprintSipHasher {
    fn write(&mut self, bytes: &[u8]) {
        self.data.extend_from_slice(bytes);
    }

    fn finish(&self) -> u64 {
        let buffer = self.data.clone();
        let mut inner = SipHasher13::new();
        buffer.hash(&mut inner);
        inner.finish()
    }
}

impl FingerprintHasher for FingerprintSipHasher {
    fn finish_fingerprint(&self) -> (Fingerprint, Vec<u8>) {
        let buffer = self.data.clone();
        let mut inner = SipHasher13::new();
        buffer.hash(&mut inner);
        let hash = inner.finish128();
        (Fingerprint(hash.h1, hash.h2), buffer)
    }
}

/// A fingerprint builder that produces unique fingerprint for each item.
/// It resolves the conflict by checking the underlying data.
/// See [`Fingerprint`] for more information.
#[derive(Default)]
pub struct FingerprintBuilder {
    /// The conflict checker mapping fingerprints to their underlying data.
    conflict_checker: HashMap<Fingerprint, Vec<u8>>,
}

impl FingerprintBuilder {
    pub fn resolve<T: Hash + 'static>(&mut self, item: &T) -> Fingerprint {
        let mut s = FingerprintSipHasher { data: Vec::new() };
        item.type_id().hash(&mut s);
        item.hash(&mut s);
        let (fingerprint, featured_data) = s.finish_fingerprint();
        if let Some(prev_featured_data) = self.conflict_checker.get(&fingerprint) {
            if prev_featured_data != &featured_data {
                // todo: soft error
                panic!("Fingerprint conflict detected!");
            }

            return fingerprint;
        }

        self.conflict_checker.insert(fingerprint, featured_data);
        fingerprint
    }
}

/// The local id of a svg item.
/// This id is only unique within the svg document.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "rkyv", derive(Archive, rDeser, rSer))]
#[cfg_attr(feature = "rkyv-validation", archive(check_bytes))]
pub struct DefId(pub u64);

/// A stable absolute reference.
/// The fingerprint is used to identify the item and likely unique between different svg documents.
/// The (local) def id is only unique within the svg document.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "rkyv", derive(Archive, rDeser, rSer))]
#[cfg_attr(feature = "rkyv-validation", archive(check_bytes))]
pub struct AbsoulteRef {
    /// The fingerprint of the item.
    pub fingerprint: Fingerprint,
    /// The local def id of the item.
    pub id: DefId,
}

impl Hash for AbsoulteRef {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.fingerprint.hash(state);
    }
}

impl AbsoulteRef {
    /// Create a xml id from the given prefix and the fingerprint of this reference.
    /// Note that the entire html document shares namespace for ids.
    #[comemo::memoize]
    fn as_svg_id_inner(fingerprint: Fingerprint, prefix: &'static str) -> String {
        let fingerprint_hi = base64::engine::general_purpose::STANDARD_NO_PAD
            .encode(fingerprint.0.to_le_bytes());
        if fingerprint.1 == 0 {
            return [prefix, &fingerprint_hi].join("");
        }

        // possible the id in the lower 64 bits.
        let fingerprint_lo = {
            let id = fingerprint.1.to_le_bytes();
            // truncate zero
            let rev_zero = id.iter().rev().skip_while(|&&b| b == 0).count();
            let id = &id[..rev_zero];
            base64::engine::general_purpose::STANDARD_NO_PAD.encode(id)
        };
        [prefix, &fingerprint_hi, &fingerprint_lo].join("")
    }

    /// Create a xml id from the given prefix and the def id of this reference.
    /// Note that the def id may not be stable across compilation.
    /// Note that the entire html document shares namespace for ids.
    #[comemo::memoize]
    fn as_unstable_svg_id_inner(id: u64, prefix: &'static str) -> String {
        let id = {
            let id = id.to_le_bytes();
            // truncate zero
            let rev_zero = id.iter().rev().skip_while(|&&b| b == 0).count();
            let id = &id[..rev_zero];
            base64::engine::general_purpose::STANDARD_NO_PAD.encode(id)
        };
        [prefix, &id].join("")
    }

    #[inline]
    pub fn as_svg_id(&self, prefix: &'static str) -> String {
        Self::as_svg_id_inner(self.fingerprint, prefix)
    }

    #[inline]
    pub fn as_unstable_svg_id(&self, prefix: &'static str) -> String {
        Self::as_unstable_svg_id_inner(self.id.0, prefix)
    }
}

/// A Svg item that is specialized for representing [`typst::doc::Document`] or its subtypes.
#[derive(Debug, Clone)]
pub enum SvgItem {
    Image(ImageItem),
    Link(LinkItem),
    Path(PathItem),
    Text(TextItem),
    Transformed(TransformedItem),
    Group(GroupItem),
}

/// Data of an `<image/>` element.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "rkyv", derive(Archive, rDeser, rSer))]
#[cfg_attr(feature = "rkyv-validation", archive(check_bytes))]
pub struct Image {
    /// The encoded image data.
    pub data: Vec<u8>,
    /// The format of the encoded `buffer`.
    pub format: ImmutStr,
    /// The size of the image.
    pub size: Axes<u32>,
    /// A text describing the image.
    pub alt: Option<ImmutStr>,
    /// prehashed image content.
    pub hash: u128,
}

/// Collect image data from [`typst::image::Image`].
impl From<typst::image::Image> for Image {
    fn from(image: typst::image::Image) -> Self {
        let format = match image.format() {
            ImageFormat::Raster(e) => match e {
                RasterFormat::Jpg => "jpeg",
                RasterFormat::Png => "png",
                RasterFormat::Gif => "gif",
            },
            ImageFormat::Vector(e) => match e {
                VectorFormat::Svg => "svg+xml",
            },
        };

        // steal prehash from [`typst::image::Image`]
        let hash = typst_affinite_hash(&image);

        Image {
            data: image.data().to_vec(),
            format: format.into(),
            size: image.size().into(),
            alt: image.alt().map(|s| s.into()),
            hash,
        }
    }
}

impl Image {
    /// Returns the width of the image.
    pub fn width(&self) -> u32 {
        self.size.x
    }
    /// Returns the height of the image.
    pub fn height(&self) -> u32 {
        self.size.y
    }
}

/// Prehashed image data.
impl Hash for Image {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.hash.hash(state);
    }
}

impl StaticHash128 for Image {
    /// Returns the hash of the image data.
    fn get_hash(&self) -> u128 {
        self.hash
    }
}

/// Item representing an `<image/>` element.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "rkyv", derive(Archive, rDeser, rSer))]
#[cfg_attr(feature = "rkyv-validation", archive(check_bytes))]
pub struct ImageItem {
    /// The source image data.
    pub image: Arc<Image>,
    /// The target size of the image.
    pub size: Size,
}

/// Item representing an `<a/>` element.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "rkyv", derive(Archive, rDeser, rSer))]
#[cfg_attr(feature = "rkyv-validation", archive(check_bytes))]
pub struct LinkItem {
    /// The target of the link item.
    pub href: ImmutStr,
    /// The box size of the link item.
    pub size: Size,
}

/// Item representing an `<path/>` element.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "rkyv", derive(Archive, rDeser, rSer))]
#[cfg_attr(feature = "rkyv-validation", archive(check_bytes))]
pub struct PathItem {
    /// The path instruction.
    pub d: ImmutStr,
    /// The path style.
    /// See [`PathStyle`] for more information.
    pub styles: Vec<PathStyle>,
}

/// Attributes that is applicable to the [`PathItem`].
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "rkyv", derive(Archive, rDeser, rSer))]
#[cfg_attr(feature = "rkyv-validation", archive(check_bytes))]
pub enum PathStyle {
    /// `fill` attribute.
    /// See <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/fill>
    Fill(ImmutStr),

    /// `stroke` attribute.
    /// See <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/stroke>
    Stroke(ImmutStr),

    /// `stroke-linecap` attribute.
    /// See <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/stroke-linecap>
    StrokeLineCap(ImmutStr),

    /// `stroke-linejoin` attribute.
    /// See <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/stroke-linejoin>
    StrokeLineJoin(ImmutStr),

    /// `stroke-miterlimit` attribute.
    /// See <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/stroke-miterlimit>
    StrokeMitterLimit(Scalar),

    /// `stroke-dashoffset` attribute.
    /// See <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/stroke-dashoffset>
    StrokeDashOffset(Abs),

    /// `stroke-dasharray` attribute.
    /// See <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/stroke-dasharray>
    StrokeDashArray(Arc<[Abs]>),

    /// `stroke-width` attribute.
    /// See <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/stroke-width>
    StrokeWidth(Abs),
}

/// Item representing an `<g><text/><g/>` element.
#[derive(Debug, Clone)]
pub struct TextItem {
    /// The content of the text item.
    pub content: Arc<TextItemContent>,
    /// The shape of the text item.
    /// See [`TextShape`] for more information.
    pub shape: Arc<TextShape>,
}

/// The content metadata of a [`TextItem`].
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct TextItemContent {
    /// The plain utf-8 content of the text item.
    /// Note: witout XML escaping.
    pub content: ImmutStr,
    /// The glyphs in the text.
    /// (offset, advance, glyph): ([`Abs`], [`Abs`], [`GlyphItem`])
    pub glyphs: Vec<(Abs, Abs, GlyphItem)>,
}

/// A glyph item.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "rkyv", derive(Archive, rDeser, rSer))]
#[cfg_attr(feature = "rkyv-validation", archive(check_bytes))]
pub struct ImageGlyphItem {
    pub ts: Transform,
    pub image: ImageItem,
}

/// A glyph item.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "rkyv", derive(Archive, rDeser, rSer))]
#[cfg_attr(feature = "rkyv-validation", archive(check_bytes))]
pub struct OutlineGlyphItem {
    pub ts: Option<Transform>,
    pub d: ImmutStr,
}

/// A glyph item.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum GlyphItem {
    // Failed,
    Raw(Font, GlyphId),
    Image(Arc<ImageGlyphItem>),
    Outline(Arc<OutlineGlyphItem>),
}

/// The shape metadata of a [`TextItem`].
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "rkyv", derive(Archive, rDeser, rSer))]
#[cfg_attr(feature = "rkyv-validation", archive(check_bytes))]
pub struct TextShape {
    // todo: save direction
    // pub dir: Dir,
    /// The ascent of the font used by the text item.
    pub ascender: Abs,
    /// The units per em of the font used by the text item.
    pub upem: Scalar,
    /// The pixels per em of the font used by the text item.
    pub ppem: Scalar,
    /// Fill font text with css color.
    pub fill: ImmutStr,
}

/// Item representing an `<g/>` element applied with a [`TransformItem`].
#[derive(Debug, Clone)]
pub struct TransformedItem(pub TransformItem, pub Box<SvgItem>);

/// Absolute positioning items at their corresponding points.
#[derive(Debug, Clone)]
pub struct GroupItem(pub Vec<(Point, SvgItem)>);

/// Item representing all the transform that is applicable to a [`SvgItem`].
/// See <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/transform>
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "rkyv", derive(Archive, rDeser, rSer))]
#[cfg_attr(feature = "rkyv-validation", archive(check_bytes))]
pub enum TransformItem {
    /// `matrix` transform.
    Matrix(Arc<Transform>),
    /// `translate` transform.
    Translate(Arc<Axes<Abs>>),
    /// `scale` transform.
    Scale(Arc<(Ratio, Ratio)>),
    /// `rotate` transform.
    Rotate(Arc<Scalar>),
    /// `skewX skewY` transform.
    Skew(Arc<(Ratio, Ratio)>),

    /// clip path.
    Clip(Arc<PathItem>),
}

/// Global style namespace.
#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u32)]
pub enum StyleNs {
    /// style that contains a single css rule: `fill: #color`.
    Fill,
}

pub type GlyphMapping = HashMap<GlyphItem, AbsoulteRef>;

/// A finished pack that stores all the glyph items.
pub type GlyphPack = Vec<(AbsoulteRef, GlyphItem)>;

/// Intermediate representation of an incompleted glyph pack.
#[derive(Default)]
pub struct GlyphPackBuilder {
    pub glyphs: GlyphMapping,

    fingerprint_builder: FingerprintBuilder,
}

impl GlyphPackBuilder {
    pub fn finalize_ref(&self) -> (GlyphPack, GlyphMapping) {
        let mut glyphs = self.glyphs.clone().into_iter().collect::<Vec<_>>();
        glyphs.sort_by(|(_, a), (_, b)| a.id.0.cmp(&b.id.0));
        (glyphs.into_iter().map(|(a, b)| (b, a)).collect(), self.glyphs.clone())
    }

    pub fn finalize(self) -> (GlyphPack, GlyphMapping) {
        let mut glyphs = self.glyphs.clone().into_iter().collect::<Vec<_>>();
        glyphs.sort_by(|(_, a), (_, b)| a.id.0.cmp(&b.id.0));
        (glyphs.into_iter().map(|(a, b)| (b, a)).collect(), self.glyphs)
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
}
