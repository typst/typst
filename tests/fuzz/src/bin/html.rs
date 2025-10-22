#![no_main]

use libfuzzer_sys::fuzz_target;
use typst_fuzz::FuzzWorld;
use typst_html::HtmlDocument;

fuzz_target!(|text: &str| {
    let world = FuzzWorld::new(text);
    if let Ok(document) = typst::compile::<HtmlDocument>(&world).output {
        _ = std::hint::black_box(typst_html::html(&document));
    }
    comemo::evict(10);
});
