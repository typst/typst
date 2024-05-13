use ecow::eco_format;
use pdf_writer::{types::Direction, Finish, Name, Pdf, Ref, Str, TextStr};
use typst::{
    foundations::{Datetime, Smart},
    layout::Dir,
    text::Lang,
};
use xmp_writer::{DateTime, LangId, RenditionClass, Timezone, XmpWriter};

use crate::{hash_base64, outline, page, ConstructContext, PdfWriter, WriteContext};

pub struct Catalog<'a> {
    pub ident: Smart<&'a str>,
    pub timestamp: Option<Datetime>,
}

impl<'a> PdfWriter for Catalog<'a> {
    /// Write the document catalog.

    fn write(
        &self,
        pdf: &mut Pdf,
        alloc: &mut Ref,
        ctx: &ConstructContext,
        refs: &WriteContext,
    ) {
        let lang = ctx.languages.iter().max_by_key(|(_, &count)| count).map(|(&l, _)| l);

        let dir = if lang.map(Lang::dir) == Some(Dir::RTL) {
            Direction::R2L
        } else {
            Direction::L2R
        };

        // Write the outline tree.
        let outline_root_id = outline::write_outline(pdf, alloc, ctx);

        // Write the page labels.
        let page_labels = page::write_page_labels(pdf, alloc, ctx);

        // Write the document information.
        let info_ref = alloc.bump();
        let mut info = pdf.document_info(info_ref);
        let mut xmp = XmpWriter::new();
        if let Some(title) = &ctx.document.title {
            info.title(TextStr(title));
            xmp.title([(None, title.as_str())]);
        }

        let authors = &ctx.document.author;
        if !authors.is_empty() {
            // Turns out that if the authors are given in both the document
            // information dictionary and the XMP metadata, Acrobat takes a little
            // bit of both: The first author from the document information
            // dictionary and the remaining authors from the XMP metadata.
            //
            // To fix this for Acrobat, we could omit the remaining authors or all
            // metadata from the document information catalog (it is optional) and
            // only write XMP. However, not all other tools (including Apple
            // Preview) read the XMP data. This means we do want to include all
            // authors in the document information dictionary.
            //
            // Thus, the only alternative is to fold all authors into a single
            // `<rdf:li>` in the XMP metadata. This is, in fact, exactly what the
            // PDF/A spec Part 1 section 6.7.3 has to say about the matter. It's a
            // bit weird to not use the array (and it makes Acrobat show the author
            // list in quotes), but there's not much we can do about that.
            let joined = authors.join(", ");
            info.author(TextStr(&joined));
            xmp.creator([joined.as_str()]);
        }

        let creator = eco_format!("Typst {}", env!("CARGO_PKG_VERSION"));
        info.creator(TextStr(&creator));
        xmp.creator_tool(&creator);

        let keywords = &ctx.document.keywords;
        if !keywords.is_empty() {
            let joined = keywords.join(", ");
            info.keywords(TextStr(&joined));
            xmp.pdf_keywords(&joined);
        }

        if let Some(date) = ctx.document.date.unwrap_or(self.timestamp) {
            let tz = ctx.document.date.is_auto();
            if let Some(pdf_date) = pdf_date(date, tz) {
                info.creation_date(pdf_date);
                info.modified_date(pdf_date);
            }
            if let Some(xmp_date) = xmp_date(date, tz) {
                xmp.create_date(xmp_date);
                xmp.modify_date(xmp_date);
            }
        }

        info.finish();
        xmp.num_pages(ctx.document.pages.len() as u32);
        xmp.format("application/pdf");
        xmp.language(ctx.languages.keys().map(|lang| LangId(lang.as_str())));

        // A unique ID for this instance of the document. Changes if anything
        // changes in the frames.
        let instance_id = hash_base64(&pdf.as_bytes());

        // Determine the document's ID. It should be as stable as possible.
        const PDF_VERSION: &str = "PDF-1.7";
        let doc_id = if let Smart::Custom(ident) = self.ident {
            // We were provided with a stable ID. Yay!
            hash_base64(&(PDF_VERSION, ident))
        } else if ctx.document.title.is_some() && !ctx.document.author.is_empty() {
            // If not provided from the outside, but title and author were given, we
            // compute a hash of them, which should be reasonably stable and unique.
            hash_base64(&(PDF_VERSION, &ctx.document.title, &ctx.document.author))
        } else {
            // The user provided no usable metadata which we can use as an `/ID`.
            instance_id.clone()
        };

        // Write IDs.
        xmp.document_id(&doc_id);
        xmp.instance_id(&instance_id);
        pdf.set_file_id((doc_id.clone().into_bytes(), instance_id.into_bytes()));

        xmp.rendition_class(RenditionClass::Proof);
        xmp.pdf_version("1.7");

        let xmp_buf = xmp.finish(None);
        let meta_ref = alloc.bump();
        pdf.stream(meta_ref, xmp_buf.as_bytes())
            .pair(Name(b"Type"), Name(b"Metadata"))
            .pair(Name(b"Subtype"), Name(b"XML"));

        // Write the document catalog.
        let catalog_ref = alloc.bump();
        let mut catalog = pdf.catalog(catalog_ref);
        catalog.pages(ctx.globals.page_tree);
        catalog.viewer_preferences().direction(dir);
        catalog.metadata(meta_ref);

        // Write the named destination tree.
        let mut name_dict = catalog.names();
        let mut dests_name_tree = name_dict.destinations();
        let mut names = dests_name_tree.names();
        for &(name, dest_ref, ..) in &refs.dests {
            names.insert(Str(name.as_str().as_bytes()), dest_ref);
        }
        names.finish();
        dests_name_tree.finish();
        name_dict.finish();

        // Insert the page labels.
        if !page_labels.is_empty() {
            let mut num_tree = catalog.page_labels();
            let mut entries = num_tree.nums();
            for (n, r) in &page_labels {
                entries.insert(n.get() as i32 - 1, *r);
            }
        }

        if let Some(outline_root_id) = outline_root_id {
            catalog.outlines(outline_root_id);
        }

        if let Some(lang) = lang {
            catalog.lang(TextStr(lang.as_str()));
        }

        catalog.finish();
    }
}

/// Converts a datetime to a pdf-writer date.
fn pdf_date(datetime: Datetime, tz: bool) -> Option<pdf_writer::Date> {
    let year = datetime.year().filter(|&y| y >= 0)? as u16;

    let mut pdf_date = pdf_writer::Date::new(year);

    if let Some(month) = datetime.month() {
        pdf_date = pdf_date.month(month);
    }

    if let Some(day) = datetime.day() {
        pdf_date = pdf_date.day(day);
    }

    if let Some(h) = datetime.hour() {
        pdf_date = pdf_date.hour(h);
    }

    if let Some(m) = datetime.minute() {
        pdf_date = pdf_date.minute(m);
    }

    if let Some(s) = datetime.second() {
        pdf_date = pdf_date.second(s);
    }

    if tz {
        pdf_date = pdf_date.utc_offset_hour(0).utc_offset_minute(0);
    }

    Some(pdf_date)
}

/// Converts a datetime to an xmp-writer datetime.
fn xmp_date(datetime: Datetime, tz: bool) -> Option<xmp_writer::DateTime> {
    let year = datetime.year().filter(|&y| y >= 0)? as u16;
    Some(DateTime {
        year,
        month: datetime.month(),
        day: datetime.day(),
        hour: datetime.hour(),
        minute: datetime.minute(),
        second: datetime.second(),
        timezone: if tz { Some(Timezone::Utc) } else { None },
    })
}
