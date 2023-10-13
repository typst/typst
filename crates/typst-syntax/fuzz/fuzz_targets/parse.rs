#![no_main]
use std::ops::Range;
use typst_syntax::parse;

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|text: &str| {
    _ = parse(text);
});
