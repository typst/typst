use std::collections::HashMap;
use std::hash::Hash;

use ecow::eco_format;
use pdf_writer::{writers::Resources, Dict, Finish, Name, Pdf, Ref};
use typst::{text::Font, visualize::Image};

use crate::{
    color_font::ColorFontSlice, extg::ExtGState, gradient::PdfGradient,
    pattern::PdfPattern, PdfContext, PdfWriter, References,
};

pub struct GlobalResources;

impl PdfWriter for GlobalResources {
    /// Write the global resource dictionary that will be referenced by all pages.
    ///
    /// We add a reference to this dictionary to each page individually instead of
    /// to the root node of the page tree because using the resource inheritance
    /// feature breaks PDF merging with Apple Preview.
    fn write(&self, pdf: &mut Pdf, alloc: &mut Ref, ctx: &PdfContext, refs: &References) {
        fn generic_resource_dict(
            pdf: &mut Pdf,
            alloc: &mut Ref,
            ctx: &PdfContext,
            refs: &References,
        ) {
            let images_ref = alloc.bump();
            let patterns_ref = alloc.bump();
            let ext_gs_states_ref = alloc.bump();
            let color_spaces_ref = alloc.bump();

            let mut pattern_mapping = HashMap::new();
            for (pattern, pattern_refs) in &refs.patterns {
                pattern_mapping.insert(pattern.clone(), pattern_refs.pattern_ref);
            }

            let mut color_font_slices = Vec::new();
            if let Some(color_fonts) = &ctx.color_fonts {
                for (font, color_font) in &color_fonts.map {
                    for i in 0..(color_font.glyphs.len() / 256) {
                        color_font_slices
                            .push(ColorFontSlice { font: font.clone(), subfont: i })
                    }
                }
            }

            resource_dict(
                pdf,
                ctx.globals.resources,
                images_ref,
                ResourceList {
                    prefix: "Im",
                    items: &ctx.images.to_items,
                    mapping: &refs.images,
                },
                patterns_ref,
                ResourceList {
                    prefix: "Gr",
                    items: &ctx.gradients.to_items,
                    mapping: &refs.gradients,
                },
                ResourceList {
                    prefix: "P",
                    items: &ctx
                        .patterns
                        .as_ref()
                        .map(|p| &p.remapper.to_items[..])
                        .unwrap_or_default(),
                    mapping: &pattern_mapping,
                },
                ext_gs_states_ref,
                ResourceList {
                    prefix: "Gs",
                    items: &ctx.ext_gs.to_items,
                    mapping: &refs.ext_gs,
                },
                color_spaces_ref,
                ResourceList {
                    prefix: "F",
                    items: &ctx.fonts.to_items,
                    mapping: &refs.fonts,
                },
                ResourceList {
                    prefix: "Cf",
                    items: &color_font_slices,
                    mapping: &refs.color_fonts,
                },
            );

            let color_spaces = pdf.indirect(color_spaces_ref).dict();
            ctx.colors.write_color_spaces(color_spaces, &ctx.globals);

            if let Some(color_fonts) = &ctx.color_fonts {
                generic_resource_dict(pdf, alloc, &color_fonts.ctx, refs);
            }

            if let Some(patterns) = &ctx.patterns {
                generic_resource_dict(pdf, alloc, &patterns.ctx, refs);
            }
        }

        generic_resource_dict(pdf, alloc, ctx, refs);

        // Write all of the functions used by the document.
        // TODO: subcontexts may refer to these functions I think,
        // but here they are only valid for the main context
        ctx.colors.write_functions(pdf, &ctx.globals);
    }
}

struct ResourceList<'a, T> {
    prefix: &'static str,
    items: &'a [T],
    mapping: &'a HashMap<T, Ref>,
}

impl<'a, T: Eq + Hash> ResourceList<'a, T> {
    fn write(&mut self, dict: &mut Dict) {
        for (number, item) in self.items.iter().enumerate() {
            let name = eco_format!("{}{}", self.prefix, number);
            let reference = self.mapping[item];
            dict.pair(Name(name.as_bytes()), reference);
        }
        dict.finish();
    }
}

fn resource_dict(
    pdf: &mut Pdf,
    id: Ref,
    images_ref: Ref,
    mut images: ResourceList<Image>,
    patterns_ref: Ref,
    mut gradients: ResourceList<PdfGradient>,
    mut patterns: ResourceList<PdfPattern>,
    ext_gs_ref: Ref,
    mut ext_gs: ResourceList<ExtGState>,
    color_spaces_ref: Ref,
    mut fonts: ResourceList<Font>,
    mut color_fonts: ResourceList<ColorFontSlice>,
) {
    let mut dict = pdf.indirect(images_ref).dict();
    images.write(&mut dict);
    dict.finish();

    let mut dict = pdf.indirect(patterns_ref).dict();
    gradients.write(&mut dict);
    patterns.write(&mut dict);
    dict.finish();

    let mut dict = pdf.indirect(ext_gs_ref).dict();
    ext_gs.write(&mut dict);
    dict.finish();

    let mut resources = pdf.indirect(id).start::<Resources>();
    resources.pair(Name(b"XObject"), images_ref);
    resources.pair(Name(b"Pattern"), patterns_ref);
    resources.pair(Name(b"ExtGState"), ext_gs_ref);
    resources.pair(Name(b"ColorSpace"), color_spaces_ref);

    let mut fonts_dict = resources.fonts();
    fonts.write(&mut fonts_dict);
    color_fonts.write(&mut fonts_dict);
    fonts.finish();
}
