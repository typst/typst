use std::sync::Arc;

use typst::{diag::SourceResult, doc::Document};

use crate::export::svg::{
    flat_ir,
    flat_vector::FlatRenderVm,
    font::{FontGlyphProvider, GlyphProvider},
    ir::{self, GlyphMapping},
    vector::{
        codegen::{generate_text, SvgText, SvgTextNode},
        lowering::{GlyphLowerBuilder, LowerBuilder},
    },
    DefaultExportFeature, ExportFeature, Module, ModuleBuilder, MultiSvgDocument, Pages,
    SerializedModule, SvgDocument, SvgExporter, SvgTask,
};

impl<Feat: ExportFeature> SvgTask<Feat> {
    /// Render a document into the svg_body.
    pub fn render(
        &mut self,
        module: &Module,
        pages: &Pages,
        svg_body: &mut Vec<SvgText>,
    ) {
        let mut render_task = self.fork_render_task(module);

        let mut acc_height = 0u32;
        for (idx, page) in pages.iter().enumerate() {
            render_task.page_off = idx;

            let entry = &page.0;
            let size = Self::page_size(page.1);

            svg_body.push(SvgText::Content(Arc::new(SvgTextNode {
                attributes: vec![
                    ("transform", format!("translate(0, {})", acc_height)),
                    ("data-tid", entry.as_svg_id("p")),
                    ("data-page-width", size.x.to_string()),
                    ("data-page-height", size.y.to_string()),
                ],
                content: vec![SvgText::Content(render_task.render_flat_item(entry))],
            })));
            acc_height += size.y;
        }
    }
}

impl SvgExporter {
    pub(crate) fn header(output: &Pages) -> String {
        // calculate the width and height of the svg
        let w = output
            .iter()
            .map(|p| p.1.x.0.ceil())
            .max_by(|a, b| a.total_cmp(b))
            .unwrap();
        let h = output.iter().map(|p| p.1.y.0.ceil()).sum::<f32>();

        Self::header_inner(w, h)
    }

    pub fn svg_doc(output: &Document) -> (SvgDocument, GlyphMapping) {
        let mut lower_builder = LowerBuilder::new(output);
        let mut builder = ModuleBuilder::default();
        let pages = output
            .pages
            .iter()
            .map(|p| {
                let abs_ref = builder.build(lower_builder.lower(p));
                (abs_ref, p.size().into())
            })
            .collect::<Vec<_>>();
        let (module, glyph_mapping) = builder.finalize();

        (SvgDocument { pages, module }, glyph_mapping)
    }

    pub fn render_flat_svg(module: &Module, pages: &Pages) -> String {
        let header = Self::header(pages);

        let mut t = SvgTask::<DefaultExportFeature>::default();
        let mut svg_body = vec![];
        t.render(module, pages, &mut svg_body);

        let glyphs = t.render_glyphs(&module.glyphs, true);

        generate_text(Self::render_template(t, header, svg_body, glyphs.into_iter()))
    }

    pub(crate) fn render_flat_doc_and_svg(
        output: Arc<Document>,
    ) -> (SvgDocument, String) {
        // render the document
        let (doc, _used_glyphs) = Self::svg_doc(&output);

        let svg = Self::render_flat_svg(&doc.module, &doc.pages);
        (doc, svg)
    }
}

pub fn serialize_module(repr: Module) -> Vec<u8> {
    // Or you can customize your serialization for better performance
    // and compatibility with #![no_std] environments
    use rkyv::ser::{serializers::AllocSerializer, Serializer};

    let mut serializer = AllocSerializer::<0>::default();
    serializer.serialize_value(&repr.item_pack).unwrap();
    let item_pack = serializer.into_serializer().into_inner();

    item_pack.into_vec()
}

pub fn serialize_multi_doc_standalone(
    doc: MultiSvgDocument,
    glyph_mapping: GlyphMapping,
) -> Vec<u8> {
    let glyph_provider = GlyphProvider::new(FontGlyphProvider::default());
    let glyph_lower_builder = GlyphLowerBuilder::new(&glyph_provider);

    let glyphs = glyph_mapping
        .into_iter()
        .filter_map(|(glyph, glyph_id)| {
            let glyph = glyph_lower_builder.lower_glyph(&glyph);
            glyph.map(|t| {
                let t = match t {
                    ir::GlyphItem::Image(i) => flat_ir::FlatGlyphItem::Image(i),
                    ir::GlyphItem::Outline(p) => flat_ir::FlatGlyphItem::Outline(p),
                    _ => unreachable!(),
                };

                (glyph_id, t)
            })
        })
        .collect::<Vec<_>>();

    // Or you can customize your serialization for better performance
    // and compatibility with #![no_std] environments
    use rkyv::ser::{serializers::AllocSerializer, Serializer};

    let mut serializer = AllocSerializer::<0>::default();
    serializer
        .serialize_value(&SerializedModule {
            item_pack: doc.module.item_pack,
            glyphs,
            layouts: doc.layouts,
        })
        .unwrap();
    let item_pack = serializer.into_serializer().into_inner();

    item_pack.into_vec()
}

pub fn export_module(output: &Document) -> SourceResult<Vec<u8>> {
    let mut t = LowerBuilder::new(output);

    let mut builder = ModuleBuilder::default();

    for page in output.pages.iter() {
        let item = t.lower(page);
        let _entry_id = builder.build(item);
    }

    let (repr, ..) = builder.finalize();

    Ok(serialize_module(repr))
}
