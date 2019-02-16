//! Writing of documents in the _PDF_ format.

use std::io::{self, Write};
use crate::doc::{Document, DocumentFont};
use pdf::{PdfWriter, Id, Rect, Size, Version, DocumentCatalog, PageTree,
          Page, PageData, Resource, font::Type1Font, Text, Trailer};


/// A type that is a sink for types that can be written conforming
/// to the _PDF_ format (that may be things like sizes, other objects
/// or whole documents).
pub trait WritePdf<T> {
    /// Write self into a byte sink, returning how many bytes were written.
    fn write_pdf(&mut self, object: &T) -> io::Result<usize>;
}

impl<W: Write> WritePdf<Document> for W {
    fn write_pdf(&mut self, doc: &Document) -> io::Result<usize> {
        let mut writer = PdfWriter::new(self);

        // Calculate unique id's for everything
        let catalog_id: Id = 1;

        let page_tree_id = catalog_id + 1;
        let pages_start = page_tree_id + 1;
        let pages_end = pages_start + doc.pages.len() as Id;

        let resources_start = pages_end;
        let font_start = resources_start;
        let font_end = font_start + doc.fonts.len() as Id;
        let resources_end = font_end;

        let content_start = resources_end;
        let content_end = content_start
            + doc.pages.iter().flat_map(|p| p.contents.iter()).count() as Id;

        writer.write_header(&Version::new(1, 7))?;

        // The document catalog
        writer.write_obj(catalog_id, &DocumentCatalog {
            page_tree: page_tree_id,
        })?;

        let font_resources: Vec<_> = (1 ..= doc.fonts.len() as u32)
            .zip(font_start .. font_end)
            .map(|(nr, id)| Resource::Font(nr, id)).collect();

        // Root page tree
        writer.write_obj(page_tree_id, &PageTree {
            parent: None,
            kids: (pages_start .. pages_end).collect(),
            data: PageData {
                resources: Some(font_resources),
                .. PageData::default()
            },
        })?;

        // The page objects
        let mut id = pages_start;
        for page in &doc.pages {
            let width = page.size[0].points;
            let height = page.size[1].points;

            writer.write_obj(id, &Page {
                parent: page_tree_id,
                data: PageData {
                    media_box: Some(Rect::new(0.0, 0.0, width, height)),
                    contents: Some((content_start .. content_end).collect()),
                    .. PageData::default()
                },
            })?;

            id += 1;
        }

        // The resources (fonts)
        let mut id = font_start;
        for font in &doc.fonts {
            match font {
                DocumentFont::Builtin(font) => {
                    writer.write_obj(id, &Type1Font {
                        base_font: font.name().to_owned(),
                    })?;
                },
                DocumentFont::Loaded(_) => unimplemented!(),
            }

            id += 1;
        }

        // The page contents
        let mut id = content_start;
        for page in &doc.pages {
            for content in &page.contents {
                let string = &content.0;

                let mut text = Text::new();
                text.set_font(1, 13.0)
                    .move_pos(108.0, 734.0)
                    .write_str(&string);

                writer.write_obj(id, &text.as_stream())?;
                id += 1;
            }
        }

        // Cross-reference table
        writer.write_xref_table()?;

        // Trailer
        writer.write_trailer(&Trailer {
            root: catalog_id,
        })?;

        Ok(writer.written())
    }
}


#[cfg(test)]
mod pdf_tests {
    use super::*;
    use crate::parsing::{Tokenize, Parse};
    use crate::doc::Generate;

    /// Create a pdf with a name from the source code.
    fn test(name: &str, src: &str) {
        let mut file = std::fs::File::create(name).unwrap();
        let doc = src.tokenize()
            .parse().unwrap()
            .generate().unwrap();
        file.write_pdf(&doc).unwrap();
    }

    #[test]
    fn pdf_simple() {
        test("../target/write1.pdf", "This is an example of a sentence.");
        test("../target/write2.pdf","
             Lorem ipsum dolor sit amet, consetetur sadipscing elitr, sed
             diam nonumy eirmod tempor invidunt ut labore et dolore magna aliquyam erat, sed diam
             voluptua. At vero eos et accusam et justo duo dolores et ea rebum. Stet clita kasd
             gubergren, no sea takimata sanctus est Lorem ipsum dolor sit amet. Lorem ipsum dolor
             sit amet, consetetur sadipscing elitr, sed diam nonumy eirmod tempor invidunt ut
             labore et dolore magna aliquyam erat, sed diam voluptua. At vero eos et accusam et
             justo duo dolores et ea rebum. Stet clita kasd gubergren, no sea takimata sanctus est
             Lorem ipsum dolor sit amet.
        ");
    }
}
