use bencher::Bencher;
use typeset::Typesetter;
use typeset::font::FileSystemFontProvider;


fn typesetting(b: &mut Bencher) {
    let src = include_str!("../test/shakespeare.tps");

    let mut typesetter = Typesetter::new();
    let provider = FileSystemFontProvider::from_listing("../fonts/fonts.toml").unwrap();
    typesetter.add_font_provider(provider);

    b.iter(|| {
        let _document = typesetter.typeset(&src).unwrap();
    });
}

bencher::benchmark_group!(benches, typesetting);
bencher::benchmark_main!(benches);
