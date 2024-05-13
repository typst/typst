use ecow::eco_format;
use pdf_writer::{writers::Resources, Finish, Name, Pdf, Ref};

use crate::{ConstructContext, PdfWriter, WriteContext};

pub struct GlobalResources;

impl PdfWriter for GlobalResources {
    /// Write the global resource dictionary that will be referenced by all pages.
    ///
    /// We add a reference to this dictionary to each page individually instead of
    /// to the root node of the page tree because using the resource inheritance
    /// feature breaks PDF merging with Apple Preview.
    fn write(
        &self,
        pdf: &mut Pdf,
        alloc: &mut Ref,
        ctx: &ConstructContext,
        refs: &WriteContext,
    ) {
        let global_res_ref = ctx.globals.global_resources;
        let type3_font_resources_ref = ctx.globals.type3_font_resources;
        let images_ref = alloc.bump();
        let patterns_ref = alloc.bump();
        let ext_gs_states_ref = alloc.bump();
        let color_spaces_ref = alloc.bump();

        let mut images = pdf.indirect(images_ref).dict();
        for (image_ref, im) in ctx.images.pdf_indices(&refs.images) {
            let name = eco_format!("Im{}", im);
            images.pair(Name(name.as_bytes()), image_ref);
        }
        images.finish();

        let mut patterns = pdf.indirect(patterns_ref).dict();
        for (gradient_ref, gr) in ctx.gradients.pdf_indices(&refs.gradients) {
            let name = eco_format!("Gr{}", gr);
            patterns.pair(Name(name.as_bytes()), gradient_ref);
        }

        let pattern_refs = refs.patterns.iter().map(|p| &p.pattern_ref);
        for (pattern_ref, p) in ctx.patterns.pdf_indices(pattern_refs) {
            let name = eco_format!("P{}", p);
            patterns.pair(Name(name.as_bytes()), pattern_ref);
        }
        patterns.finish();

        let mut ext_gs_states = pdf.indirect(ext_gs_states_ref).dict();
        for (gs_ref, gs) in ctx.ext_gs.pdf_indices(&refs.ext_gs) {
            let name = eco_format!("Gs{}", gs);
            ext_gs_states.pair(Name(name.as_bytes()), gs_ref);
        }
        ext_gs_states.finish();

        let color_spaces = pdf.indirect(color_spaces_ref).dict();
        ctx.colors.write_color_spaces(color_spaces, &ctx.globals);

        let mut resources = pdf.indirect(global_res_ref).start::<Resources>();
        resources.pair(Name(b"XObject"), images_ref);
        resources.pair(Name(b"Pattern"), patterns_ref);
        resources.pair(Name(b"ExtGState"), ext_gs_states_ref);
        resources.pair(Name(b"ColorSpace"), color_spaces_ref);

        let mut fonts = resources.fonts();
        for (font_ref, f) in ctx.fonts.pdf_indices(&refs.fonts) {
            let name = eco_format!("F{}", f);
            fonts.pair(Name(name.as_bytes()), font_ref);
        }

        for font in &ctx.color_fonts.all_refs {
            let name = eco_format!("Cf{}", font.get());
            fonts.pair(Name(name.as_bytes()), font);
        }
        fonts.finish();

        resources.finish();

        // Also write the resources for Type3 fonts, that only contains images,
        // color spaces and regular fonts (COLR glyphs depend on them).
        if !ctx.color_fonts.all_refs.is_empty() {
            let mut resources =
                pdf.indirect(type3_font_resources_ref).start::<Resources>();
            resources.pair(Name(b"XObject"), images_ref);
            resources.pair(Name(b"Pattern"), patterns_ref);
            resources.pair(Name(b"ExtGState"), ext_gs_states_ref);
            resources.pair(Name(b"ColorSpace"), color_spaces_ref);

            let mut fonts = resources.fonts();
            for (font_ref, f) in ctx.fonts.pdf_indices(&refs.fonts) {
                let name = eco_format!("F{}", f);
                fonts.pair(Name(name.as_bytes()), font_ref);
            }
            fonts.finish();

            resources.finish();
        }

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
