#![no_main]

use libfuzzer_sys::fuzz_target;
use typst_fuzz::FuzzWorld;
use typst_html::{HtmlDocument, HtmlOptions};

fuzz_target!(|text: &str| {
    let world = FuzzWorld::new(text);
    let options = HtmlOptions::default();
    if let Ok(document) = typst::compile::<HtmlDocument>(&world).output {
        _ = std::hint::black_box(typst_html::html(&document, &options));
    }
    comemo::evict(10);
});
