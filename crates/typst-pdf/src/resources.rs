use ecow::eco_format;
use pdf_writer::{writers::Resources, Dict, Finish, Name, Pdf, Ref};

use crate::{color_font::MaybeColorFont, PdfContext, PdfWriter, References};

pub struct GlobalResources;

impl PdfWriter for GlobalResources {
    /// Write the global resource dictionary that will be referenced by all pages.
    ///
    /// We add a reference to this dictionary to each page individually instead of
    /// to the root node of the page tree because using the resource inheritance
    /// feature breaks PDF merging with Apple Preview.
    fn write(&self, pdf: &mut Pdf, alloc: &mut Ref, ctx: &PdfContext, refs: &References) {
        let global_res_ref = ctx.globals.global_resources;
        let type3_font_resources_ref = ctx.globals.type3_font_resources;

        #[must_use]
        fn generic_resource_dict<C: MaybeColorFont>(
            pdf: &mut Pdf,
            alloc: &mut Ref,
            id: Ref,
            ctx: &PdfContext<C>,
            refs: &References,
            color_fonts: ResourceList,
        ) -> Ref {
            let images_ref = alloc.bump();
            let patterns_ref = alloc.bump();
            let ext_gs_states_ref = alloc.bump();
            let color_spaces_ref = alloc.bump();

            resource_dict(
                pdf,
                id,
                images_ref,
                ResourceList {
                    prefix: "Im",
                    items: &mut ctx.images.pdf_indices(&refs.images),
                },
                patterns_ref,
                ResourceList {
                    prefix: "Gr",
                    items: &mut ctx.gradients.pdf_indices(&refs.gradients),
                },
                ResourceList {
                    prefix: "P",
                    items: &mut ctx
                        .patterns
                        .pdf_indices(refs.patterns.iter().map(|p| &p.pattern_ref)),
                },
                ext_gs_states_ref,
                ResourceList {
                    prefix: "Gs",
                    items: &mut ctx.ext_gs.pdf_indices(&refs.ext_gs),
                },
                color_spaces_ref,
                ResourceList {
                    prefix: "F",
                    items: &mut ctx.fonts.pdf_indices(&refs.fonts),
                },
                color_fonts,
            );

            color_spaces_ref
        }

        let global_color_spaces = generic_resource_dict(
            pdf,
            alloc,
            global_res_ref,
            ctx,
            refs, // TODO: these refs are not matching with ctx
            ResourceList {
                prefix: "Cf",
                // TODO: allocate an actual number for color fonts instead of using their ID?
                items: &mut refs.color_fonts.iter().map(|r| (*r, r.get() as usize)),
            },
        );
        let color_spaces = pdf.indirect(global_color_spaces).dict();
        ctx.colors.write_color_spaces(color_spaces, &ctx.globals);

        let type3_color_spaces = generic_resource_dict(
            pdf,
            alloc,
            type3_font_resources_ref,
            &ctx.color_fonts.ctx,
            refs,
            ResourceList { prefix: "", items: &mut std::iter::empty() },
        );
        let color_spaces = pdf.indirect(type3_color_spaces).dict();
        ctx.color_fonts
            .ctx
            .colors
            .write_color_spaces(color_spaces, &ctx.globals);

        // Write the resources for each pattern
        for (refs, pattern) in refs.patterns.iter().zip(&ctx.remapped_patterns) {
            let resources = &pattern.resources;
            let resources_ref = refs.resources_ref;

            let mut resources_map: Resources = pdf.indirect(resources_ref).start();

            resources_map.x_objects().pairs(
                resources
                    .iter()
                    .filter(|(res, _)| res.is_x_object())
                    .map(|(res, ref_)| (res.name(), ref_)),
            );

            resources_map.fonts().pairs(
                resources
                    .iter()
                    .filter(|(res, _)| res.is_font())
                    .map(|(res, ref_)| (res.name(), ref_)),
            );

            ctx.colors
                .write_color_spaces(resources_map.color_spaces(), &ctx.globals);

            resources_map
                .patterns()
                .pairs(
                    resources
                        .iter()
                        .filter(|(res, _)| res.is_pattern())
                        .map(|(res, ref_)| (res.name(), ref_)),
                )
                .pairs(
                    resources
                        .iter()
                        .filter(|(res, _)| res.is_gradient())
                        .map(|(res, ref_)| (res.name(), ref_)),
                );

            resources_map.ext_g_states().pairs(
                resources
                    .iter()
                    .filter(|(res, _)| res.is_ext_g_state())
                    .map(|(res, ref_)| (res.name(), ref_)),
            );

            resources_map.finish();
        }

        // Write all of the functions used by the document.
        ctx.colors.write_functions(pdf, &ctx.globals);
    }
}

struct ResourceList<'a> {
    prefix: &'static str,
    items: &'a mut dyn Iterator<Item = (Ref, usize)>,
}

impl<'a> ResourceList<'a> {
    fn write(&mut self, dict: &mut Dict) {
        for (reference, number) in &mut self.items {
            let name = eco_format!("{}{}", self.prefix, number);
            dict.pair(Name(name.as_bytes()), reference);
        }
        dict.finish();
    }
}

fn resource_dict(
    pdf: &mut Pdf,
    id: Ref,
    images_ref: Ref,
    mut images: ResourceList,
    patterns_ref: Ref,
    mut gradients: ResourceList,
    mut patterns: ResourceList,
    ext_gs_ref: Ref,
    mut ext_gs: ResourceList,
    color_spaces_ref: Ref,
    mut fonts: ResourceList,
    mut color_fonts: ResourceList,
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
