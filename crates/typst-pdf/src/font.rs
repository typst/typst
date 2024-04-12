use std::collections::BTreeMap;
use std::sync::Arc;

use ecow::{eco_format, EcoString};
use pdf_writer::types::{CidFontType, FontFlags, SystemInfo, UnicodeCmap};
use pdf_writer::writers::FontDescriptor;
use pdf_writer::{Filter, Finish, Name, Rect, Str};
use ttf_parser::{name_id, GlyphId, Tag};
use typst::layout::{Abs, Ratio, Transform};
use typst::text::Font;
use typst::util::SliceExt;
use unicode_properties::{GeneralCategory, UnicodeGeneralCategory};

use crate::page::{write_frame, PageContext};
use crate::{deflate, AbsExt, EmExt, PdfContext};

const CFF: Tag = Tag::from_bytes(b"CFF ");
const CFF2: Tag = Tag::from_bytes(b"CFF2");
const CMAP_NAME: Name = Name(b"Custom");
const SYSTEM_INFO: SystemInfo = SystemInfo {
    registry: Str(b"Adobe"),
    ordering: Str(b"Identity"),
    supplement: 0,
};

/// Embed all used fonts into the PDF.
#[typst_macros::time(name = "write fonts")]
pub(crate) fn write_fonts(ctx: &mut PdfContext) {
    let emoji_font_map = ctx.color_font_map.take_map();
    for (font_info, font) in emoji_font_map {
        for (font_index, subfont_id) in font.refs.iter().enumerate() {
            let cmap_ref = ctx.alloc.bump();
            let descriptor_ref = ctx.alloc.bump();
            let mut glyphs_to_instructions = BTreeMap::new();

            let start = font_index * 256;
            let end = (start + 256).min(font.glyphs.len());
            for (cid, emoji) in font.glyphs[start..end].iter().enumerate() {
                let page_ref = ctx.alloc.bump();
                // create a fake page context for write_frame
                // we are only interested in the contents of the page
                let size = emoji.image.size();
                let mut page_ctx = PageContext::new(ctx, page_ref, size);
                page_ctx.bottom = size.y.to_f32();
                page_ctx.content.start_color_glyph(1.0);
                page_ctx.transform(Transform {
                    sx: Ratio::one(),
                    ky: Ratio::zero(),
                    kx: Ratio::zero(),
                    sy: size.aspect_ratio() * -1.0,
                    tx: Abs::zero(),
                    ty: size.y,
                });
                write_frame(&mut page_ctx, &emoji.image);
                let stream = page_ctx.content.finish();
                ctx.pdf.stream(page_ref, &stream);

                glyphs_to_instructions.insert(cid, page_ref);
            }

            let mut pdf_font = ctx.pdf.type3_font(*subfont_id);
            pdf_font.pair(Name(b"Resources"), ctx.type3_font_resources_ref);
            pdf_font.bbox(font.bbox);
            pdf_font.matrix([1.0, 0.0, 0.0, 1.0, 0.0, 0.0]);
            let mut char_procs = pdf_font.char_procs();
            for (gid, instructions_ref) in &glyphs_to_instructions {
                char_procs
                    .pair(Name(eco_format!("glyph{gid}").as_bytes()), *instructions_ref);
            }
            char_procs.finish();

            let mut encoding = pdf_font.encoding_custom();
            let mut differences = encoding.differences();
            for gid in glyphs_to_instructions.keys() {
                differences.consecutive(
                    *gid as u8,
                    vec![Name(eco_format!("glyph{gid}").as_bytes())],
                );
            }
            differences.finish();
            encoding.finish();

            let first = 0;
            let last = (end - 1 - start) as u8;
            pdf_font.first_char(first);
            pdf_font.last_char(last);
            pdf_font
                .widths(std::iter::repeat(1.0).take(last as usize - first as usize + 1));
            pdf_font.to_unicode(cmap_ref);
            pdf_font.font_descriptor(descriptor_ref);
            pdf_font.finish();

            // Encode a CMAP to make it possible to search or copy glyphs

            // To avoid maintaining a separate glyph set structure for Type3
            // fonts, a hack is used: the glyph ID is only in the
            // less-significant byte (as there can only be 256 of them per
            // font), the rest stores the Type3 font Ref.
            let full_glyph_set = ctx.glyph_sets.get_mut(&font_info).unwrap();
            // We retrieve only the part relevant for this font
            let glyph_set = full_glyph_set
                .iter()
                .filter(|(k, _)| *k / 256 == subfont_id.get() as u16)
                .map(|(k, v)| (k % 256, v));

            // And we write it
            let mut cmap = UnicodeCmap::new(CMAP_NAME, SYSTEM_INFO);
            for (g, text) in glyph_set {
                if !text.is_empty() {
                    cmap.pair_with_multiple(g, text.chars());
                }
            }
            ctx.pdf.cmap(cmap_ref, &cmap.finish());

            let postscript_name = font_info
                .find_name(name_id::POST_SCRIPT_NAME)
                .unwrap_or_else(|| "unknown".to_string());

            let base_font = eco_format!("COLOR{font_index:x}+{postscript_name}");

            let font_descriptor =
                font_descriptor(&mut ctx.pdf, descriptor_ref, &font_info, &base_font);
            font_descriptor.finish()
        }
    }

    for font in ctx.font_map.items() {
        let type0_ref = ctx.alloc.bump();
        let cid_ref = ctx.alloc.bump();
        let descriptor_ref = ctx.alloc.bump();
        let cmap_ref = ctx.alloc.bump();
        let data_ref = ctx.alloc.bump();
        ctx.font_refs.push(type0_ref);

        let glyph_set = ctx.glyph_sets.get_mut(font).unwrap();
        let ttf = font.ttf();

        // Do we have a TrueType or CFF font?
        //
        // FIXME: CFF2 must be handled differently and requires PDF 2.0
        // (or we have to convert it to CFF).
        let is_cff = ttf
            .raw_face()
            .table(CFF)
            .or_else(|| ttf.raw_face().table(CFF2))
            .is_some();

        let postscript_name = font
            .find_name(name_id::POST_SCRIPT_NAME)
            .unwrap_or_else(|| "unknown".to_string());

        let subset_tag = subset_tag(glyph_set);
        let base_font = eco_format!("{subset_tag}+{postscript_name}");
        let base_font_type0 = if is_cff {
            eco_format!("{base_font}-Identity-H")
        } else {
            base_font.clone()
        };

        // Write the base font object referencing the CID font.
        ctx.pdf
            .type0_font(type0_ref)
            .base_font(Name(base_font_type0.as_bytes()))
            .encoding_predefined(Name(b"Identity-H"))
            .descendant_font(cid_ref)
            .to_unicode(cmap_ref);

        // Write the CID font referencing the font descriptor.
        let mut cid = ctx.pdf.cid_font(cid_ref);
        cid.subtype(if is_cff { CidFontType::Type0 } else { CidFontType::Type2 });
        cid.base_font(Name(base_font.as_bytes()));
        cid.system_info(SYSTEM_INFO);
        cid.font_descriptor(descriptor_ref);
        cid.default_width(0.0);
        if !is_cff {
            cid.cid_to_gid_map_predefined(Name(b"Identity"));
        }

        // Extract the widths of all glyphs.
        let mut widths = vec![];
        for gid in std::iter::once(0).chain(glyph_set.keys().copied()) {
            let width = ttf.glyph_hor_advance(GlyphId(gid)).unwrap_or(0);
            let units = font.to_em(width).to_font_units();
            let cid = glyph_cid(font, gid);
            if usize::from(cid) >= widths.len() {
                widths.resize(usize::from(cid) + 1, 0.0);
                widths[usize::from(cid)] = units;
            }
        }

        // Write all non-zero glyph widths.
        let mut first = 0;
        let mut width_writer = cid.widths();
        for (w, group) in widths.group_by_key(|&w| w) {
            let end = first + group.len();
            if w != 0.0 {
                let last = end - 1;
                width_writer.same(first as u16, last as u16, w);
            }
            first = end;
        }

        width_writer.finish();
        cid.finish();

        // Write the /ToUnicode character map, which maps glyph ids back to
        // unicode codepoints to enable copying out of the PDF.
        let cmap = create_cmap(font, glyph_set);
        ctx.pdf.cmap(cmap_ref, &cmap.finish());

        // Subset and write the font's bytes.
        let glyphs: Vec<_> = glyph_set.keys().copied().collect();
        let data = subset_font(font, &glyphs);

        let mut stream = ctx.pdf.stream(data_ref, &data);
        stream.filter(Filter::FlateDecode);
        if is_cff {
            stream.pair(Name(b"Subtype"), Name(b"CIDFontType0C"));
        }

        stream.finish();

        let mut font_descriptor =
            font_descriptor(&mut ctx.pdf, descriptor_ref, font, &base_font);
        if is_cff {
            font_descriptor.font_file3(data_ref);
        } else {
            font_descriptor.font_file2(data_ref);
        }
        font_descriptor.finish();
    }
}

fn font_descriptor<'a>(
    pdf: &'a mut pdf_writer::Pdf,
    descriptor_ref: pdf_writer::Ref,
    font: &'a Font,
    base_font: &EcoString,
) -> FontDescriptor<'a> {
    let ttf = font.ttf();
    let metrics = font.metrics();
    let postscript_name = font
        .find_name(name_id::POST_SCRIPT_NAME)
        .unwrap_or_else(|| "unknown".to_string());

    let mut flags = FontFlags::empty();
    flags.set(FontFlags::SERIF, postscript_name.contains("Serif"));
    flags.set(FontFlags::FIXED_PITCH, ttf.is_monospaced());
    flags.set(FontFlags::ITALIC, ttf.is_italic());
    flags.insert(FontFlags::SYMBOLIC);
    flags.insert(FontFlags::SMALL_CAP);

    let global_bbox = ttf.global_bounding_box();
    let bbox = Rect::new(
        font.to_em(global_bbox.x_min).to_font_units(),
        font.to_em(global_bbox.y_min).to_font_units(),
        font.to_em(global_bbox.x_max).to_font_units(),
        font.to_em(global_bbox.y_max).to_font_units(),
    );

    let italic_angle = ttf.italic_angle().unwrap_or(0.0);
    let ascender = metrics.ascender.to_font_units();
    let descender = metrics.descender.to_font_units();
    let cap_height = metrics.cap_height.to_font_units();
    let stem_v = 10.0 + 0.244 * (f32::from(ttf.weight().to_number()) - 50.0);

    // Write the font descriptor (contains metrics about the font).
    let mut font_descriptor = pdf.font_descriptor(descriptor_ref);
    font_descriptor
        .name(Name(base_font.as_bytes()))
        .flags(flags)
        .bbox(bbox)
        .italic_angle(italic_angle)
        .ascent(ascender)
        .descent(descender)
        .cap_height(cap_height)
        .stem_v(stem_v);

    font_descriptor
}

/// Subset a font to the given glyphs.
///
/// - For a font with TrueType outlines, this returns the whole OpenType font.
/// - For a font with CFF outlines, this returns just the CFF font program.
#[comemo::memoize]
#[typst_macros::time(name = "subset font")]
fn subset_font(font: &Font, glyphs: &[u16]) -> Arc<Vec<u8>> {
    let data = font.data();
    let profile = subsetter::Profile::pdf(glyphs);
    let subsetted = subsetter::subset(data, font.index(), profile);
    let mut data = subsetted.as_deref().unwrap_or(data);

    // Extract the standalone CFF font program if applicable.
    let raw = ttf_parser::RawFace::parse(data, 0).unwrap();
    if let Some(cff) = raw.table(CFF) {
        data = cff;
    }

    Arc::new(deflate(data))
}

/// Produce a unique 6 letter tag for a glyph set.
fn subset_tag(glyphs: &BTreeMap<u16, EcoString>) -> EcoString {
    const LEN: usize = 6;
    const BASE: u128 = 26;
    let mut hash = typst::util::hash128(&glyphs);
    let mut letter = [b'A'; LEN];
    for l in letter.iter_mut() {
        *l = b'A' + (hash % BASE) as u8;
        hash /= BASE;
    }
    std::str::from_utf8(&letter).unwrap().into()
}

/// Create a /ToUnicode CMap.
fn create_cmap(font: &Font, glyph_set: &mut BTreeMap<u16, EcoString>) -> UnicodeCmap {
    let ttf = font.ttf();

    // For glyphs that have codepoints mapping to them in the font's cmap table,
    // we prefer them over pre-existing text mappings from the document. Only
    // things that don't have a corresponding codepoint (or only a private-use
    // one) like the "Th" in Linux Libertine get the text of their first
    // occurrences in the document instead.
    for subtable in ttf.tables().cmap.into_iter().flat_map(|table| table.subtables) {
        if !subtable.is_unicode() {
            continue;
        }

        subtable.codepoints(|n| {
            let Some(c) = std::char::from_u32(n) else { return };
            if c.general_category() == GeneralCategory::PrivateUse {
                return;
            }

            let Some(GlyphId(g)) = ttf.glyph_index(c) else { return };
            if glyph_set.contains_key(&g) {
                glyph_set.insert(g, c.into());
            }
        });
    }

    // Produce a reverse mapping from glyphs' CIDs to unicode strings.
    let mut cmap = UnicodeCmap::new(CMAP_NAME, SYSTEM_INFO);
    for (&g, text) in glyph_set.iter() {
        if !text.is_empty() {
            cmap.pair_with_multiple(glyph_cid(font, g), text.chars());
        }
    }

    cmap
}

/// Get the CID for a glyph id.
///
/// When writing text into a PDF, we have to specify CIDs (character ids) not
/// GIDs (glyph IDs).
///
/// Most of the time, the mapping between these two is an identity mapping. In
/// particular, for TrueType fonts, the mapping is an identity mapping because
/// of this line above:
/// ```ignore
/// cid.cid_to_gid_map_predefined(Name(b"Identity"));
/// ```
///
/// However, CID-keyed CFF fonts may have a non-identity mapping defined in
/// their charset. For those, we must map the glyph IDs in a `TextItem` to CIDs.
/// The font defines the map through its charset. The charset usually maps
/// glyphs to SIDs (string ids) specifying the glyph's name. Not for CID-keyed
/// fonts though! For these, the SIDs are CIDs in disguise. Relevant quote from
/// the CFF spec:
///
/// > The charset data, although in the same format as non-CIDFonts, will
/// > represent CIDs rather than SIDs, [...]
///
/// This function performs the mapping from glyph ID to CID. It also works for
/// non CID-keyed fonts. Then, it will simply return the glyph ID.
pub(super) fn glyph_cid(font: &Font, glyph_id: u16) -> u16 {
    font.ttf()
        .tables()
        .cff
        .and_then(|cff| cff.glyph_cid(ttf_parser::GlyphId(glyph_id)))
        .unwrap_or(glyph_id)
}
