use std::collections::HashMap;
use std::hash::Hash;

use ecow::eco_format;
use pdf_writer::{Dict, Finish, Name, Ref};
use typst::{text::Font, visualize::Image};

use crate::{
    color::ColorSpaces, color_font::ColorFontSlice, extg::ExtGState,
    gradient::PdfGradient, pattern::PdfPattern, AllocGlobalRefs, PdfChunk, Renumber,
    Resources, WriteResources,
};

pub struct ResourcesRefs {
    pub reference: Ref,
    pub color_fonts: Option<Box<ResourcesRefs>>,
    pub patterns: Option<Box<ResourcesRefs>>,
}

impl Renumber for ResourcesRefs {
    fn renumber(&mut self, mapping: &HashMap<Ref, Ref>) {
        self.reference.renumber(mapping);
        if let Some(color_fonts) = &mut self.color_fonts {
            color_fonts.renumber(mapping);
        }
        if let Some(patterns) = &mut self.patterns {
            patterns.renumber(mapping);
        }
    }
}

pub fn alloc_resources_refs(context: &AllocGlobalRefs) -> (PdfChunk, ResourcesRefs) {
    let mut chunk = PdfChunk::new();
    fn refs_for(resources: &Resources<()>, chunk: &mut PdfChunk) -> ResourcesRefs {
        ResourcesRefs {
            reference: chunk.alloc(),
            color_fonts: resources
                .color_fonts
                .as_ref()
                .map(|c| Box::new(refs_for(&c.resources, chunk))),
            patterns: resources
                .patterns
                .as_ref()
                .map(|p| Box::new(refs_for(&p.resources, chunk))),
        }
    }

    let refs = refs_for(&context.resources, &mut chunk);
    (chunk, refs)
}

/// Write the resource dictionaries that will be referenced by all pages.
///
/// We add a reference to this dictionary to each page individually instead of
/// to the root node of the page tree because using the resource inheritance
/// feature breaks PDF merging with Apple Preview.
///
/// Also write resource dictionaries for Type3 fonts and patterns.
pub fn write_resource_dictionaries(ctx: &WriteResources) -> (PdfChunk, ()) {
    let mut chunk = PdfChunk::new();
    let mut used_color_spaces = ColorSpaces::default();

    ctx.resources.traverse(&mut |resources| {
        used_color_spaces.merge(&resources.colors);

        let images_ref = chunk.alloc.bump();
        let patterns_ref = chunk.alloc.bump();
        let ext_gs_states_ref = chunk.alloc.bump();
        let color_spaces_ref = chunk.alloc.bump();

        let mut pattern_mapping = HashMap::new();
        for (pattern, pattern_refs) in &ctx.references.patterns {
            pattern_mapping.insert(pattern.clone(), pattern_refs.pattern_ref);
        }

        let mut color_font_slices = Vec::new();
        if let Some(color_fonts) = &resources.color_fonts {
            for (font, color_font) in &color_fonts.map {
                for i in 0..(color_font.glyphs.len() / 256) + 1 {
                    color_font_slices
                        .push(ColorFontSlice { font: font.clone(), subfont: i })
                }
            }
        }

        resource_dict(
            &mut chunk,
            resources.reference,
            images_ref,
            ResourceList {
                prefix: "Im",
                items: &resources.images.to_items,
                mapping: &ctx.references.images,
            },
            patterns_ref,
            ResourceList {
                prefix: "Gr",
                items: &resources.gradients.to_items,
                mapping: &ctx.references.gradients,
            },
            ResourceList {
                prefix: "P",
                items: resources
                    .patterns
                    .as_ref()
                    .map(|p| &p.remapper.to_items[..])
                    .unwrap_or_default(),
                mapping: &pattern_mapping,
            },
            ext_gs_states_ref,
            ResourceList {
                prefix: "Gs",
                items: &resources.ext_gs.to_items,
                mapping: &ctx.references.ext_gs,
            },
            color_spaces_ref,
            ResourceList {
                prefix: "F",
                items: &resources.fonts.to_items,
                mapping: &ctx.references.fonts,
            },
            ResourceList {
                prefix: "Cf",
                items: &color_font_slices,
                mapping: &ctx.references.color_fonts,
            },
        );

        let color_spaces = chunk.indirect(color_spaces_ref).dict();
        resources
            .colors
            .write_color_spaces(color_spaces, &ctx.globals.color_functions);
    });

    used_color_spaces.write_functions(&mut chunk, &ctx.globals.color_functions);

    (chunk, ())
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

#[allow(clippy::too_many_arguments)] // TODO
fn resource_dict(
    pdf: &mut PdfChunk,
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

    let mut resources = pdf.indirect(id).start::<pdf_writer::writers::Resources>();
    resources.pair(Name(b"XObject"), images_ref);
    resources.pair(Name(b"Pattern"), patterns_ref);
    resources.pair(Name(b"ExtGState"), ext_gs_ref);
    resources.pair(Name(b"ColorSpace"), color_spaces_ref);

    let mut fonts_dict = resources.fonts();
    fonts.write(&mut fonts_dict);
    color_fonts.write(&mut fonts_dict);
    fonts.finish();
}
