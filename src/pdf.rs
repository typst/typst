//! Writing of documents in the _PDF_ format.

use std::io::{self, Write};
use crate::doc::Document;
use pdf::{PdfWriter, Id, Rect, Version, DocumentCatalog, PageTree,
          Page, PageData, Resource, font::Type1Font, Text, Trailer};


/// A type that is a sink for types that can be written conforming
/// to the _PDF_ format.
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

        let font_start = pages_end;
        let font_end = font_start + 1;

        let content_start = font_end;
        let content_end = content_start
            + doc.pages.iter().flat_map(|p| p.contents.iter()).count() as Id;

        writer.write_header(&Version::new(1, 7))?;

        // The document catalog
        writer.write_obj(catalog_id, &DocumentCatalog {
            page_tree: page_tree_id,
        })?;

        // Root page tree
        writer.write_obj(page_tree_id, &PageTree {
            parent: None,
            kids: (pages_start .. pages_end).collect(),
            data: PageData {
                resources: Some(vec![Resource::Font { nr: 1, id: font_start }]),
                .. PageData::none()
            },
        })?;

        // The page objects
        let mut id = pages_start;
        for page in &doc.pages {
            let width = page.size[0].to_points();
            let height = page.size[1].to_points();

            writer.write_obj(id, &Page {
                parent: page_tree_id,
                data: PageData {
                    media_box: Some(Rect::new(0.0, 0.0, width, height)),
                    contents: Some((content_start .. content_end).collect()),
                    .. PageData::none()
                },
            })?;

            id += 1;
        }

        // The resources, currently only one hardcoded font
        writer.write_obj(font_start, &Type1Font {
            base_font: "Helvetica".to_owned(),
        })?;

        // The page contents
        let mut id = content_start;
        for page in &doc.pages {
            for content in &page.contents {
                let string = &content.0;

                writer.write_obj(id, &Text::new()
                    .set_font(1, 13.0)
                    .move_pos(108.0, 734.0)
                    .write_text(&string)
                    .to_stream()
                )?;
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
