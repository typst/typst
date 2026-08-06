#![no_main]

use libfuzzer_sys::fuzz_target;
use typst_fuzz::{FuzzWorld, Input};
use typst_html::{HtmlDocument, HtmlOptions};

fuzz_target!(|input: Input<'_>| {
    let world = FuzzWorld::new(input.text, input.preferred_version);
    let options = HtmlOptions::default();
    if let Ok(document) = typst::compile::<HtmlDocument>(&world).output {
        _ = std::hint::black_box(typst_html::html(&document, &options));
    }
    comemo::evict(10);
});
