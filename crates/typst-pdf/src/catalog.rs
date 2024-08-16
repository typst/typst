use std::num::NonZeroUsize;

use ecow::eco_format;
use pdf_writer::{
    types::Direction, writers::PageLabel, Finish, Name, Pdf, Ref, Str, TextStr,
};
use xmp_writer::{LangId, RenditionClass, XmpWriter};

use typst::foundations::{Datetime, Smart};
use typst::layout::Dir;
use typst::text::Lang;

use crate::WithEverything;
use crate::{hash_base64, outline, page::PdfPageLabel};

pub struct Timestamp {
    pub datetime: Datetime,
    // The local timezone minus the UTC, in hours and remaining minutes. If None,
    // the PDF timestamps will not have a timezone suffix.
    pub timezone_offset: Option<(i8, u8)>,
}

impl Timestamp {
    pub fn new(datetime: Datetime) -> Self {
        Self { datetime, timezone_offset: None }
    }
    pub fn with_timezone_offset(self, seconds: i32) -> Option<Self> {
        let hour: i8 = (seconds / 3600).try_into().ok()?;
        let minute: u8 = (seconds.abs() % 3600).try_into().ok()?;
        Some(Self { timezone_offset: Some((hour, minute)), ..self })
    }
}

/// Write the document catalog.
pub fn write_catalog(
    ctx: WithEverything,
    ident: Smart<&str>,
    timestamp: Option<Timestamp>,
    pdf: &mut Pdf,
    alloc: &mut Ref,
) {
    let lang = ctx
        .resources
        .languages
        .iter()
        .max_by_key(|(_, &count)| count)
        .map(|(&l, _)| l);

    let dir = if lang.map(Lang::dir) == Some(Dir::RTL) {
        Direction::R2L
    } else {
        Direction::L2R
    };

    // Write the outline tree.
    let outline_root_id = outline::write_outline(pdf, alloc, &ctx);

    // Write the page labels.
    let page_labels = write_page_labels(pdf, alloc, &ctx);

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

    if let Some((pdf_date, xmp_date)) = generate_writer_date(timestamp) {
        info.creation_date(pdf_date).modified_date(pdf_date);
        xmp.create_date(xmp_date).modify_date(xmp_date);
    }

    info.finish();
    xmp.num_pages(ctx.document.pages.len() as u32);
    xmp.format("application/pdf");
    xmp.language(ctx.resources.languages.keys().map(|lang| LangId(lang.as_str())));

    // A unique ID for this instance of the document. Changes if anything
    // changes in the frames.
    let instance_id = hash_base64(&pdf.as_bytes());

    // Determine the document's ID. It should be as stable as possible.
    const PDF_VERSION: &str = "PDF-1.7";
    let doc_id = if let Smart::Custom(ident) = ident {
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
    catalog.pages(ctx.page_tree_ref);
    catalog.viewer_preferences().direction(dir);
    catalog.metadata(meta_ref);

    // Write the named destination tree.
    let mut name_dict = catalog.names();
    let mut dests_name_tree = name_dict.destinations();
    let mut names = dests_name_tree.names();
    for &(name, dest_ref, ..) in &ctx.references.named_destinations.dests {
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

/// Write the page labels.
pub(crate) fn write_page_labels(
    chunk: &mut Pdf,
    alloc: &mut Ref,
    ctx: &WithEverything,
) -> Vec<(NonZeroUsize, Ref)> {
    // If there is no exported page labeled, we skip the writing
    if !ctx.pages.iter().filter_map(Option::as_ref).any(|p| {
        p.label
            .as_ref()
            .is_some_and(|l| l.prefix.is_some() || l.style.is_some())
    }) {
        return Vec::new();
    }

    let mut result = vec![];
    let empty_label = PdfPageLabel::default();
    let mut prev: Option<&PdfPageLabel> = None;

    // Skip non-exported pages for numbering.
    for (i, page) in ctx.pages.iter().filter_map(Option::as_ref).enumerate() {
        let nr = NonZeroUsize::new(1 + i).unwrap();
        // If there are pages with empty labels between labeled pages, we must
        // write empty PageLabel entries.
        let label = page.label.as_ref().unwrap_or(&empty_label);

        if let Some(pre) = prev {
            if label.prefix == pre.prefix
                && label.style == pre.style
                && label.offset == pre.offset.map(|n| n.saturating_add(1))
            {
                prev = Some(label);
                continue;
            }
        }

        let id = alloc.bump();
        let mut entry = chunk.indirect(id).start::<PageLabel>();

        // Only add what is actually provided. Don't add empty prefix string if
        // it wasn't given for example.
        if let Some(prefix) = &label.prefix {
            entry.prefix(TextStr(prefix));
        }

        if let Some(style) = label.style {
            entry.style(style.to_pdf_numbering_style());
        }

        if let Some(offset) = label.offset {
            entry.offset(offset.get() as i32);
        }

        result.push((nr, id));
        prev = Some(label);
    }

    result
}

/// Converts a timestamp to a pdf-writer date and a xmp-writer date.
fn generate_writer_date(
    timestamp: Option<Timestamp>,
) -> Option<(pdf_writer::Date, xmp_writer::DateTime)> {
    let Timestamp { datetime, timezone_offset } = timestamp?;
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

    if let Some((hour, minute)) = timezone_offset {
        pdf_date = pdf_date.utc_offset_hour(hour).utc_offset_minute(minute);
    }

    let xmp_date = xmp_writer::DateTime {
        year,
        month: datetime.month(),
        day: datetime.day(),
        hour: datetime.hour(),
        minute: datetime.minute(),
        second: datetime.second(),
        timezone: match timezone_offset {
            Some((0, 0)) => Some(xmp_writer::Timezone::Utc),
            Some((hour, minute)) => Some(xmp_writer::Timezone::Local {
                hour,
                minute: minute.try_into().ok()?,
            }),
            None => None,
        },
    };

    Some((pdf_date, xmp_date))
}
