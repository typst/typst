//! Capabilities for Typst IDE support.

mod analyze;
mod complete;
mod jump;
mod tooltip;

pub use self::analyze::analyze_labels;
pub use self::complete::{autocomplete, Completion, CompletionKind};
pub use self::jump::{jump_from_click, jump_from_cursor, Jump};
pub use self::tooltip::{tooltip, Tooltip};

use std::fmt::Write;

use ecow::{eco_format, EcoString};
use typst::text::{FontInfo, FontStyle};

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
    use comemo::Prehashed;
    use once_cell::sync::Lazy;
    use typst::diag::{FileError, FileResult};
    use typst::foundations::{Bytes, Datetime};
    use typst::syntax::{FileId, Source};
    use typst::text::{Font, FontBook};
    use typst::{Library, World};

    /// A world for IDE testing.
    pub struct TestWorld {
        pub main: Source,
        base: &'static TestBase,
    }

    impl TestWorld {
        /// Create a new world for a single test.
        ///
        /// This is cheap because the shared base for all test runs is lazily
        /// initialized just once.
        pub fn new(text: &str) -> Self {
            static BASE: Lazy<TestBase> = Lazy::new(TestBase::default);
            let main = Source::detached(text);
            Self { main, base: &*BASE }
        }
    }

    impl World for TestWorld {
        fn library(&self) -> &Prehashed<Library> {
            &self.base.library
        }

        fn book(&self) -> &Prehashed<FontBook> {
            &self.base.book
        }

        fn main(&self) -> Source {
            self.main.clone()
        }

        fn source(&self, id: FileId) -> FileResult<Source> {
            if id == self.main.id() {
                Ok(self.main.clone())
            } else {
                Err(FileError::NotFound(id.vpath().as_rootless_path().into()))
            }
        }

        fn file(&self, id: FileId) -> FileResult<Bytes> {
            Err(FileError::NotFound(id.vpath().as_rootless_path().into()))
        }

        fn font(&self, index: usize) -> Option<Font> {
            Some(self.base.fonts[index].clone())
        }

        fn today(&self, _: Option<i64>) -> Option<Datetime> {
            None
        }
    }

    /// Shared foundation of all test worlds.
    struct TestBase {
        library: Prehashed<Library>,
        book: Prehashed<FontBook>,
        fonts: Vec<Font>,
    }

    impl Default for TestBase {
        fn default() -> Self {
            let fonts: Vec<_> = typst_assets::fonts()
                .chain(typst_dev_assets::fonts())
                .flat_map(|data| Font::iter(Bytes::from_static(data)))
                .collect();

            Self {
                library: Prehashed::new(Library::default()),
                book: Prehashed::new(FontBook::from_fonts(&fonts)),
                fonts,
            }
        }
    }
}
