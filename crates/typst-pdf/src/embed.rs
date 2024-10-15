use crate::{PdfChunk, WithGlobalRefs};
use pdf_writer::{Date, Finish, Name, Ref, Str, TextStr};
use std::collections::HashMap;
use typst::diag::SourceResult;
use typst::foundations::Bytes;

pub fn build_embedded_files_references(
    ctx: &WithGlobalRefs,
) -> SourceResult<(PdfChunk, HashMap<String, Ref>)> {
    let mut chunk = PdfChunk::new();
    let mut embedded_files = HashMap::default();
    for embed in &ctx.resources.embeds {
        let embedded_file_stream_ref = chunk.alloc.bump();
        let file_spec_dict_ref = chunk.alloc.bump();

        let bytes: Bytes = embed.data.clone().into();
        let length = bytes.len();
        let mut embedded_file =
            chunk.embedded_file(embedded_file_stream_ref, bytes.as_ref());
        embedded_file
            .subtype(Name(b"text/xml"))
            .pair(Name(b"Length"), length as i32);
        embedded_file.params().modification_date(Date::new(2023)).finish(); // Todo: can we just let this out?
        embedded_file.finish();

        let mut file_spec = chunk.file_spec(file_spec_dict_ref);
        file_spec
            .path(Str(embed.path.as_bytes()))
            .unic_file(TextStr(embed.path.as_str()))
            .description(TextStr(embed.path.as_str())) // Todo
            .pair(Name(b"AFRelationship"), Name(b"Data")); // Todo
        file_spec
            .insert(Name(b"EF"))
            .dict()
            .pair(Name(b"F"), embedded_file_stream_ref)
            .pair(Name(b"UF"), embedded_file_stream_ref)
            .finish();
        file_spec.finish();

        embedded_files.insert(embed.path.to_string(), file_spec_dict_ref);
    }
    Ok((chunk, embedded_files))
}
