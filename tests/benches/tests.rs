use base64::Engine;
use iai_callgrind::{library_benchmark, library_benchmark_group, main};
use typst::syntax::{FileId, Source, VirtualPath};
use typst_tests::world::TestWorld;

fn benchmark_setup(line: String) -> TestWorld {
    let source = String::from_utf8(
        base64::engine::general_purpose::STANDARD
            .decode(line.split_ascii_whitespace().nth(1).unwrap())
            .unwrap(),
    )
    .unwrap();
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
main!(library_benchmark_groups = main_group);
