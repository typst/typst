#![no_main]

use typst_syntax::parse;


use libfuzzer_sys::fuzz_target;

fuzz_target!(|text: &str| {
    _ = parse(text);
});
