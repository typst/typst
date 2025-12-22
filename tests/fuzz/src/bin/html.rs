#![no_main]

use libfuzzer_sys::fuzz_target;
use typst_fuzz::{FuzzWorld, Input};
use typst_html::{HtmlDocument, HtmlOptions};
use typst_syntax::{FileId, Source};

fuzz_target!(|input: Input<'_>| {
    let source = Source::new(
        FileId::new_for_test("fuzz_html_output"),
        input.text,
        input.preferred_version,
    );
    let world = FuzzWorld::new(source, input.preferred_version);
    let options = HtmlOptions::default();
    if let Ok(document) = typst::compile::<HtmlDocument>(&world).output {
        _ = std::hint::black_box(typst_html::html(&document, &options));
    }
    comemo::evict(10);
});
