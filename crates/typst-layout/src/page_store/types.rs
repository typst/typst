//! Serializable proxy types for Frame and its contents.
//!
//! Each type mirrors the corresponding Typst type but replaces
//! non-serializable fields (Arc<Font>, Content, etc.) with
//! serializable references (hashes, IDs).

use serde::{Deserialize, Serialize};

/// Serializable proxy for [`typst_library::layout::Frame`].
#[derive(Serialize, Deserialize)]
pub struct SFrame {
    pub width: f64,
    pub height: f64,
    pub baseline: Option<f64>,
    pub kind: SFrameKind,
    pub items: Vec<(SPoint, SFrameItem)>,
}

#[derive(Serialize, Deserialize)]
pub enum SFrameKind {
    Soft,
    Hard,
}

#[derive(Serialize, Deserialize)]
pub struct SPoint {
    pub x: f64,
    pub y: f64,
}

/// Serializable proxy for [`typst_library::layout::FrameItem`].
#[derive(Serialize, Deserialize)]
pub enum SFrameItem {
    Group(SGroupItem),
    Text(STextItem),
    Shape(SShape, u64), // Span as raw u64
    Image(SImageRef, f64, f64, u64), // width, height, span
    Link(SDestination, f64, f64), // width, height
    Tag(STag),
}

#[derive(Serialize, Deserialize)]
pub struct SGroupItem {
    pub frame: SFrame,
    pub transform: STransform,
    pub clip: Option<SCurve>,
    pub label: Option<String>,
    pub parent: Option<SFrameParent>,
}

#[derive(Serialize, Deserialize)]
pub struct SFrameParent {
    pub location: u128,
    pub inherit: bool,
}

#[derive(Serialize, Deserialize)]
pub struct STransform {
    pub sx: f64,
    pub ky: f64,
    pub kx: f64,
    pub sy: f64,
    pub tx: f64,
    pub ty: f64,
}

// --- Text ---

/// Serializable proxy for [`typst_library::text::TextItem`].
#[derive(Serialize, Deserialize)]
pub struct STextItem {
    pub font_ref: SFontRef,
    pub size: f64,
    pub fill: SPaint,
    pub stroke: Option<SFixedStroke>,
    pub lang: [u8; 4], // 3 bytes + length
    pub region: Option<[u8; 2]>,
    pub text: String,
    pub glyphs: Vec<SGlyph>,
}

/// Reference to a font by its data hash and collection index.
#[derive(Serialize, Deserialize, Clone, Hash, Eq, PartialEq)]
pub struct SFontRef {
    pub data_hash: u128,
    pub index: u32,
}

#[derive(Serialize, Deserialize)]
pub struct SGlyph {
    pub id: u16,
    pub x_advance: f64,
    pub x_offset: f64,
    pub y_advance: f64,
    pub y_offset: f64,
    pub range_start: u16,
    pub range_end: u16,
    pub span: u64,
    pub span_offset: u16,
}

// --- Shapes ---

#[derive(Serialize, Deserialize)]
pub struct SShape {
    pub geometry: SGeometry,
    pub fill: Option<SPaint>,
    pub fill_rule: SFillRule,
    pub stroke: Option<SFixedStroke>,
}

#[derive(Serialize, Deserialize)]
pub enum SGeometry {
    Line(SPoint),
    Rect(f64, f64), // width, height
    Curve(SCurve),
}

#[derive(Serialize, Deserialize)]
pub struct SCurve(pub Vec<SCurveItem>);

#[derive(Serialize, Deserialize)]
pub enum SCurveItem {
    Move(SPoint),
    Line(SPoint),
    Cubic(SPoint, SPoint, SPoint),
    Close,
}

#[derive(Serialize, Deserialize)]
pub enum SFillRule {
    NonZero,
    EvenOdd,
}

#[derive(Serialize, Deserialize)]
pub struct SFixedStroke {
    pub paint: SPaint,
    pub thickness: f64,
    pub cap: SLineCap,
    pub join: SLineJoin,
    pub dash: Option<SDashPattern>,
    pub miter_limit: f64,
}

#[derive(Serialize, Deserialize)]
pub enum SLineCap {
    Butt,
    Round,
    Square,
}

#[derive(Serialize, Deserialize)]
pub enum SLineJoin {
    Miter,
    Round,
    Bevel,
}

#[derive(Serialize, Deserialize)]
pub struct SDashPattern {
    pub array: Vec<SDashLength>,
    pub phase: f64,
}

#[derive(Serialize, Deserialize)]
pub enum SDashLength {
    LineWidth,
    Length(f64),
}

// --- Paint ---

#[derive(Serialize, Deserialize)]
pub enum SPaint {
    Solid(SColor),
    /// Gradient stored by ID into a side table (contains Arc).
    GradientRef(u32),
    /// Tiling stored by ID into a side table (contains Frame).
    TilingRef(u32),
}

#[derive(Serialize, Deserialize)]
pub enum SColor {
    Luma(f32, f32),          // value, alpha
    Oklab(f32, f32, f32, f32), // l, a, b, alpha
    Oklch(f32, f32, f32, f32), // l, c, h, alpha
    Rgb(f32, f32, f32, f32),   // r, g, b, alpha
    LinearRgb(f32, f32, f32, f32),
    Cmyk(f32, f32, f32, f32),  // c, m, y, k (no alpha in CMYK)
    Hsl(f32, f32, f32, f32),
    Hsv(f32, f32, f32, f32),
}

// --- Image ---

/// Reference to an image by its content hash.
#[derive(Serialize, Deserialize, Clone, Hash, Eq, PartialEq)]
pub struct SImageRef {
    pub data_hash: u128,
}

// --- Link ---

#[derive(Serialize, Deserialize)]
pub enum SDestination {
    Url(String),
    Position(SPagedPosition),
    Location(u128),
}

#[derive(Serialize, Deserialize)]
pub struct SPagedPosition {
    pub page: usize,
    pub x: f64,
    pub y: f64,
}

// --- Tags ---

/// Tags are stored by sequential ID in a side table since Content
/// cannot be serialized. The side table is kept in memory (small relative
/// to frame data).
#[derive(Serialize, Deserialize)]
pub enum STag {
    /// References a Content stored in the TagStore by sequential ID.
    Start(u32, u128, STagFlags), // id, location, flags
    End(u128, u128, STagFlags),  // location, key, flags
}

#[derive(Serialize, Deserialize)]
pub struct STagFlags {
    pub introspectable: bool,
    pub tagged: bool,
}

// --- Page ---

/// Serializable proxy for [`crate::Page`].
#[derive(Serialize, Deserialize)]
pub struct SPage {
    pub frame: SFrame,
    pub fill: Option<Option<SPaint>>, // None = Smart::Auto, Some(None) = transparent
    pub numbering_ref: Option<u32>,   // Reference into numbering side table
    pub supplement_ref: u32,          // Reference into content side table
    pub number: u64,
}
