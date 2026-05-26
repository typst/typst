#![no_main]

use libfuzzer_sys::fuzz_target;
use typst::model::Document;
use typst_fuzz::FuzzWorld;
use typst_layout::PagedDocument;
use typst_pdf::PdfOptions;
use typst_render::{Png, RenderOptions};
use typst_svg::{Svg, SvgOptions};

fuzz_target!(|text: &str| {
    let world = FuzzWorld::new(text);
    if let Ok(document) = typst::compile::<PagedDocument>(&world).output {
        if let Some(page) = document.pages().first() {
            let png_options =
                RenderOptions::default().resolve(document.options().get::<Png>());
            std::hint::black_box(typst_render::render(page, &png_options));

            let svg_options =
                SvgOptions::default().resolve(document.options().get::<Svg>());
            std::hint::black_box(typst_svg::svg(page, &svg_options));
        }
        _ = std::hint::black_box(typst_pdf::pdf(&document, &PdfOptions::default()));
    }
    comemo::evict(10);
});
