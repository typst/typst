use crate::{PdfAConformanceLevel, PdfChunk, WithGlobalRefs};
use ecow::EcoString;
use pdf_writer::{Date, Finish, Name, Ref, Str, TextStr};
use std::collections::HashMap;
use typst::diag::SourceResult;

pub fn write_embedded_files(
    ctx: &WithGlobalRefs,
) -> SourceResult<(PdfChunk, HashMap<EcoString, Ref>)> {
    let mut chunk = PdfChunk::new();
    let mut embedded_files = HashMap::default();
    for embed in &ctx.resources.embeds {
        let embedded_file_stream_ref = chunk.alloc.bump();
        let file_spec_dict_ref = chunk.alloc.bump();

        let length = embed.data().len();
        let mut embedded_file =
            chunk.embedded_file(embedded_file_stream_ref, embed.data().as_ref());
        embedded_file
            .subtype(Name(b"text/xml"))
            .pair(Name(b"Length"), length as i32);
        embedded_file.params().modification_date(Date::new(2023)).finish(); // Todo: can we just let this out?
        embedded_file.finish();

        let mut file_spec = chunk.file_spec(file_spec_dict_ref);
        file_spec
            .path(Str(embed.path().as_bytes()))
            .unic_file(TextStr(embed.path().as_str()));
        if Some(PdfAConformanceLevel::A_3) == ctx.options.standards.pdfa {
            // PDF 2.0, but ISO 19005-3 (PDF/A-3) Annex E allows it for PDF/A-3
            file_spec.pair(Name(b"AFRelationship"), Name(b"Data"));
        }
        if let Some(description) = embed.description() {
            file_spec.description(TextStr(description));
        }
        file_spec
            .insert(Name(b"EF"))
            .dict()
            .pair(Name(b"F"), embedded_file_stream_ref)
            .pair(Name(b"UF"), embedded_file_stream_ref)
            .finish();
        file_spec.finish();

        embedded_files.insert(embed.name().clone(), file_spec_dict_ref);
    }
    Ok((chunk, embedded_files))
}
