use iai::{black_box, main};

use typst::diag::TypResult;
use typst::loading::FileId;
use typst::parse::{parse, Scanner, TokenMode, Tokens};
use typst::syntax::SyntaxTree;

const SRC: &str = include_str!("../../tests/typ/coma.typ");

fn bench_decode() -> usize {
    // We don't use chars().count() because that has a special
    // superfast implementation.
    let mut count = 0;
    let mut chars = black_box(SRC).chars();
    while let Some(_) = chars.next() {
        count += 1;
    }
    count
}

fn bench_scan() -> usize {
    let mut count = 0;
    let mut scanner = Scanner::new(black_box(SRC));
    while let Some(_) = scanner.eat() {
        count += 1;
    }
    count
}

fn bench_tokenize() -> usize {
    Tokens::new(black_box(SRC), black_box(TokenMode::Markup)).count()
}

fn bench_parse() -> TypResult<SyntaxTree> {
    parse(FileId::from_raw(0), black_box(SRC))
}

main!(bench_decode, bench_scan, bench_tokenize, bench_parse);
