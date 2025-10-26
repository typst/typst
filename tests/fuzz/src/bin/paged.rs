#![no_main]

use libfuzzer_sys::fuzz_target;
use typst::layout::PagedDocument;
use typst_fuzz::FuzzWorld;
use typst_pdf::PdfOptions;
use typst_render::RenderOptions;
use typst_svg::SvgOptions;

fuzz_target!(|text: &str| {
    let world = FuzzWorld::new(text);
    if let Ok(document) = typst::compile::<PagedDocument>(&world).output {
        if let Some(page) = document.pages.first() {
            std::hint::black_box(typst_render::render(page, RenderOptions::default()));
            std::hint::black_box(typst_svg::svg(page, SvgOptions::default()));
        }
        _ = std::hint::black_box(typst_pdf::pdf(&document, &PdfOptions::default()));
    }
    comemo::evict(10);
});
