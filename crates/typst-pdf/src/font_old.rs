use std::collections::{BTreeMap, HashMap};
use std::hash::Hash;
use std::sync::Arc;

use ecow::{eco_format, EcoString};
use pdf_writer::types::{CidFontType, FontFlags, SystemInfo, UnicodeCmap};
use pdf_writer::writers::{FontDescriptor, WMode};
use pdf_writer::{Chunk, Filter, Finish, Name, Rect, Ref, Str};
use subsetter::GlyphRemapper;
use ttf_parser::{name_id, GlyphId, Tag};
use typst_library::diag::{At, SourceResult};
use typst_library::text::Font;
use typst_syntax::Span;
use typst_utils::SliceExt;

use crate::{deflate, EmExt, NameExt, PdfChunk, WithGlobalRefs};

const CFF: Tag = Tag::from_bytes(b"CFF ");
const CFF2: Tag = Tag::from_bytes(b"CFF2");

const SUBSET_TAG_LEN: usize = 6;
const IDENTITY_H: &str = "Identity-H";

pub(crate) const CMAP_NAME: Name = Name(b"Custom");
pub(crate) const SYSTEM_INFO: SystemInfo = SystemInfo {
    registry: Str(b"Adobe"),
    ordering: Str(b"Identity"),
    supplement: 0,
};

/// Embed all used fonts into the PDF.
#[typst_macros::time(name = "write fonts")]
pub fn write_fonts(
    context: &WithGlobalRefs,
) -> SourceResult<(PdfChunk, HashMap<Font, Ref>)> {
    let mut chunk = PdfChunk::new();
    let mut out = HashMap::new();
    context.resources.traverse(&mut |resources| {
        for font in resources.fonts.items() {
            if out.contains_key(font) {
                continue;
            }

            let type0_ref = chunk.alloc();
            let cid_ref = chunk.alloc();
            let descriptor_ref = chunk.alloc();
            let cmap_ref = chunk.alloc();
            let data_ref = chunk.alloc();
            out.insert(font.clone(), type0_ref);

            let glyph_set = resources.glyph_sets.get(font).unwrap();
            let glyph_remapper = resources.glyph_remappers.get(font).unwrap();
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

            let base_font = base_font_name(font, glyph_set);
            let base_font_type0 = if is_cff {
                eco_format!("{base_font}-{IDENTITY_H}")
            } else {
                base_font.clone()
            };

            // Write the base font object referencing the CID font.
            chunk
                .type0_font(type0_ref)
                .base_font(Name(base_font_type0.as_bytes()))
                .encoding_predefined(Name(IDENTITY_H.as_bytes()))
                .descendant_font(cid_ref)
                .to_unicode(cmap_ref);

            // Write the CID font referencing the font descriptor.
            let mut cid = chunk.cid_font(cid_ref);
            cid.subtype(if is_cff { CidFontType::Type0 } else { CidFontType::Type2 });
            cid.base_font(Name(base_font.as_bytes()));
            cid.system_info(SYSTEM_INFO);
            cid.font_descriptor(descriptor_ref);
            cid.default_width(0.0);
            if !is_cff {
                cid.cid_to_gid_map_predefined(Name(b"Identity"));
            }

            // Extract the widths of all glyphs.
            // `remapped_gids` returns an iterator over the old GIDs in their new sorted
            // order, so we can append the widths as is.
            let widths = glyph_remapper
                .remapped_gids()
                .map(|gid| {
                    let width = ttf.glyph_hor_advance(GlyphId(gid)).unwrap_or(0);
                    font.to_em(width).to_font_units()
                })
                .collect::<Vec<_>>();

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
            let cmap = create_cmap(glyph_set, glyph_remapper);
            chunk
                .cmap(cmap_ref, &cmap)
                .writing_mode(WMode::Horizontal)
                .filter(Filter::FlateDecode);

            let subset = subset_font(font, glyph_remapper)
                .map_err(|err| {
                    let postscript_name = font.find_name(name_id::POST_SCRIPT_NAME);
                    let name = postscript_name.as_deref().unwrap_or(&font.info().family);
                    eco_format!("failed to process font {name}: {err}")
                })
                .at(Span::detached())?;

            let mut stream = chunk.stream(data_ref, &subset);
            stream.filter(Filter::FlateDecode);
            if is_cff {
                stream.pair(Name(b"Subtype"), Name(b"CIDFontType0C"));
            }
            stream.finish();

            let mut font_descriptor =
                write_font_descriptor(&mut chunk, descriptor_ref, font, &base_font);
            if is_cff {
                font_descriptor.font_file3(data_ref);
            } else {
                font_descriptor.font_file2(data_ref);
            }
        }

        Ok(())
    })?;

    Ok((chunk, out))
}

/// Writes a FontDescriptor dictionary.
pub fn write_font_descriptor<'a>(
    pdf: &'a mut Chunk,
    descriptor_ref: Ref,
    font: &'a Font,
    base_font: &str,
) -> FontDescriptor<'a> {
    let ttf = font.ttf();
    let metrics = font.metrics();
    let serif = font
        .find_name(name_id::POST_SCRIPT_NAME)
        .is_some_and(|name| name.contains("Serif"));

    let mut flags = FontFlags::empty();
    flags.set(FontFlags::SERIF, serif);
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
/// - For a font with TrueType outlines, this produces the whole OpenType font.
/// - For a font with CFF outlines, this produces just the CFF font program.
///
/// In both cases, this returns the already compressed data.
#[comemo::memoize]
#[typst_macros::time(name = "subset font")]
fn subset_font(
    font: &Font,
    glyph_remapper: &GlyphRemapper,
) -> Result<Arc<Vec<u8>>, subsetter::Error> {
    let data = font.data();
    let subset = subsetter::subset(data, font.index(), glyph_remapper)?;
    let mut data = subset.as_ref();

    // Extract the standalone CFF font program if applicable.
    let raw = ttf_parser::RawFace::parse(data, 0).unwrap();
    if let Some(cff) = raw.table(CFF) {
        data = cff;
    }

    Ok(Arc::new(deflate(data)))
}

/// Creates the base font name for a font with a specific glyph subset.
/// Consists of a subset tag and the PostScript name of the font.
///
/// Returns a string of length maximum 116, so that even with `-Identity-H`
/// added it does not exceed the maximum PDF/A name length of 127.
pub(crate) fn base_font_name<T: Hash>(font: &Font, glyphs: &T) -> EcoString {
    const MAX_LEN: usize = Name::PDFA_LIMIT - REST_LEN;
    const REST_LEN: usize = SUBSET_TAG_LEN + 1 + 1 + IDENTITY_H.len();

    let postscript_name = font.find_name(name_id::POST_SCRIPT_NAME);
    let name = postscript_name.as_deref().unwrap_or("unknown");
    let trimmed = &name[..name.len().min(MAX_LEN)];

    // Hash the full name (we might have trimmed) and the glyphs to produce
    // a fairly unique subset tag.
    let subset_tag = subset_tag(&(name, glyphs));

    eco_format!("{subset_tag}+{trimmed}")
}

/// Produce a unique 6 letter tag for a glyph set.
pub(crate) fn subset_tag<T: Hash>(glyphs: &T) -> EcoString {
    const BASE: u128 = 26;
    let mut hash = typst_utils::hash128(&glyphs);
    let mut letter = [b'A'; SUBSET_TAG_LEN];
    for l in letter.iter_mut() {
        *l = b'A' + (hash % BASE) as u8;
        hash /= BASE;
    }
    std::str::from_utf8(&letter).unwrap().into()
}

/// Create a compressed `/ToUnicode` CMap.
#[comemo::memoize]
#[typst_macros::time(name = "create cmap")]
fn create_cmap(
    glyph_set: &BTreeMap<u16, EcoString>,
    glyph_remapper: &GlyphRemapper,
) -> Arc<Vec<u8>> {
    // Produce a reverse mapping from glyphs' CIDs to unicode strings.
    let mut cmap = UnicodeCmap::new(CMAP_NAME, SYSTEM_INFO);
    for (&g, text) in glyph_set.iter() {
        // See commend in `write_normal_text` for why we can choose the CID this way.
        let cid = glyph_remapper.get(g).unwrap();
        if !text.is_empty() {
            cmap.pair_with_multiple(cid, text.chars());
        }
    }
    Arc::new(deflate(&cmap.finish()))
}
