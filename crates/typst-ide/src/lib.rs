//! Capabilities for Typst IDE support.

mod analyze;
mod complete;
mod definition;
mod jump;
mod matchers;
mod tooltip;

pub use self::analyze::{analyze_expr, analyze_import, analyze_labels};
pub use self::complete::{autocomplete, Completion, CompletionKind};
pub use self::definition::{definition, Definition, DefinitionKind};
pub use self::jump::{jump_from_click, jump_from_cursor, Jump};
pub use self::matchers::{deref_target, named_items, DerefTarget, NamedItem};
pub use self::tooltip::{tooltip, Tooltip};

use std::fmt::Write;

use ecow::{eco_format, EcoString};
use typst::syntax::package::PackageSpec;
use typst::text::{FontInfo, FontStyle};
use typst::World;

/// Extends the `World` for IDE functionality.
pub trait IdeWorld: World {
    /// Turn into a normal [`World`].
    ///
    /// This is necessary because trait upcasting is experimental in Rust.
    /// See: https://github.com/rust-lang/rust/issues/65991
    ///
    /// Implementors can simply return `self`.
    fn upcast(&self) -> &dyn World;

    /// A list of all available packages and optionally descriptions for them.
    ///
    /// This function is **optional** to implement. It enhances the user
    /// experience by enabling autocompletion for packages. Details about
    /// packages from the `@preview` namespace are available from
    /// `https://packages.typst.org/preview/index.json`.
    fn packages(&self) -> &[(PackageSpec, Option<EcoString>)] {
        &[]
    }
}

/// Extract the first sentence of plain text of a piece of documentation.
///
/// Removes Markdown formatting.
fn plain_docs_sentence(docs: &str) -> EcoString {
    let mut s = unscanny::Scanner::new(docs);
    let mut output = EcoString::new();
    let mut link = false;
    while let Some(c) = s.eat() {
        match c {
            '`' => {
                let mut raw = s.eat_until('`');
                if (raw.starts_with('{') && raw.ends_with('}'))
                    || (raw.starts_with('[') && raw.ends_with(']'))
                {
                    raw = &raw[1..raw.len() - 1];
                }

                s.eat();
                output.push('`');
                output.push_str(raw);
                output.push('`');
            }
            '[' => link = true,
            ']' if link => {
                if s.eat_if('(') {
                    s.eat_until(')');
                    s.eat();
                } else if s.eat_if('[') {
                    s.eat_until(']');
                    s.eat();
                }
                link = false
            }
            '*' | '_' => {}
            '.' => {
                output.push('.');
                break;
            }
            _ => output.push(c),
        }
    }

    output
}

/// Create a short description of a font family.
fn summarize_font_family<'a>(variants: impl Iterator<Item = &'a FontInfo>) -> EcoString {
    let mut infos: Vec<_> = variants.collect();
    infos.sort_by_key(|info| info.variant);

    let mut has_italic = false;
    let mut min_weight = u16::MAX;
    let mut max_weight = 0;
    for info in &infos {
        let weight = info.variant.weight.to_number();
        has_italic |= info.variant.style == FontStyle::Italic;
        min_weight = min_weight.min(weight);
        max_weight = min_weight.max(weight);
    }

    let count = infos.len();
    let mut detail = eco_format!("{count} variant{}.", if count == 1 { "" } else { "s" });

    if min_weight == max_weight {
        write!(detail, " Weight {min_weight}.").unwrap();
    } else {
        write!(detail, " Weights {min_weight}–{max_weight}.").unwrap();
    }

    if has_italic {
        detail.push_str(" Has italics.");
    }

    detail
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use typst::diag::{FileError, FileResult};
    use typst::foundations::{Bytes, Datetime, Smart};
    use typst::layout::{Abs, Margin, PageElem};
    use typst::syntax::{FileId, Source, VirtualPath};
    use typst::text::{Font, FontBook, TextElem, TextSize};
    use typst::utils::{singleton, LazyHash};
    use typst::{Library, World};

    use crate::IdeWorld;

    /// A world for IDE testing.
    pub struct TestWorld {
        pub main: Source,
        assets: HashMap<FileId, Bytes>,
        sources: HashMap<FileId, Source>,
        base: &'static TestBase,
    }

    impl TestWorld {
        /// Create a new world for a single test.
        ///
        /// This is cheap because the shared base for all test runs is lazily
        /// initialized just once.
        pub fn new(text: &str) -> Self {
            let main = Source::new(Self::main_id(), text.into());
            Self {
                main,
                assets: HashMap::new(),
                sources: HashMap::new(),
                base: singleton!(TestBase, TestBase::default()),
            }
        }

        /// Add an additional asset file to the test world.
        #[track_caller]
        pub fn with_asset_by_name(mut self, filename: &str) -> Self {
            let id = FileId::new(None, VirtualPath::new(filename));
            let data = typst_dev_assets::get_by_name(filename).unwrap();
            let bytes = Bytes::from_static(data);
            self.assets.insert(id, bytes);
            self
        }

        /// Add an additional source file to the test world.
        pub fn with_source(mut self, path: &str, text: &str) -> Self {
            let id = FileId::new(None, VirtualPath::new(path));
            let source = Source::new(id, text.into());
            self.sources.insert(id, source);
            self
        }

        /// The ID of the main file in a `TestWorld`.
        pub fn main_id() -> FileId {
            *singleton!(FileId, FileId::new(None, VirtualPath::new("main.typ")))
        }
    }

    impl World for TestWorld {
        fn library(&self) -> &LazyHash<Library> {
            &self.base.library
        }

        fn book(&self) -> &LazyHash<FontBook> {
            &self.base.book
        }

        fn main(&self) -> FileId {
            self.main.id()
        }

        fn source(&self, id: FileId) -> FileResult<Source> {
            if id == self.main.id() {
                Ok(self.main.clone())
            } else if let Some(source) = self.sources.get(&id) {
                Ok(source.clone())
            } else {
                Err(FileError::NotFound(id.vpath().as_rootless_path().into()))
            }
        }

        fn file(&self, id: FileId) -> FileResult<Bytes> {
            match self.assets.get(&id) {
                Some(bytes) => Ok(bytes.clone()),
                None => Err(FileError::NotFound(id.vpath().as_rootless_path().into())),
            }
        }

        fn font(&self, index: usize) -> Option<Font> {
            Some(self.base.fonts[index].clone())
        }

        fn today(&self, _: Option<i64>) -> Option<Datetime> {
            None
        }
    }

    impl IdeWorld for TestWorld {
        fn upcast(&self) -> &dyn World {
            self
        }
    }

    /// Extra methods for [`Source`].
    pub trait SourceExt {
        /// Negative cursors index from the back.
        fn cursor(&self, cursor: isize) -> usize;
    }

    impl SourceExt for Source {
        fn cursor(&self, cursor: isize) -> usize {
            if cursor < 0 {
                self.len_bytes().checked_add_signed(cursor).unwrap()
            } else {
                cursor as usize
            }
        }
    }

    /// Shared foundation of all test worlds.
    struct TestBase {
        library: LazyHash<Library>,
        book: LazyHash<FontBook>,
        fonts: Vec<Font>,
    }

    impl Default for TestBase {
        fn default() -> Self {
            let fonts: Vec<_> = typst_assets::fonts()
                .chain(typst_dev_assets::fonts())
                .flat_map(|data| Font::iter(Bytes::from_static(data)))
                .collect();

            Self {
                library: LazyHash::new(library()),
                book: LazyHash::new(FontBook::from_fonts(&fonts)),
                fonts,
            }
        }
    }

    /// The extended standard library for testing.
    fn library() -> Library {
        // Set page width to 120pt with 10pt margins, so that the inner page is
        // exactly 100pt wide. Page height is unbounded and font size is 10pt so
        // that it multiplies to nice round numbers.
        let mut lib = typst::Library::default();
        lib.styles
            .set(PageElem::set_width(Smart::Custom(Abs::pt(120.0).into())));
        lib.styles.set(PageElem::set_height(Smart::Auto));
        lib.styles.set(PageElem::set_margin(Margin::splat(Some(Smart::Custom(
            Abs::pt(10.0).into(),
        )))));
        lib.styles.set(TextElem::set_size(TextSize(Abs::pt(10.0).into())));
        lib
    }
}
