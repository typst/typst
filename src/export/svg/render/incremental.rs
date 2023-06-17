use std::{
    collections::{hash_map::RandomState, HashSet},
    sync::Arc,
};

use typst::doc::Document;

use crate::export::svg::{
    flat_vector::{FlatRenderVm, SvgDocument},
    ir::AbsoulteRef,
    vector::codegen::{SvgText, SvgTextNode},
    DefaultExportFeature, ExportFeature, SvgExporter, SvgTask,
};

pub struct IncrementalRenderContext {
    prev: SvgDocument,
    next: SvgDocument,
}

impl<Feat: ExportFeature> SvgTask<Feat> {
    /// Render a document difference into the svg_body.
    pub fn render_diff(
        &mut self,
        ctx: &IncrementalRenderContext,
        svg_body: &mut Vec<SvgText>,
    ) {
        let mut acc_height = 0u32;
        let mut render_task = self.fork_render_task(&ctx.next.module);

        let reusable: HashSet<AbsoulteRef, RandomState> =
            HashSet::from_iter(ctx.prev.pages.iter().map(|e| e.0.clone()));

        for (idx, (entry, size)) in ctx.next.pages.iter().enumerate() {
            render_task.page_off = idx;

            let size = Self::page_size(*size);
            if reusable.contains(entry) {
                svg_body.push(SvgText::Content(Arc::new(SvgTextNode {
                    attributes: vec![
                        ("transform", format!("translate(0, {})", acc_height)),
                        ("data-tid", entry.as_svg_id("p")),
                        ("data-reuse-from", entry.as_svg_id("p")),
                        ("data-page-width", size.x.to_string()),
                        ("data-page-height", size.y.to_string()),
                    ],
                    content: vec![],
                })));

                acc_height += size.y;
                continue;
            }

            let item = render_task.render_flat_item(entry);

            let mut attributes = vec![
                ("transform", format!("translate(0, {})", acc_height)),
                ("data-tid", entry.as_svg_id("p")),
                ("data-page-width", size.x.to_string()),
                ("data-page-height", size.y.to_string()),
            ];

            // todo: evaluate simlarity
            if let Some((abs_ref, ..)) = ctx.prev.pages.get(idx) {
                attributes.push(("data-reuse-from", abs_ref.as_svg_id("p")));
            }

            svg_body.push(SvgText::Content(Arc::new(SvgTextNode {
                attributes,
                content: vec![SvgText::Content(item)],
            })));
            acc_height += size.y;
        }
    }
}

impl SvgExporter {
    fn render_svg_incremental(
        prev: SvgDocument,
        output: Arc<Document>,
    ) -> (SvgDocument, String) {
        let instant = std::time::Instant::now();

        // render the document
        let mut t = SvgTask::<DefaultExportFeature>::default();

        let (next, used_glyphs) = Self::svg_doc(&output);

        let mut svg = Vec::<SvgText>::new();
        svg.push(SvgText::Plain(Self::header(&next.pages)));
        let mut svg_body = vec![];

        let render_context = IncrementalRenderContext { prev, next };
        t.render_diff(&render_context, &mut svg_body);
        let svg_doc = render_context.next;

        // base style
        svg.push(r#"<style type="text/css" data-reuse="1">"#.into());
        svg.push("</style>".into());

        // attach the glyph defs, clip paths, and style defs
        svg.push("<defs>".into());
        let _ = used_glyphs;

        svg.push("</defs>".into());

        // incremental style
        svg.push(r#"<style type="text/css" data-reuse="1">"#.into());
        svg.push("</style>".into());

        // body
        svg.append(&mut svg_body);

        // attach the javascript for animations
        svg.push(r#"<script type="text/javascript" data-reuse="1">"#.into());
        svg.push("</script>".into());

        svg.push("</svg>".into());

        println!("svg render time (incremental): {:?}", instant.elapsed());

        let mut string_io = String::new();
        string_io.reserve(svg.iter().map(SvgText::estimated_len).sum());
        for s in svg {
            s.write_string_io(&mut string_io);
        }
        (svg_doc, string_io)
    }
}

#[derive(Default)]
pub struct IncrementalSvgExporter {
    prev: Option<SvgDocument>,
}

impl IncrementalSvgExporter {
    pub fn render(&mut self, output: Arc<Document>) -> String {
        let (next, packet) = match self.prev.take() {
            Some(prev) => {
                let (next, svg) = SvgExporter::render_svg_incremental(prev, output);
                (next, ["diff-v0,", &svg].concat())
            }
            None => {
                let (next, svg) = SvgExporter::render_flat_doc_and_svg(output);
                (next, ["new,", &svg].concat())
            }
        };

        self.prev = Some(next);
        packet
    }
}
