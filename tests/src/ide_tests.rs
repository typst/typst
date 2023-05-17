use std::path::Path;

use comemo::Prehashed;
use typst::{
    diag::FileError,
    eval::Library,
    font::FontBook,
    syntax::{Source, SourceId},
    World,
};

struct TestWorld {
    library: Prehashed<Library>,
    fonts: Prehashed<FontBook>,
    main_src: Source,
}

impl TestWorld {
    fn new(src: String) -> Self {
        Self {
            library: Prehashed::new(typst_library::build()),
            fonts: Prehashed::new(FontBook::new()),
            main_src: Source::new(SourceId::detached(), Path::new("main.typ"), src),
        }
    }
}

impl World for TestWorld {
    fn library(&self) -> &Prehashed<Library> {
        &self.library
    }

    fn main(&self) -> &Source {
        &self.main_src
    }

    fn resolve(
        &self,
        path: &std::path::Path,
    ) -> typst::diag::FileResult<typst::syntax::SourceId> {
        Err(FileError::NotFound(path.to_path_buf()))
    }

    fn source(&self, _id: typst::syntax::SourceId) -> &Source {
        &self.main_src
    }

    fn book(&self) -> &Prehashed<FontBook> {
        &self.fonts
    }

    fn font(&self, _id: usize) -> Option<typst::font::Font> {
        None
    }

    fn file(
        &self,
        path: &std::path::Path,
    ) -> typst::diag::FileResult<typst::util::Buffer> {
        Err(FileError::NotFound(path.to_path_buf()))
    }
}

#[test]
fn invalid_let() {
    // Make sure that generating a tooltip for the below code does not crash the compiler
    const SRC: &str = r"#let 5 = 6";

    let world = TestWorld::new(SRC.to_string());
    let source = world.main();

    let cursor = 2;

    typst::ide::tooltip(&world, &[], source, cursor);
}
