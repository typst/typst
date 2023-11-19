#![no_main]

use libfuzzer_sys::fuzz_target;
use typst_syntax::parse;

fuzz_target!(|text: &str| {
    _ = parse(text);
});
