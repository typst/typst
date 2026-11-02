#![no_main]

use libfuzzer_sys::fuzz_target;
use typst_fuzz::FuzzWorld;
use typst_html::{HtmlDocument, HtmlOptions};
use typst_syntax::{FileId, Source};

fuzz_target!(|text: &str| {
    let source = Source::new(FileId::new_for_test("fuzz_html_output"), text);
    let world = FuzzWorld::new(source);
    let options = HtmlOptions::default();
    if let Ok(document) = typst::compile::<HtmlDocument>(&world).output {
        _ = std::hint::black_box(typst_html::html(&document, &options));
    }
    comemo::evict(10);
});
