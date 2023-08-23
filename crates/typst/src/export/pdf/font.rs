use std::collections::BTreeMap;

use ecow::{eco_format, EcoString};
use pdf_writer::types::{CidFontType, FontFlags, SystemInfo, UnicodeCmap};
use pdf_writer::{Filter, Finish, Name, Rect, Str};
use ttf_parser::{name_id, GlyphId, Tag};
use unicode_general_category::GeneralCategory;

use super::{deflate, EmExt, PdfContext, RefExt};
use crate::eval::Bytes;
use crate::font::Font;
use crate::util::SliceExt;

const CMAP_NAME: Name = Name(b"Custom");
const SYSTEM_INFO: SystemInfo = SystemInfo {
    registry: Str(b"Adobe"),
    ordering: Str(b"Identity"),
    supplement: 0,
};

/// Embed all used fonts into the PDF.
#[tracing::instrument(skip_all)]
pub fn write_fonts(ctx: &mut PdfContext) {
    for font in ctx.font_map.items() {
        let type0_ref = ctx.alloc.bump();
        let cid_ref = ctx.alloc.bump();
        let descriptor_ref = ctx.alloc.bump();
        let cmap_ref = ctx.alloc.bump();
        let data_ref = ctx.alloc.bump();
        ctx.font_refs.push(type0_ref);

        let glyph_set = ctx.glyph_sets.get_mut(font).unwrap();
        let metrics = font.metrics();
        let ttf = font.ttf();

        let postscript_name = font
            .find_name(name_id::POST_SCRIPT_NAME)
            .unwrap_or_else(|| "unknown".to_string());

        let base_font = eco_format!("ABCDEF+{}", postscript_name);
        let base_font = Name(base_font.as_bytes());

        // Write the base font object referencing the CID font.
        ctx.writer
            .type0_font(type0_ref)
            .base_font(base_font)
            .encoding_predefined(Name(b"Identity-H"))
            .descendant_font(cid_ref)
            .to_unicode(cmap_ref);

        // Check for the presence of CFF outlines to select the correct
        // CID-Font subtype.
        let subtype = match ttf
            .raw_face()
            .table(Tag::from_bytes(b"CFF "))
            .or(ttf.raw_face().table(Tag::from_bytes(b"CFF2")))
        {
            Some(_) => CidFontType::Type0,
            None => CidFontType::Type2,
        };

        // Write the CID font referencing the font descriptor.
        let mut cid = ctx.writer.cid_font(cid_ref);
        cid.subtype(subtype);
        cid.base_font(base_font);
        cid.system_info(SYSTEM_INFO);
        cid.font_descriptor(descriptor_ref);
        cid.default_width(0.0);

        if subtype == CidFontType::Type2 {
            cid.cid_to_gid_map_predefined(Name(b"Identity"));
        }

        // Extract the widths of all glyphs.
        let num_glyphs = ttf.number_of_glyphs();
        let mut widths = vec![0.0; num_glyphs as usize];
        for &g in glyph_set.keys() {
            let x = ttf.glyph_hor_advance(GlyphId(g)).unwrap_or(0);
            widths[g as usize] = font.to_em(x).to_font_units();
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
        let mut font_descriptor = ctx.writer.font_descriptor(descriptor_ref);
        font_descriptor
            .name(base_font)
            .flags(flags)
            .bbox(bbox)
            .italic_angle(italic_angle)
            .ascent(ascender)
            .descent(descender)
            .cap_height(cap_height)
            .stem_v(stem_v);

        match subtype {
            CidFontType::Type0 => font_descriptor.font_file3(data_ref),
            CidFontType::Type2 => font_descriptor.font_file2(data_ref),
        };

        font_descriptor.finish();

        // Write the /ToUnicode character map, which maps glyph ids back to
        // unicode codepoints to enable copying out of the PDF.
        let cmap = create_cmap(ttf, glyph_set);
        ctx.writer.cmap(cmap_ref, &cmap.finish());

        // Subset and write the font's bytes.
        let glyphs: Vec<_> = glyph_set.keys().copied().collect();
        let data = subset_font(font, &glyphs);
        let mut stream = ctx.writer.stream(data_ref, &data);
        stream.filter(Filter::FlateDecode);

        if subtype == CidFontType::Type0 {
            stream.pair(Name(b"Subtype"), Name(b"CIDFontType0C"));
        }

        stream.finish();
    }
}

/// Subset a font to the given glyphs.
#[comemo::memoize]
fn subset_font(font: &Font, glyphs: &[u16]) -> Bytes {
    let data = font.data();
    let profile = subsetter::Profile::pdf(glyphs);
    let subsetted = subsetter::subset(data, font.index(), profile);
    let data = subsetted.as_deref().unwrap_or(data);
    deflate(data).into()
}

/// Create a /ToUnicode CMap.
fn create_cmap(
    ttf: &ttf_parser::Face,
    glyph_set: &mut BTreeMap<u16, EcoString>,
) -> UnicodeCmap {
    // For glyphs that have codepoints mapping to in the font's cmap table, we
    // prefer them over pre-existing text mappings from the document. Only
    // things that don't have a corresponding codepoint (or only a private-use
    // one) like the "Th" in Linux Libertine get the text of their first
    // occurrences in the document instead.
    for subtable in ttf.tables().cmap.into_iter().flat_map(|table| table.subtables) {
        if !subtable.is_unicode() {
            continue;
        }

        subtable.codepoints(|n| {
            let Some(c) = std::char::from_u32(n) else { return };
            if unicode_general_category::get_general_category(c)
                == GeneralCategory::PrivateUse
            {
                return;
            }

            let Some(GlyphId(g)) = ttf.glyph_index(c) else { return };
            if glyph_set.contains_key(&g) {
                glyph_set.insert(g, c.into());
            }
        });
    }

    // Produce a reverse mapping from glyphs to unicode strings.
    let mut cmap = UnicodeCmap::new(CMAP_NAME, SYSTEM_INFO);
    for (&g, text) in glyph_set.iter() {
        if !text.is_empty() {
            cmap.pair_with_multiple(g, text.chars());
        }
    }

    cmap
}
