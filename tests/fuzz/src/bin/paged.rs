#![no_main]

use libfuzzer_sys::fuzz_target;
use typst_fuzz::{FuzzWorld, Input};
use typst_layout::PagedDocument;
use typst_pdf::PdfOptions;

fuzz_target!(|input: Input<'_>| {
    let world = FuzzWorld::new(input.text, input.preferred_version);
    if let Ok(document) = typst::compile::<PagedDocument>(&world).output {
        if let Some(page) = document.pages().first() {
            std::hint::black_box(typst_render::render(page, 1.0));
            std::hint::black_box(typst_svg::svg(page));
        }
        _ = std::hint::black_box(typst_pdf::pdf(&document, &PdfOptions::default()));
    }
    comemo::evict(10);
});
