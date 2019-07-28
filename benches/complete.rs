use bencher::Bencher;
use typeset::Typesetter;
use typeset::font::FileSystemFontProvider;
use typeset::export::pdf::PdfExporter;


fn prepare<'p>() -> (Typesetter<'p>, &'static str) {
    let src = include_str!("../test/shakespeare.tps");

    let mut typesetter = Typesetter::new();
    let provider = FileSystemFontProvider::from_listing("../fonts/fonts.toml").unwrap();
    typesetter.add_font_provider(provider);

    (typesetter, src)
}

/// Benchmarks only the parsing step.
fn parsing(b: &mut Bencher) {
    let (typesetter, src) = prepare();
    b.iter(|| { typesetter.parse(src).unwrap(); });
}

/// Benchmarks only the layouting step.
fn layouting(b: &mut Bencher) {
    let (typesetter, src) = prepare();
    let tree = typesetter.parse(src).unwrap();
    b.iter(|| { typesetter.layout(&tree).unwrap(); });
}

/// Benchmarks the full typesetting step.
fn typesetting(b: &mut Bencher) {
    let (typesetter, src) = prepare();
    b.iter(|| { typesetter.typeset(src).unwrap(); });
}

/// Benchmarks only the exporting step.
fn exporting(b: &mut Bencher) {
    let (typesetter, src) = prepare();
    let doc = typesetter.typeset(src).unwrap();
    let exporter = PdfExporter::new();
    b.iter(|| {
        let mut buf = Vec::new();
        exporter.export(&doc, &typesetter.loader(), &mut buf).unwrap();
    });
}

bencher::benchmark_group!(benches, parsing, layouting, typesetting, exporting);
bencher::benchmark_main!(benches);
