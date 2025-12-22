#![no_main]

use libfuzzer_sys::fuzz_target;
use typst_fuzz::Input;
use typst_syntax::parse;

fuzz_target!(|input: Input<'_>| {
    std::hint::black_box(parse(input.text, input.preferred_version));
});
