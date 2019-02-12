//! Writing of documents in the _PDF_ format.

use std::io::{self, Write};
use crate::doc::{Document, Text, DocumentFont, Size};


/// A type that is a sink for types that can be written conforming
/// to the _PDF_ format (that may be things like sizes, other objects
/// or whole documents).
pub trait WritePdf<T> {
    /// Write self into a byte sink, returning how many bytes were written.
    fn write_pdf(&mut self, object: &T) -> io::Result<usize>;
}

impl<W: Write> WritePdf<Document> for W {
    fn write_pdf(&mut self, document: &Document) -> io::Result<usize> {
        PdfWriter::new(document).write(self)
    }
}

impl<W: Write> WritePdf<Size> for W {
    fn write_pdf(&mut self, size: &Size) -> io::Result<usize> {
        self.write_str(size.points)
    }
}

/// A type that is a sink for types that can be converted to strings
/// and thus can be written string-like into a byte sink.
pub trait WriteByteString {
    /// Write the string-like type into self, returning how many
    /// bytes were written.
    fn write_str<S: ToString>(&mut self, string_like: S) -> io::Result<usize>;
}

impl<W: Write> WriteByteString for W {
    fn write_str<S: ToString>(&mut self, string_like: S) -> io::Result<usize> {
        self.write(string_like.to_string().as_bytes())
    }
}


/// Writes an abstract document into a byte sink in the _PDF_ format.
#[derive(Debug, Clone)]
struct PdfWriter<'d> {
    doc: &'d Document,
    w: usize,
    catalog_id: u32,
    page_tree_id: u32,
    resources_start: u32,
    pages_start: u32,
    content_start: u32,
    xref_table: Vec<u32>,
    offset_xref: u32,
}

impl<'d> PdfWriter<'d> {
    /// Create a new pdf writer from a document.
    fn new(doc: &'d Document) -> PdfWriter<'d> {
        // Calculate unique ids for each object
        let catalog_id: u32 = 1;
        let page_tree_id = catalog_id + 1;
        let pages_start = page_tree_id + 1;
        let resources_start = pages_start + doc.pages.len() as u32;
        let content_start = resources_start + doc.fonts.len() as u32;

        PdfWriter {
            doc,
            catalog_id,
            page_tree_id,
            resources_start,
            pages_start,
            content_start,
            w: 0,
            xref_table: vec![],
            offset_xref: 0,
        }
    }

    /// Write the document into a byte sink.
    fn write<W: Write>(&mut self, target: &mut W) -> io::Result<usize> {
        self.write_header(target)?;

        self.write_document_catalog(target)?;
        self.write_page_tree(target)?;
        self.write_pages(target)?;

        self.write_resources(target)?;

        self.write_content(target)?;
        // self.write_fonts(target)?;

        self.write_xref_table(target)?;
        self.write_trailer(target)?;
        self.write_start_xref(target)?;

        Ok(self.w)
    }

    /// Write the pdf header.
    fn write_header<W: Write>(&mut self, target: &mut W) -> io::Result<usize> {
        // Write the magic start
        self.w += target.write(b"%PDF-1.7\n")?;
        Ok(self.w)
    }

    /// Write the document catalog (contains general info about the document).
    fn write_document_catalog<W: Write>(&mut self, target: &mut W) -> io::Result<usize> {
        self.xref_table.push(self.w as u32);

        self.w += target.write_str(self.catalog_id)?;
        self.w += target.write(b" 0 obj\n")?;
        self.w += target.write(b"<<\n")?;
        self.w += target.write(b"/Type /Catalog\n")?;

        self.w += target.write(b"/Pages ")?;
        self.w += target.write_str(self.page_tree_id)?;
        self.w += target.write(b" 0 R\n")?;

        self.w += target.write(b">>\n")?;
        self.w += target.write(b"endobj\n")?;

        Ok(self.w)
    }

    /// Write the page tree (overview over the pages of a document).
    fn write_page_tree<W: Write>(&mut self, target: &mut W) -> io::Result<usize> {
        self.xref_table.push(self.w as u32);

        // Create page tree
        self.w += target.write_str(self.page_tree_id)?;
        self.w += target.write(b" 0 obj\n")?;
        self.w += target.write(b"<<\n")?;
        self.w += target.write(b"/Type /Pages\n")?;

        self.w += target.write(b"/Count ")?;
        self.w += target.write_str(self.doc.pages.len())?;
        self.w += target.write(b"\n")?;

        self.w += target.write(b"/Kids [")?;

        for id in self.pages_start .. self.pages_start + self.doc.pages.len() as u32 {
            self.w += target.write_str(id)?;
            self.w += target.write(b" 0 R ")?;
        }

        self.w += target.write(b"]\n")?;

        self.w += target.write(b"/Resources\n")?;
        self.w += target.write(b"<<\n")?;

        self.w += target.write(b"/Font\n")?;
        self.w += target.write(b"<<\n")?;

        let mut font_id = self.resources_start;
        for nr in 1 ..= self.doc.fonts.len() as u32 {
            self.w += target.write(b"/F")?;
            self.w += target.write_str(nr)?;
            self.w += target.write(b" ")?;
            self.w += target.write_str(font_id)?;
            self.w += target.write(b" 0 R\n")?;
            font_id += 1;
        }

        self.w += target.write(b">>\n")?;
        self.w += target.write(b">>\n")?;

        self.w += target.write(b">>\n")?;
        self.w += target.write(b"endobj\n")?;

        Ok(self.w)
    }

    /// Write the page descriptions.
    fn write_pages<W: Write>(&mut self, target: &mut W) -> io::Result<usize> {
        let mut page_id = self.pages_start;
        let mut content_id = self.content_start;

        for page in &self.doc.pages {
            self.xref_table.push(self.w as u32);

            self.w += target.write_str(page_id)?;
            self.w += target.write(b" 0 obj\n")?;
            self.w += target.write(b"<<\n")?;
            self.w += target.write(b"/Type /Page\n")?;

            self.w += target.write(b"/Parent ")?;
            self.w += target.write_str(self.page_tree_id)?;
            self.w += target.write(b" 0 R\n")?;

            self.w += target.write(b"/MediaBox [0 0 ")?;
            self.w += target.write_pdf(&page.size[0])?;
            self.w += target.write(b" ")?;
            self.w += target.write_pdf(&page.size[1])?;
            self.w += target.write(b"]\n")?;

            self.w += target.write(b"/Contents [")?;

            for _ in &page.contents {
                self.w += target.write_str(content_id)?;
                self.w += target.write(b" 0 R ")?;

                content_id += 1;
            }

            self.w += target.write(b"]\n")?;

            self.w += target.write(b">>\n")?;
            self.w += target.write(b"endobj\n")?;

            page_id += 1;
        }

        Ok(self.w)
    }

    /// Write the resources used by the file (fonts and friends).
    fn write_resources<W: Write>(&mut self, target: &mut W) -> io::Result<usize> {
        let mut id = self.resources_start;

        for font in &self.doc.fonts {
            self.xref_table.push(self.w as u32);

            self.w += target.write_str(id)?;
            self.w += target.write(b" 0 obj\n")?;
            self.w += target.write(b"<<\n")?;
            self.w += target.write(b"/Type /Font\n")?;

            match font {
                DocumentFont::Builtin(builtin) => {
                    self.w += target.write(b"/Subtype /Type1\n")?;
                    self.w += target.write(b"/BaseFont /")?;
                    self.w += target.write_str(builtin.name())?;
                    self.w += target.write(b"\n")?;
                },
                DocumentFont::Loaded(font) => {
                    self.w += target.write(b"/Subtype /TrueType\n")?;
                    self.w += target.write(b"/BaseFont /")?;
                    self.w += target.write_str(font.name.as_str())?;
                    self.w += target.write(b"\n")?;
                    unimplemented!();
                },
            }

            self.w += target.write(b">>\n")?;
            self.w += target.write(b"endobj\n")?;

            id += 1;
        }

        Ok(self.w)
    }

    /// Write the page contents.
    fn write_content<W: Write>(&mut self, target: &mut W) -> io::Result<usize> {
        let mut id = self.content_start;

        for page in &self.doc.pages {
            for content in &page.contents {
                self.xref_table.push(self.w as u32);

                self.w += target.write_str(id)?;
                self.w += target.write(b" 0 obj\n")?;
                self.w += target.write(b"<<\n")?;

                let mut buffer = Vec::new();
                    buffer.write(b"BT/\n")?;

                    buffer.write(b"/F1 13 Tf\n")?;
                    buffer.write(b"108 734 Td\n")?;
                    buffer.write(b"(")?;

                    let Text(string) = content;
                    buffer.write(string.as_bytes())?;

                    buffer.write(b") Tj\n")?;
                    buffer.write(b"ET\n")?;

                self.w += target.write(b"/Length ")?;
                self.w += target.write_str(buffer.len())?;
                self.w += target.write(b"\n")?;

                self.w += target.write(b">>\n")?;

                self.w += target.write(b"stream\n")?;
                self.w += target.write(&buffer)?;
                self.w += target.write(b"endstream\n")?;

                self.w += target.write(b"endobj\n")?;

                id += 1;
            }
        }

        Ok(self.w)
    }

    /// Write the cross-reference table.
    fn write_xref_table<W: Write>(&mut self, target: &mut W) -> io::Result<usize> {
        self.offset_xref = self.w as u32;

        self.w += target.write(b"xref\n")?;
        self.w += target.write(b"0 ")?;
        self.w += target.write_str(self.xref_table.len())?;
        self.w += target.write(b"\n")?;

        self.w += target.write(b"0000000000 65535 f\r\n")?;

        for offset in &self.xref_table {
            self.w += target.write(format!("{:010}", offset).as_bytes())?;
            self.w += target.write(b" 00000 n")?;
            self.w += target.write(b"\r\n")?;
        }

        Ok(self.w)
    }

    /// Write the trailer (points to the root object).
    fn write_trailer<W: Write>(&mut self, target: &mut W) -> io::Result<usize> {
        self.w += target.write(b"trailer\n")?;
        self.w += target.write(b"<<\n")?;

        self.w += target.write(b"/Root ")?;
        self.w += target.write_str(self.catalog_id)?;
        self.w += target.write(b" 0 R\n")?;

        self.w += target.write(b"/Size ")?;
        self.w += target.write_str(self.xref_table.len() + 1)?;
        self.w += target.write(b"\n")?;

        self.w += target.write(b">>\n")?;

        Ok(self.w)
    }

    /// Write where the cross-reference table starts.
    fn write_start_xref<W: Write>(&mut self, target: &mut W) -> io::Result<usize> {
        self.w += target.write(b"startxref\n")?;
        self.w += target.write_str(self.offset_xref)?;
        self.w += target.write(b"\n")?;

        Ok(self.w)
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
