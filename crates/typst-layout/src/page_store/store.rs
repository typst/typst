//! Disk-backed page store: serializes pages to a temp file and reads
//! them back one at a time during PDF export.

use std::io::{self, BufReader, BufWriter, Read, Seek, Write};

use typst_library::foundations::{Content, Label, Smart};
use typst_library::introspection::{Location, Tag, TagFlags};
use typst_library::layout::*;
use typst_library::model::{Destination, Numbering};
use typst_library::text::{Glyph, Lang, Region, TextItem};
use typst_library::visualize::*;
use typst_utils::PicoStr;

use super::registry::{FontRegistry, ImageRegistry};
use super::types::*;
use crate::Page;

/// A disk-backed store for document pages.
///
/// Serializes page frames to a temporary file, keeping only lightweight
/// metadata (fonts, images, tags, numberings) in memory. Pages can be
/// read back one at a time for streaming export.
pub struct DiskPageStore {
    /// Temp file holding serialized page data.
    file: tempfile::NamedTempFile,
    /// Number of pages stored.
    page_count: usize,
    /// Byte offsets of each page in the file (for random access).
    offsets: Vec<u64>,
    /// Font registry for resolving font references.
    pub fonts: FontRegistry,
    /// Image registry for resolving image references.
    pub images: ImageRegistry,
    /// Tag content objects (Content can't be serialized).
    tags: Vec<Content>,
    /// Gradient objects (contain Arc, can't be serialized).
    gradients: Vec<Gradient>,
    /// Tiling objects (contain Frame, can't be serialized).
    tilings: Vec<Tiling>,
    /// Numbering objects (contain Func, can't be serialized).
    numberings: Vec<Numbering>,
    /// Page supplement Content objects.
    supplements: Vec<Content>,
}

impl DiskPageStore {
    /// Creates a new empty store backed by a temporary file.
    /// Pages can be appended one at a time via `append_page()`.
    pub fn new() -> io::Result<Self> {
        let file = tempfile::NamedTempFile::new()?;
        Ok(DiskPageStore {
            file,
            page_count: 0,
            offsets: Vec::new(),
            fonts: FontRegistry::new(),
            images: ImageRegistry::new(),
            tags: Vec::new(),
            gradients: Vec::new(),
            tilings: Vec::new(),
            numberings: Vec::new(),
            supplements: Vec::new(),
        })
    }

    /// Creates a new store and serializes all pages to disk.
    /// After this call, the pages can be dropped from memory.
    pub fn from_pages(pages: &[Page]) -> io::Result<Self> {
        let file = tempfile::NamedTempFile::new()?;
        let mut writer = BufWriter::new(file.reopen()?);
        let mut store = DiskPageStore {
            file,
            page_count: pages.len(),
            offsets: Vec::with_capacity(pages.len()),
            fonts: FontRegistry::new(),
            images: ImageRegistry::new(),
            tags: Vec::new(),
            gradients: Vec::new(),
            tilings: Vec::new(),
            numberings: Vec::new(),
            supplements: Vec::new(),
        };

        let mut offset: u64 = 0;
        for page in pages {
            store.offsets.push(offset);
            let spage = store.convert_page(page);
            let bytes = bincode::serialize(&spage)
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
            let len = bytes.len() as u64;
            writer.write_all(&len.to_le_bytes())?;
            writer.write_all(&bytes)?;
            offset += 8 + len;
        }
        writer.flush()?;

        Ok(store)
    }

    /// Appends a single page to the store.
    pub fn append_page(&mut self, page: &Page) -> io::Result<()> {
        let mut file = self.file.reopen()?;
        file.seek(io::SeekFrom::End(0))?;
        let file_len = file.stream_position()?;
        self.offsets.push(file_len);

        let mut writer = BufWriter::new(file);
        let spage = self.convert_page(page);
        let bytes = bincode::serialize(&spage)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        let len = bytes.len() as u64;
        writer.write_all(&len.to_le_bytes())?;
        writer.write_all(&bytes)?;
        writer.flush()?;

        self.page_count += 1;
        Ok(())
    }

    /// Returns the number of pages in the store.
    pub fn page_count(&self) -> usize {
        self.page_count
    }

    /// Reads a single page back from disk and reconstructs it.
    pub fn read_page(&self, index: usize) -> io::Result<Page> {
        if index >= self.page_count {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "page index out of range"));
        }

        let mut reader = BufReader::new(self.file.reopen()?);
        let offset = self.offsets[index];

        // Seek to the page's offset
        io::copy(&mut reader.by_ref().take(offset), &mut io::sink())?;

        // Read length prefix
        let mut len_bytes = [0u8; 8];
        reader.read_exact(&mut len_bytes)?;
        let len = u64::from_le_bytes(len_bytes) as usize;

        // Read serialized page
        let mut buf = vec![0u8; len];
        reader.read_exact(&mut buf)?;

        let spage: SPage = bincode::deserialize(&buf)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        Ok(self.reconstruct_page(spage))
    }

    /// Returns an iterator that reads pages sequentially from disk.
    /// Uses a single buffered reader for efficient sequential access.
    pub fn pages_iter(&self) -> io::Result<SequentialPageIterator<'_>> {
        let reader = io::BufReader::new(self.file.reopen()?);
        Ok(SequentialPageIterator { store: self, reader, index: 0 })
    }

    // --- Conversion: Page → SPage ---

    fn convert_page(&mut self, page: &Page) -> SPage {
        let frame = self.convert_frame(&page.frame);

        let fill = match &page.fill {
            Smart::Auto => None,
            Smart::Custom(None) => Some(None),
            Smart::Custom(Some(paint)) => Some(Some(self.convert_paint(paint))),
        };

        let numbering_ref = page.numbering.as_ref().map(|n| {
            let id = self.numberings.len() as u32;
            self.numberings.push(n.clone());
            id
        });

        let supplement_ref = self.supplements.len() as u32;
        self.supplements.push(page.supplement.clone());

        SPage {
            frame,
            fill,
            numbering_ref,
            supplement_ref,
            number: page.number,
        }
    }

    fn convert_frame(&mut self, frame: &Frame) -> SFrame {
        let items: Vec<(SPoint, SFrameItem)> = frame
            .items()
            .map(|(pos, item)| {
                let spos = SPoint { x: pos.x.to_raw(), y: pos.y.to_raw() };
                let sitem = self.convert_frame_item(item);
                (spos, sitem)
            })
            .collect();

        SFrame {
            width: frame.width().to_raw(),
            height: frame.height().to_raw(),
            baseline: if frame.has_baseline() { Some(frame.baseline().to_raw()) } else { None },
            kind: match frame.kind() {
                FrameKind::Soft => SFrameKind::Soft,
                FrameKind::Hard => SFrameKind::Hard,
            },
            items,
        }
    }

    fn convert_frame_item(&mut self, item: &FrameItem) -> SFrameItem {
        match item {
            FrameItem::Group(g) => SFrameItem::Group(self.convert_group(g)),
            FrameItem::Text(t) => SFrameItem::Text(self.convert_text(t)),
            FrameItem::Shape(s, span) => {
                SFrameItem::Shape(self.convert_shape(s), span.into_raw().get())
            }
            FrameItem::Image(img, size, span) => {
                let hash = self.images.register(img);
                SFrameItem::Image(
                    SImageRef { data_hash: hash },
                    size.x.to_raw(),
                    size.y.to_raw(),
                    span.into_raw().get(),
                )
            }
            FrameItem::Link(dest, size) => SFrameItem::Link(
                self.convert_destination(dest),
                size.x.to_raw(),
                size.y.to_raw(),
            ),
            FrameItem::Tag(tag) => SFrameItem::Tag(self.convert_tag(tag)),
        }
    }

    fn convert_group(&mut self, g: &GroupItem) -> SGroupItem {
        SGroupItem {
            frame: self.convert_frame(&g.frame),
            transform: STransform {
                sx: g.transform.sx.get(),
                ky: g.transform.ky.get(),
                kx: g.transform.kx.get(),
                sy: g.transform.sy.get(),
                tx: g.transform.tx.to_raw(),
                ty: g.transform.ty.to_raw(),
            },
            clip: g.clip.as_ref().map(|c| self.convert_curve(c)),
            label: g.label.map(|l| l.resolve().as_str().to_string()),
            parent: g.parent.map(|p| SFrameParent {
                location: p.location.hash(),
                inherit: matches!(p.inherit, Inherit::Yes),
            }),
        }
    }

    fn convert_text(&mut self, t: &TextItem) -> STextItem {
        let font_ref = self.fonts.register(&t.font);
        STextItem {
            font_ref,
            size: t.size.to_raw(),
            fill: self.convert_paint(&t.fill),
            stroke: t.stroke.as_ref().map(|s| self.convert_fixed_stroke(s)),
            lang: {
                let s = t.lang.as_str();
                let bytes = s.as_bytes();
                let mut arr = [0u8; 4];
                let len = bytes.len().min(3);
                arr[..len].copy_from_slice(&bytes[..len]);
                arr[3] = len as u8;
                arr
            },
            region: t.region.map(|r| {
                let s = r.as_str();
                let bytes = s.as_bytes();
                let mut arr = [0u8; 2];
                let len = bytes.len().min(2);
                arr[..len].copy_from_slice(&bytes[..len]);
                arr
            }),
            text: t.text.to_string(),
            glyphs: t.glyphs.iter().map(|g| SGlyph {
                id: g.id,
                x_advance: g.x_advance.get(),
                x_offset: g.x_offset.get(),
                y_advance: g.y_advance.get(),
                y_offset: g.y_offset.get(),
                range_start: g.range.start,
                range_end: g.range.end,
                span: g.span.0.into_raw().get(),
                span_offset: g.span.1,
            }).collect(),
        }
    }

    fn convert_shape(&mut self, s: &Shape) -> SShape {
        SShape {
            geometry: match &s.geometry {
                Geometry::Line(p) => SGeometry::Line(SPoint { x: p.x.to_raw(), y: p.y.to_raw() }),
                Geometry::Rect(sz) => SGeometry::Rect(sz.x.to_raw(), sz.y.to_raw()),
                Geometry::Curve(c) => SGeometry::Curve(self.convert_curve(c)),
            },
            fill: s.fill.as_ref().map(|p| self.convert_paint(p)),
            fill_rule: match s.fill_rule {
                FillRule::NonZero => SFillRule::NonZero,
                FillRule::EvenOdd => SFillRule::EvenOdd,
            },
            stroke: s.stroke.as_ref().map(|st| self.convert_fixed_stroke(st)),
        }
    }

    fn convert_curve(&self, c: &Curve) -> SCurve {
        SCurve(c.0.iter().map(|item| match *item {
            CurveItem::Move(p) => SCurveItem::Move(SPoint { x: p.x.to_raw(), y: p.y.to_raw() }),
            CurveItem::Line(p) => SCurveItem::Line(SPoint { x: p.x.to_raw(), y: p.y.to_raw() }),
            CurveItem::Cubic(a, b, c) => SCurveItem::Cubic(
                SPoint { x: a.x.to_raw(), y: a.y.to_raw() },
                SPoint { x: b.x.to_raw(), y: b.y.to_raw() },
                SPoint { x: c.x.to_raw(), y: c.y.to_raw() },
            ),
            CurveItem::Close => SCurveItem::Close,
        }).collect())
    }

    fn convert_fixed_stroke(&mut self, s: &FixedStroke) -> SFixedStroke {
        SFixedStroke {
            paint: self.convert_paint(&s.paint),
            thickness: s.thickness.to_raw(),
            cap: match s.cap {
                LineCap::Butt => SLineCap::Butt,
                LineCap::Round => SLineCap::Round,
                LineCap::Square => SLineCap::Square,
            },
            join: match s.join {
                LineJoin::Miter => SLineJoin::Miter,
                LineJoin::Round => SLineJoin::Round,
                LineJoin::Bevel => SLineJoin::Bevel,
            },
            dash: s.dash.as_ref().map(|d| SDashPattern {
                array: d.array.iter().map(|l| SDashLength::Length(l.to_raw())).collect(),
                phase: d.phase.to_raw(),
            }),
            miter_limit: s.miter_limit.get(),
        }
    }

    fn convert_paint(&mut self, paint: &Paint) -> SPaint {
        match paint {
            Paint::Solid(color) => SPaint::Solid(convert_color(color)),
            Paint::Gradient(g) => {
                let id = self.gradients.len() as u32;
                self.gradients.push(g.clone());
                SPaint::GradientRef(id)
            }
            Paint::Tiling(t) => {
                let id = self.tilings.len() as u32;
                self.tilings.push(t.clone());
                SPaint::TilingRef(id)
            }
        }
    }

    fn convert_destination(&self, dest: &Destination) -> SDestination {
        match dest {
            Destination::Url(url) => SDestination::Url(url.as_str().to_string()),
            Destination::Position(pos) => SDestination::Position(SPagedPosition {
                page: pos.page.get(),
                x: pos.point.x.to_raw(),
                y: pos.point.y.to_raw(),
            }),
            Destination::Location(loc) => SDestination::Location(loc.hash()),
        }
    }

    fn convert_tag(&mut self, tag: &Tag) -> STag {
        match tag {
            Tag::Start(content, loc, flags) => {
                let id = self.tags.len() as u32;
                self.tags.push(content.clone());
                STag::Start(id, loc.hash(), STagFlags {
                    introspectable: flags.introspectable,
                    tagged: flags.tagged,
                })
            }
            Tag::End(loc, key, flags) => STag::End(
                loc.hash(),
                *key,
                STagFlags {
                    introspectable: flags.introspectable,
                    tagged: flags.tagged,
                },
            ),
        }
    }

    // --- Reconstruction: SPage → Page ---

    fn reconstruct_page(&self, spage: SPage) -> Page {
        let frame = self.reconstruct_frame(spage.frame);

        let fill = match spage.fill {
            None => Smart::Auto,
            Some(None) => Smart::Custom(None),
            Some(Some(paint)) => Smart::Custom(Some(self.reconstruct_paint(paint))),
        };

        let numbering = spage.numbering_ref.map(|id| {
            self.numberings[id as usize].clone()
        });

        let supplement = self.supplements[spage.supplement_ref as usize].clone();

        Page {
            frame,
            fill,
            numbering,
            supplement,
            number: spage.number,
        }
    }

    fn reconstruct_frame(&self, sf: SFrame) -> Frame {
        let kind = match sf.kind {
            SFrameKind::Soft => FrameKind::Soft,
            SFrameKind::Hard => FrameKind::Hard,
        };

        let size = Size::new(Abs::raw(sf.width), Abs::raw(sf.height));
        let mut frame = Frame::new(size, kind);

        if let Some(b) = sf.baseline {
            frame.set_baseline(Abs::raw(b));
        }

        let items: Vec<(Point, FrameItem)> = sf.items.into_iter().map(|(sp, si)| {
            let point = Point::new(Abs::raw(sp.x), Abs::raw(sp.y));
            let item = self.reconstruct_frame_item(si);
            (point, item)
        }).collect();

        frame.push_multiple(items);
        frame
    }

    fn reconstruct_frame_item(&self, si: SFrameItem) -> FrameItem {
        match si {
            SFrameItem::Group(g) => FrameItem::Group(self.reconstruct_group(g)),
            SFrameItem::Text(t) => FrameItem::Text(self.reconstruct_text(t)),
            SFrameItem::Shape(s, span) => {
                FrameItem::Shape(self.reconstruct_shape(s), raw_to_span(span))
            }
            SFrameItem::Image(img_ref, w, h, span) => {
                let image = self.images.resolve(img_ref.data_hash)
                    .expect("image not found in registry");
                FrameItem::Image(image, Size::new(Abs::raw(w), Abs::raw(h)), raw_to_span(span))
            }
            SFrameItem::Link(dest, w, h) => {
                FrameItem::Link(self.reconstruct_destination(dest), Size::new(Abs::raw(w), Abs::raw(h)))
            }
            SFrameItem::Tag(tag) => FrameItem::Tag(self.reconstruct_tag(tag)),
        }
    }

    fn reconstruct_group(&self, sg: SGroupItem) -> GroupItem {
        GroupItem {
            frame: self.reconstruct_frame(sg.frame),
            transform: Transform {
                sx: Ratio::new(sg.transform.sx),
                ky: Ratio::new(sg.transform.ky),
                kx: Ratio::new(sg.transform.kx),
                sy: Ratio::new(sg.transform.sy),
                tx: Abs::raw(sg.transform.tx),
                ty: Abs::raw(sg.transform.ty),
            },
            clip: sg.clip.map(|c| reconstruct_curve(c)),
            label: sg.label.and_then(|s| Label::new(PicoStr::intern(&s))),
            parent: sg.parent.map(|p| FrameParent {
                location: Location::new(p.location),
                inherit: if p.inherit { Inherit::Yes } else { Inherit::No },
            }),
        }
    }

    fn reconstruct_text(&self, st: STextItem) -> TextItem {
        let font = self.fonts.resolve(&st.font_ref)
            .expect("font not found in registry");

        TextItem {
            font,
            size: Abs::raw(st.size),
            fill: self.reconstruct_paint(st.fill),
            stroke: st.stroke.map(|s| self.reconstruct_fixed_stroke(s)),
            lang: {
                let len = st.lang[3] as usize;
                let s = std::str::from_utf8(&st.lang[..len]).unwrap_or("en");
                s.parse::<Lang>().unwrap_or(Lang::ENGLISH)
            },
            region: st.region.and_then(|r| {
                let s = std::str::from_utf8(&r).ok()?;
                s.parse::<Region>().ok()
            }),
            text: st.text.into(),
            glyphs: st.glyphs.into_iter().map(|g| Glyph {
                id: g.id,
                x_advance: Em::new(g.x_advance),
                x_offset: Em::new(g.x_offset),
                y_advance: Em::new(g.y_advance),
                y_offset: Em::new(g.y_offset),
                range: g.range_start..g.range_end,
                span: (raw_to_span(g.span), g.span_offset),
            }).collect(),
        }
    }

    fn reconstruct_shape(&self, ss: SShape) -> Shape {
        Shape {
            geometry: match ss.geometry {
                SGeometry::Line(p) => Geometry::Line(Point::new(Abs::raw(p.x), Abs::raw(p.y))),
                SGeometry::Rect(w, h) => Geometry::Rect(Size::new(Abs::raw(w), Abs::raw(h))),
                SGeometry::Curve(c) => Geometry::Curve(reconstruct_curve(c)),
            },
            fill: ss.fill.map(|p| self.reconstruct_paint(p)),
            fill_rule: match ss.fill_rule {
                SFillRule::NonZero => FillRule::NonZero,
                SFillRule::EvenOdd => FillRule::EvenOdd,
            },
            stroke: ss.stroke.map(|s| self.reconstruct_fixed_stroke(s)),
        }
    }

    fn reconstruct_fixed_stroke(&self, ss: SFixedStroke) -> FixedStroke {
        FixedStroke {
            paint: self.reconstruct_paint(ss.paint),
            thickness: Abs::raw(ss.thickness),
            cap: match ss.cap {
                SLineCap::Butt => LineCap::Butt,
                SLineCap::Round => LineCap::Round,
                SLineCap::Square => LineCap::Square,
            },
            join: match ss.join {
                SLineJoin::Miter => LineJoin::Miter,
                SLineJoin::Round => LineJoin::Round,
                SLineJoin::Bevel => LineJoin::Bevel,
            },
            dash: ss.dash.map(|d| DashPattern {
                array: d.array.into_iter().map(|dl| match dl {
                    SDashLength::LineWidth => Abs::zero(),
                    SDashLength::Length(l) => Abs::raw(l),
                }).collect(),
                phase: Abs::raw(d.phase),
            }),
            miter_limit: Ratio::new(ss.miter_limit),
        }
    }

    fn reconstruct_paint(&self, sp: SPaint) -> Paint {
        match sp {
            SPaint::Solid(c) => Paint::Solid(reconstruct_color(c)),
            SPaint::GradientRef(id) => Paint::Gradient(self.gradients[id as usize].clone()),
            SPaint::TilingRef(id) => Paint::Tiling(self.tilings[id as usize].clone()),
        }
    }

    fn reconstruct_destination(&self, sd: SDestination) -> Destination {
        match sd {
            SDestination::Url(url) => {
                Destination::Url(typst_library::model::Url::new(&url).unwrap_or_else(|_| {
                    typst_library::model::Url::new("about:blank").unwrap()
                }))
            }
            SDestination::Position(pos) => Destination::Position(
                typst_library::introspection::PagedPosition {
                    page: std::num::NonZeroUsize::new(pos.page).unwrap_or(std::num::NonZeroUsize::MIN),
                    point: Point::new(Abs::raw(pos.x), Abs::raw(pos.y)),
                },
            ),
            SDestination::Location(raw) => Destination::Location(Location::new(raw)),
        }
    }

    fn reconstruct_tag(&self, st: STag) -> Tag {
        match st {
            STag::Start(id, loc, flags) => {
                let content = self.tags[id as usize].clone();
                Tag::Start(content, Location::new(loc), TagFlags {
                    introspectable: flags.introspectable,
                    tagged: flags.tagged,
                })
            }
            STag::End(loc, key, flags) => Tag::End(
                Location::new(loc),
                key,
                TagFlags {
                    introspectable: flags.introspectable,
                    tagged: flags.tagged,
                },
            ),
        }
    }
}

/// Iterator that reads pages one at a time from the disk store.
pub struct PageIterator<'a> {
    store: &'a DiskPageStore,
    index: usize,
}

impl Iterator for PageIterator<'_> {
    type Item = io::Result<Page>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.store.page_count {
            return None;
        }
        let result = self.store.read_page(self.index);
        self.index += 1;
        Some(result)
    }
}

// --- Helpers ---

fn convert_color(c: &Color) -> SColor {
    // Always normalize to Oklab for serialization.
    let oklab = c.to_space(ColorSpace::Oklab);
    let [l, a, b, alpha] = oklab.to_vec4();
    SColor::Oklab(l, a, b, alpha)
}

fn reconstruct_color(sc: SColor) -> Color {
    // All colors are normalized to Oklab during serialization, so the
    // Oklab branch is the only one that should be reached in practice.
    // The other branches are provided for forward-compatibility if the
    // serialization strategy changes.
    match sc {
        SColor::Oklab(l, a, b, alpha) => Color::Oklab(Oklab::new(l, a, b, alpha)),
        SColor::Luma(v, a) => Color::Luma(Luma::new(v, a)),
        SColor::Oklch(l, c, h, alpha) => Color::Oklch(Oklch::new(l, c, h, alpha)),
        SColor::Rgb(r, g, b, a) => Color::Rgb(Rgb::new(r, g, b, a)),
        SColor::LinearRgb(r, g, b, a) => Color::LinearRgb(LinearRgb::new(r, g, b, a)),
        SColor::Cmyk(c, m, y, k) => Color::Cmyk(Cmyk { c, m, y, k }),
        // Hsl and Hsv are never produced by convert_color, but handle
        // gracefully by converting via Oklab round-trip.
        SColor::Hsl(h, s, l, a) => {
            // Approximate: treat as Oklab since we cannot construct RgbHue
            // without the palette crate as a direct dependency.
            Color::Oklab(Oklab::new(l, h, s, a))
        }
        SColor::Hsv(h, s, v, a) => {
            Color::Oklab(Oklab::new(v, h, s, a))
        }
    }
}

fn reconstruct_curve(sc: SCurve) -> Curve {
    let items: Vec<CurveItem> = sc.0.into_iter().map(|item| match item {
        SCurveItem::Move(p) => CurveItem::Move(Point::new(Abs::raw(p.x), Abs::raw(p.y))),
        SCurveItem::Line(p) => CurveItem::Line(Point::new(Abs::raw(p.x), Abs::raw(p.y))),
        SCurveItem::Cubic(a, b, c) => CurveItem::Cubic(
            Point::new(Abs::raw(a.x), Abs::raw(a.y)),
            Point::new(Abs::raw(b.x), Abs::raw(b.y)),
            Point::new(Abs::raw(c.x), Abs::raw(c.y)),
        ),
        SCurveItem::Close => CurveItem::Close,
    }).collect();
    Curve(items)
}

fn raw_to_span(raw: u64) -> typst_syntax::Span {
    typst_syntax::Span::from_raw(std::num::NonZeroU64::new(raw).unwrap_or(
        std::num::NonZeroU64::new(1).unwrap()
    ))
}

/// Sequential page iterator using a single buffered reader.
/// Much faster than random-access `read_page` for sequential reads.
pub struct SequentialPageIterator<'a> {
    store: &'a DiskPageStore,
    reader: io::BufReader<std::fs::File>,
    index: usize,
}

impl Iterator for SequentialPageIterator<'_> {
    type Item = io::Result<Page>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.store.page_count {
            return None;
        }

        let result = (|| -> io::Result<Page> {
            let mut len_bytes = [0u8; 8];
            self.reader.read_exact(&mut len_bytes)?;
            let len = u64::from_le_bytes(len_bytes) as usize;

            let mut buf = vec![0u8; len];
            self.reader.read_exact(&mut buf)?;

            let spage: SPage = bincode::deserialize(&buf)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

            Ok(self.store.reconstruct_page(spage))
        })();

        self.index += 1;
        Some(result)
    }
}
