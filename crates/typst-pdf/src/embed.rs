use crate::catalog::{document_date, pdf_date};
use crate::{PdfChunk, WithGlobalRefs};
use ecow::EcoString;
use pdf_writer::{Finish, Name, Ref, Str, TextStr};
use std::collections::HashMap;
use typst_library::diag::{bail, SourceResult};
use typst_library::foundations::NativeElement;
use typst_library::pdf::embed::{Embed, EmbedElem};

/// Query for all [`EmbedElem`] and write them and their file specifications.
///
/// This returns a map of embedding names and references so that we can later add them to the
/// catalog's name dictionary.
pub fn write_embedded_files(
    ctx: &WithGlobalRefs,
) -> SourceResult<(PdfChunk, HashMap<EcoString, Ref>)> {
    let mut chunk = PdfChunk::new();

    let elements = ctx.document.introspector.query(&EmbedElem::elem().select());
    if !ctx.options.standards.embedded_files {
        if let Some(element) = elements.first() {
            bail!(
                element.span(),
                "file embeddings are currently only supported for PDF/A-3"
            );
        }
    }

    let mut embedded_files = HashMap::default();
    for elem in elements.iter() {
        let packed_elem = elem.to_packed::<EmbedElem>().unwrap();
        let embed = Embed::from_element(packed_elem);
        embedded_files.insert(embed.name().clone(), embed_file(ctx, &mut chunk, &embed));
    }

    Ok((chunk, embedded_files))
}

/// Write the embedded file stream and its file specification.
fn embed_file(ctx: &WithGlobalRefs, chunk: &mut PdfChunk, embed: &Embed) -> Ref {
    let embedded_file_stream_ref = chunk.alloc.bump();
    let file_spec_dict_ref = chunk.alloc.bump();

    let length = embed.data().len();

    let mut embedded_file =
        chunk.embedded_file(embedded_file_stream_ref, embed.data().as_ref());
    embedded_file.pair(Name(b"Length"), length as i32);
    if let Some(mime_type) = embed.mime_type() {
        embedded_file.subtype(Name(mime_type.as_bytes()));
    }
    let (date, tz) = document_date(ctx.document.info.date, ctx.options.timestamp);
    if let Some(pdf_date) = date.and_then(|date| pdf_date(date, tz)) {
        embedded_file.params().modification_date(pdf_date).finish();
    }
    embedded_file.finish();

    let mut file_spec = chunk.file_spec(file_spec_dict_ref);
    file_spec
        .path(Str(embed.path().as_bytes()))
        .unic_file(TextStr(embed.path().as_str()))
        .insert(Name(b"EF"))
        .dict()
        .pair(Name(b"F"), embedded_file_stream_ref)
        .pair(Name(b"UF"), embedded_file_stream_ref);
    if let Some(relationship) = embed.relationship() {
        if ctx.options.standards.pdfa {
            let name = relationship.name();
            // PDF 2.0, but ISO 19005-3 (PDF/A-3) Annex E allows it for PDF/A-3
            file_spec.pair(Name(b"AFRelationship"), Name(name.as_bytes()));
        }
    }
    if let Some(description) = embed.description() {
        file_spec.description(TextStr(description));
    }
    file_spec.finish();

    file_spec_dict_ref
}
