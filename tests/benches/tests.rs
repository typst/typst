use base64::Engine;
use ecow::EcoString;
use iai_callgrind::{
    library_benchmark, library_benchmark_group, main, LibraryBenchmarkConfig,
};
use typst::syntax::{FileId, Source, VirtualPath};
use typst_tests::world::TestWorld;

fn source_from_line(line: &str) -> Result<String, EcoString> {
    let Some(content) = line.split_ascii_whitespace().nth(1) else {
        return Ok(String::new());
    };
    String::from_utf8(
        base64::engine::general_purpose::STANDARD
            .decode(content)
            .map_err::<EcoString, _>(|e| typst::diag::error!("Invalid base64: {e}"))?,
    )
    .map_err::<EcoString, _>(|e| {
        typst::diag::error!("Invalid UTF-8 of decoded content: {e}")
    })
}

fn benchmark_setup(line: String) -> TestWorld {
    let source =
        source_from_line(&line).unwrap_or_else(|e| panic!("Malformed line {line}: {e}"));
    TestWorld::new(Source::new(FileId::new_fake(VirtualPath::new("stdin")), source))
}

#[library_benchmark]
#[benches::my_id(file = "tests/store/fixtures", setup = benchmark_setup)]
fn benchmark_function(world: TestWorld) {
    let output = typst::compile(&world);
    let _ = std::hint::black_box(output);
}

library_benchmark_group!(
    name = main_group;
    benchmarks = benchmark_function
);

main!(
    config = LibraryBenchmarkConfig::default().truncate_description(Some(100));
    library_benchmark_groups = main_group
);
