#![no_main]

use libfuzzer_sys::fuzz_target;
use typst::layout::PagedDocument;
use typst_fuzz::FuzzWorld;
use typst_pdf::PdfOptions;

fuzz_target!(|text: &str| {
    let world = FuzzWorld::new(text);
    if let Ok(document) = typst::compile::<PagedDocument>(&world).output {
        if let Some(page) = document.pages.first() {
            let opts = typst_render::Options { pixel_per_pt: 1.0, render_bleed: false };
            std::hint::black_box(typst_render::render(page, opts));

            let opts = typst_svg::Options { render_bleed: false };
            std::hint::black_box(typst_svg::svg(page, opts));
        }
        _ = std::hint::black_box(typst_pdf::pdf(&document, &PdfOptions::default()));
    }
    comemo::evict(10);
});
