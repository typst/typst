use std::fmt::Display;
use std::str::FromStr;

use ecow::EcoString;
use indexmap::IndexMap;
use rustc_hash::FxBuildHasher;
use tiny_skia as sk;
use typst::diag::{At, SourceResult, StrResult, bail};
use typst::layout::{Abs, Frame, FrameItem, PagedDocument, Transform};
use typst::visualize::Color;
use typst_html::HtmlDocument;
use typst_pdf::{PdfOptions, PdfStandard, PdfStandards};
use typst_syntax::Span;

use crate::collect::{Test, TestOutput};
use crate::pdftags;

/// Result of comparing two images.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MatchResult {
    /// Images match exactly (within ±1 per channel tolerance).
    Exact,
    /// Images match after normalized comparison (within ±3 per channel, with error budget).
    Normalized,
    /// Images match perceptually (SSIM >= threshold). Contains the SSIM score.
    Perceptual(f64),
    /// Images do not match. Contains the SSIM score if computed.
    Mismatch(Option<f64>),
}

impl MatchResult {
    /// Whether the comparison passed (any match type).
    pub fn is_match(self) -> bool {
        !matches!(self, MatchResult::Mismatch(_))
    }

    /// Whether this was a perceptual-only match (requires warning).
    pub fn is_perceptual_only(self) -> bool {
        matches!(self, MatchResult::Perceptual(_))
    }

    /// Get the SSIM score if this was a perceptual comparison.
    pub fn ssim_score(self) -> Option<f64> {
        match self {
            MatchResult::Perceptual(score) => Some(score),
            MatchResult::Mismatch(score) => score,
            _ => None,
        }
    }
}

/// A map from a test name to the corresponding reference hash.
#[derive(Default)]
pub struct HashedRefs {
    /// Whether a reference hash has been added/removed/updated and the hash
    /// file on disk should be updated.
    pub changed: bool,
    refs: IndexMap<EcoString, HashedRef, FxBuildHasher>,
}

impl HashedRefs {
    /// Get the reference hash for a test.
    pub fn get(&self, name: &str) -> Option<HashedRef> {
        self.refs.get(name).copied()
    }

    /// Whether the map is empty.
    pub fn is_empty(&self) -> bool {
        self.refs.is_empty()
    }

    /// Remove a reference hash.
    pub fn remove(&mut self, name: &str) {
        self.changed = true;
        self.refs.shift_remove(name);
    }

    /// Update a reference hash.
    pub fn update(&mut self, name: EcoString, hashed_ref: HashedRef) {
        self.changed = true;
        self.refs.insert(name, hashed_ref);
    }

    /// Sort the reference hashes lexicographically.
    pub fn sort(&mut self) {
        if !self.refs.keys().is_sorted() {
            self.changed = true;
            self.refs.sort_keys();
        }
    }

    /// The names of all tests in this map.
    pub fn names(&self) -> impl Iterator<Item = &EcoString> {
        self.refs.keys()
    }
}

impl Display for HashedRefs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (name, hash) in self.refs.iter() {
            writeln!(f, "{hash} {name}")?;
        }
        Ok(())
    }
}

impl FromStr for HashedRefs {
    type Err = EcoString;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let refs = s
            .lines()
            .map(|line| {
                let mut parts = line.split_whitespace();
                let Some(hash) = parts.next() else { bail!("found empty line") };
                let hash = hash.parse()?;

                let Some(name) = parts.next() else { bail!("missing test name") };

                if parts.next().is_some() {
                    bail!("found trailing characters");
                }

                Ok((name.into(), hash))
            })
            .collect::<StrResult<IndexMap<_, _, _>>>()?;
        Ok(HashedRefs { changed: false, refs })
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct HashedRef(u128);

impl Display for HashedRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:032x}", self.0)
    }
}

impl FromStr for HashedRef {
    type Err = EcoString;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() != 32 {
            bail!("invalid hash: hexadecimal length must be 32");
        }
        if !s.chars().all(|c| c.is_ascii_hexdigit()) {
            bail!("invalid hash: not a valid hexadecimal digit");
        }
        let val = u128::from_str_radix(s, 16).unwrap();
        Ok(HashedRef(val))
    }
}

/// An output type we can test.
pub trait OutputType: Sized {
    /// The document type this output requires.
    type Doc;
    /// The type that represents live output.
    type Live;

    /// The test output format.
    const OUTPUT: TestOutput;

    /// Whether the test output is trivial and needs no reference output.
    fn is_skippable(_doc: &Self::Doc, _live: &Self::Live) -> Result<bool, ()> {
        Ok(false)
    }

    /// Produces the live output.
    fn make_live(test: &Test, doc: &Self::Doc) -> SourceResult<Self::Live>;

    /// Converts the live output to bytes that can be saved to disk.
    fn save_live(doc: &Self::Doc, live: &Self::Live) -> impl AsRef<[u8]>;
}

/// An output type that produces file references.
pub trait FileOutputType: OutputType {
    /// Produces the reference output from the live output.
    fn make_ref(live: &Self::Live) -> impl AsRef<[u8]>;

    /// Checks whether the reference output matches.
    /// Returns a `MatchResult` indicating the type of match (or mismatch).
    fn matches(old: &[u8], new: &Self::Live) -> MatchResult;
}

/// An output type that produces hashed references.
pub trait HashOutputType: OutputType {
    /// The index into the [`crate::run::HASHES`] array.
    const INDEX: usize;

    /// Produces the reference output from the live output.
    fn make_hash(live: &Self::Live) -> HashedRef;
}

pub struct Render;

impl OutputType for Render {
    type Doc = PagedDocument;
    type Live = tiny_skia::Pixmap;

    const OUTPUT: TestOutput = TestOutput::Render;

    fn is_skippable(doc: &Self::Doc, _: &Self::Live) -> Result<bool, ()> {
        is_empty_paged_document(doc)
    }

    fn make_live(_: &Test, doc: &Self::Doc) -> SourceResult<Self::Live> {
        Ok(render(doc, 1.0))
    }

    fn save_live(doc: &Self::Doc, live: &Self::Live) -> impl AsRef<[u8]> {
        // Save live version, possibly rerendering if different scale is
        // requested.
        let mut pixmap_live = live;
        let slot;
        let scale = crate::ARGS.scale;
        if scale != 1.0 {
            slot = render(doc, scale);
            pixmap_live = &slot;
        }
        pixmap_live.encode_png().unwrap()
    }
}

impl FileOutputType for Render {
    fn make_ref(live: &Self::Live) -> impl AsRef<[u8]> {
        let opts = oxipng::Options::max_compression();
        let data = live.encode_png().unwrap();
        oxipng::optimize_from_memory(&data, &opts).unwrap()
    }

    fn matches(old: &[u8], new: &Self::Live) -> MatchResult {
        let old_pixmap = sk::Pixmap::decode_png(old).unwrap();
        compare_pixmaps(&old_pixmap, new)
    }
}

pub struct Pdf;

impl OutputType for Pdf {
    type Doc = PagedDocument;
    type Live = Vec<u8>;

    const OUTPUT: TestOutput = TestOutput::Pdf;

    fn make_live(test: &Test, doc: &Self::Doc) -> SourceResult<Self::Live> {
        // Always run the default PDF export and PDF/UA-1 export, to detect
        // crashes, since there are quite a few different code paths involved.
        // If another standard is specified in the test, run that as well.
        let default_pdf = generate_pdf(doc, None);
        let ua1_pdf = generate_pdf(doc, Some(PdfStandard::Ua_1));
        match test.attrs.pdf_standard {
            Some(PdfStandard::Ua_1) => ua1_pdf,
            Some(other) => generate_pdf(doc, Some(other)),
            None => default_pdf,
        }
    }

    fn save_live(_: &Self::Doc, live: &Self::Live) -> impl AsRef<[u8]> {
        live
    }
}

impl HashOutputType for Pdf {
    const INDEX: usize = 0;

    fn make_hash(live: &Self::Live) -> HashedRef {
        HashedRef(typst_utils::hash128(live))
    }
}

fn generate_pdf(
    doc: &PagedDocument,
    standard: Option<PdfStandard>,
) -> SourceResult<Vec<u8>> {
    let standards = PdfStandards::new(standard.as_slice()).unwrap();
    let options = PdfOptions { standards, ..Default::default() };
    typst_pdf::pdf(doc, &options)
}

pub struct Pdftags;

impl OutputType for Pdftags {
    type Doc = Vec<u8>;
    type Live = String;

    const OUTPUT: TestOutput = TestOutput::Pdftags;

    fn is_skippable(_: &Self::Doc, live: &Self::Live) -> Result<bool, ()> {
        Ok(live.is_empty())
    }

    fn make_live(_: &Test, doc: &Self::Doc) -> SourceResult<Self::Live> {
        pdftags::format(doc).at(Span::detached())
    }

    fn save_live(_: &Self::Doc, live: &Self::Live) -> impl AsRef<[u8]> {
        live
    }
}

impl FileOutputType for Pdftags {
    fn make_ref(live: &Self::Live) -> impl AsRef<[u8]> {
        live
    }

    fn matches(old: &[u8], new: &Self::Live) -> MatchResult {
        if old == new.as_bytes() {
            MatchResult::Exact
        } else {
            MatchResult::Mismatch(None)
        }
    }
}

pub struct Svg;

impl OutputType for Svg {
    type Doc = PagedDocument;
    type Live = String;

    const OUTPUT: TestOutput = TestOutput::Svg;

    fn is_skippable(_: &Self::Doc, live: &Self::Live) -> Result<bool, ()> {
        Ok(live.is_empty())
    }

    fn make_live(_: &Test, doc: &Self::Doc) -> SourceResult<Self::Live> {
        Ok(typst_svg::svg_merged(doc, Abs::pt(1.0)))
    }

    fn save_live(_: &Self::Doc, live: &Self::Live) -> impl AsRef<[u8]> {
        live
    }
}

impl HashOutputType for Svg {
    const INDEX: usize = 1;

    fn make_hash(live: &Self::Live) -> HashedRef {
        HashedRef(typst_utils::hash128(live))
    }
}

pub struct Html;

impl OutputType for Html {
    type Doc = HtmlDocument;
    type Live = String;

    const OUTPUT: TestOutput = TestOutput::Html;

    fn is_skippable(_: &Self::Doc, live: &Self::Live) -> Result<bool, ()> {
        Ok(live.is_empty())
    }

    fn make_live(_: &Test, doc: &Self::Doc) -> SourceResult<Self::Live> {
        typst_html::html(doc)
    }

    fn save_live(_: &Self::Doc, live: &Self::Live) -> impl AsRef<[u8]> {
        live
    }
}

impl FileOutputType for Html {
    fn make_ref(live: &Self::Live) -> impl AsRef<[u8]> {
        live
    }

    fn matches(old: &[u8], new: &Self::Live) -> MatchResult {
        if old == new.as_bytes() {
            MatchResult::Exact
        } else {
            MatchResult::Mismatch(None)
        }
    }
}

/// Whether rendering of this document can be skipped.
fn is_empty_paged_document(doc: &PagedDocument) -> Result<bool, ()> {
    fn skippable_frame(frame: &Frame) -> bool {
        frame.items().all(|(_, item)| match item {
            FrameItem::Group(group) => skippable_frame(&group.frame),
            FrameItem::Tag(_) => true,
            _ => false,
        })
    }

    match doc.pages.as_slice() {
        [] => Err(()),
        [page] => Ok(page.frame.width().approx_eq(Abs::pt(120.0))
            && page.frame.height().approx_eq(Abs::pt(20.0))
            && page.fill.is_auto()
            && skippable_frame(&page.frame)),
        _ => Ok(false),
    }
}

/// Draw all frames into one image with padding in between.
fn render(document: &PagedDocument, pixel_per_pt: f32) -> sk::Pixmap {
    for page in &document.pages {
        let limit = Abs::cm(100.0);
        if page.frame.width() > limit || page.frame.height() > limit {
            panic!("overlarge frame: {:?}", page.frame.size());
        }
    }

    let gap = Abs::pt(1.0);
    let mut pixmap =
        typst_render::render_merged(document, pixel_per_pt, gap, Some(Color::BLACK));

    let gap = (pixel_per_pt * gap.to_pt() as f32).round();

    let mut y = 0.0;
    for page in &document.pages {
        let ts =
            sk::Transform::from_scale(pixel_per_pt, pixel_per_pt).post_translate(0.0, y);
        render_links(&mut pixmap, ts, &page.frame);
        y += (pixel_per_pt * page.frame.height().to_pt() as f32).round().max(1.0) + gap;
    }

    pixmap
}

/// Draw extra boxes for links so we can see whether they are there.
fn render_links(canvas: &mut sk::Pixmap, ts: sk::Transform, frame: &Frame) {
    for (pos, item) in frame.items() {
        let ts = ts.pre_translate(pos.x.to_pt() as f32, pos.y.to_pt() as f32);
        match *item {
            FrameItem::Group(ref group) => {
                let ts = ts.pre_concat(to_sk_transform(&group.transform));
                render_links(canvas, ts, &group.frame);
            }
            FrameItem::Link(_, size) => {
                let w = size.x.to_pt() as f32;
                let h = size.y.to_pt() as f32;
                let rect = sk::Rect::from_xywh(0.0, 0.0, w, h).unwrap();
                let mut paint = sk::Paint::default();
                paint.set_color_rgba8(40, 54, 99, 40);
                canvas.fill_rect(rect, &paint, ts, None);
            }
            _ => {}
        }
    }
}

/// Compare two pixmaps using tiered comparison:
/// 1. Exact: within ±1 per channel (original behavior)
/// 2. Normalized: within ±3 per channel with total error budget
/// 3. Perceptual: SSIM >= 0.99
fn compare_pixmaps(a: &sk::Pixmap, b: &sk::Pixmap) -> MatchResult {
    // Dimensions must always match
    if a.width() != b.width() || a.height() != b.height() {
        return MatchResult::Mismatch(None);
    }

    // Try exact match first (±1 tolerance per channel)
    if a.data().iter().zip(b.data()).all(|(&a, &b)| a.abs_diff(b) <= 1) {
        return MatchResult::Exact;
    }

    // Try normalized match (±3 tolerance per channel, with error budget)
    // Allow up to 0.1% of pixels to exceed the tolerance
    let total_pixels = (a.width() * a.height()) as usize;
    let max_outliers = (total_pixels / 1000).max(1); // 0.1% or at least 1

    let mut outliers = 0;
    let normalized_ok = a.data().iter().zip(b.data()).all(|(&a, &b)| {
        let diff = a.abs_diff(b);
        if diff <= 3 {
            true
        } else {
            outliers += 1;
            outliers <= max_outliers
        }
    });

    if normalized_ok {
        return MatchResult::Normalized;
    }

    // Try perceptual match using SSIM
    let ssim = compute_ssim(a, b);
    if ssim >= 0.99 {
        return MatchResult::Perceptual(ssim);
    }

    MatchResult::Mismatch(Some(ssim))
}

/// Compute Structural Similarity Index (SSIM) between two pixmaps.
/// Returns a value between 0.0 (completely different) and 1.0 (identical).
/// Uses the luminance channel only for efficiency.
fn compute_ssim(a: &sk::Pixmap, b: &sk::Pixmap) -> f64 {
    let width = a.width() as usize;
    let height = a.height() as usize;

    if width == 0 || height == 0 {
        return 1.0; // Empty images are considered identical
    }

    // Convert to luminance (grayscale)
    let luma_a = to_luminance(a);
    let luma_b = to_luminance(b);

    // SSIM constants (as per the original paper)
    const K1: f64 = 0.01;
    const K2: f64 = 0.03;
    const L: f64 = 255.0; // Dynamic range
    let c1 = (K1 * L) * (K1 * L);
    let c2 = (K2 * L) * (K2 * L);

    // Use 8x8 windows for efficiency (instead of Gaussian blur)
    let window_size = 8;
    let mut ssim_sum = 0.0;
    let mut window_count = 0;

    let step = window_size; // Non-overlapping windows for speed
    let mut y = 0;
    while y + window_size <= height {
        let mut x = 0;
        while x + window_size <= width {
            let (mean_a, mean_b, var_a, var_b, cov_ab) =
                window_stats(&luma_a, &luma_b, width, x, y, window_size);

            // SSIM formula
            let numerator = (2.0 * mean_a * mean_b + c1) * (2.0 * cov_ab + c2);
            let denominator =
                (mean_a * mean_a + mean_b * mean_b + c1) * (var_a + var_b + c2);

            ssim_sum += numerator / denominator;
            window_count += 1;

            x += step;
        }
        y += step;
    }

    if window_count == 0 {
        // Image smaller than window, fall back to simple comparison
        let (mean_a, mean_b, var_a, var_b, cov_ab) =
            window_stats(&luma_a, &luma_b, width, 0, 0, width.min(height));

        let numerator = (2.0 * mean_a * mean_b + c1) * (2.0 * cov_ab + c2);
        let denominator = (mean_a * mean_a + mean_b * mean_b + c1) * (var_a + var_b + c2);

        return numerator / denominator;
    }

    ssim_sum / window_count as f64
}

/// Convert RGBA pixmap to luminance values.
fn to_luminance(pixmap: &sk::Pixmap) -> Vec<f64> {
    pixmap
        .data()
        .chunks_exact(4)
        .map(|rgba| {
            // Standard luminance coefficients (BT.709)
            0.2126 * rgba[0] as f64 + 0.7152 * rgba[1] as f64 + 0.0722 * rgba[2] as f64
        })
        .collect()
}

/// Compute mean, variance, and covariance for a window.
fn window_stats(
    a: &[f64],
    b: &[f64],
    width: usize,
    start_x: usize,
    start_y: usize,
    size: usize,
) -> (f64, f64, f64, f64, f64) {
    let mut sum_a = 0.0;
    let mut sum_b = 0.0;
    let mut sum_a2 = 0.0;
    let mut sum_b2 = 0.0;
    let mut sum_ab = 0.0;
    let mut count = 0;

    for dy in 0..size {
        for dx in 0..size {
            let idx = (start_y + dy) * width + (start_x + dx);
            if idx >= a.len() {
                continue;
            }

            let va = a[idx];
            let vb = b[idx];
            sum_a += va;
            sum_b += vb;
            sum_a2 += va * va;
            sum_b2 += vb * vb;
            sum_ab += va * vb;
            count += 1;
        }
    }

    if count == 0 {
        return (0.0, 0.0, 0.0, 0.0, 0.0);
    }

    let n = count as f64;
    let mean_a = sum_a / n;
    let mean_b = sum_b / n;
    let var_a = (sum_a2 / n) - (mean_a * mean_a);
    let var_b = (sum_b2 / n) - (mean_b * mean_b);
    let cov_ab = (sum_ab / n) - (mean_a * mean_b);

    (mean_a, mean_b, var_a.max(0.0), var_b.max(0.0), cov_ab)
}

/// Convert a Typst transform to a tiny-skia transform.
fn to_sk_transform(transform: &Transform) -> sk::Transform {
    let Transform { sx, ky, kx, sy, tx, ty } = *transform;
    sk::Transform::from_row(
        sx.get() as _,
        ky.get() as _,
        kx.get() as _,
        sy.get() as _,
        tx.to_pt() as f32,
        ty.to_pt() as f32,
    )
}
