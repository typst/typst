use std::collections::BTreeMap;

use ecow::EcoString;
use pdf_writer::{Finish, Name, Ref, Str, TextStr};
use typst_library::diag::{bail, SourceResult};
use typst_library::foundations::{NativeElement, Packed, StyleChain};
use typst_library::pdf::{EmbedElem, EmbeddedFileRelationship};

use crate::catalog::{document_date, pdf_date};
use crate::{PdfChunk, WithGlobalRefs};

/// Query for all [`EmbedElem`] and write them and their file specifications.
///
/// This returns a map of embedding names and references so that we can later add them to the
/// catalog's name dictionary.
pub fn write_embedded_files(
    ctx: &WithGlobalRefs,
) -> SourceResult<(PdfChunk, BTreeMap<EcoString, Ref>)> {
    let mut chunk = PdfChunk::new();
    let mut embedded_files = BTreeMap::default();

    let elements = ctx.document.introspector.query(&EmbedElem::elem().select());
    for elem in &elements {
        if !ctx.options.standards.embedded_files {
            // PDF/A-2 requires embedded files to be PDF/A-1 or PDF/A-2,
            // which we don't currently check.
            bail!(
                elem.span(),
                "file embeddings are not currently supported for PDF/A-2";
                hint: "PDF/A-3 supports arbitrary embedded files"
            );
        }

        let embed = elem.to_packed::<EmbedElem>().unwrap();
        let name = embed
            .name(StyleChain::default())
            .as_ref()
            .unwrap_or(&embed.resolved_path);
        embedded_files.insert(name.clone(), embed_file(ctx, &mut chunk, embed)?);
    }

    Ok((chunk, embedded_files))
}

/// Write the embedded file stream and its file specification.
fn embed_file(
    ctx: &WithGlobalRefs,
    chunk: &mut PdfChunk,
    embed: &Packed<EmbedElem>,
) -> SourceResult<Ref> {
    let embedded_file_stream_ref = chunk.alloc.bump();
    let file_spec_dict_ref = chunk.alloc.bump();

    let length = embed.data().as_slice().len();
    let data = embed.data().as_slice();

    let mut embedded_file =
        chunk.embedded_file(embedded_file_stream_ref, embed.data().as_slice());
    if let Some(mime_type) = embed.mime_type(StyleChain::default()) {
        embedded_file.subtype(Name(mime_type.as_bytes()));
    }

    let mut params = embedded_file.params();
    params.size(data.len() as i32);

    let (date, tz) = document_date(ctx.document.info.date, ctx.options.timestamp);
    if let Some(pdf_date) = date.and_then(|date| pdf_date(date, tz)) {
        params.modification_date(pdf_date);
    } else if ctx.options.standards.pdfa {
        bail!(embed.span(), "embedded files must have a modification date in PDF/A-3");
    }

    params.finish();
    embedded_file.finish();

    let path = embed.resolved_path().replace("\\", "/");
    let mut file_spec = chunk.file_spec(file_spec_dict_ref);
    file_spec
        .path(Str(path.as_bytes()))
        .unic_file(TextStr(&path))
        .insert(Name(b"EF"))
        .dict()
        .pair(Name(b"F"), embedded_file_stream_ref)
        .pair(Name(b"UF"), embedded_file_stream_ref);

    if ctx.options.standards.pdfa {
        if let Some(relationship) = embed.relationship(StyleChain::default()) {
            // PDF 2.0, but ISO 19005-3 (PDF/A-3) Annex E allows it for PDF/A-3
            file_spec.association_kind(match relationship {
                EmbeddedFileRelationship::Source => AssociationKind::Source,
                EmbeddedFileRelationship::Data => AssociationKind::Data,
                EmbeddedFileRelationship::Alternative => AssociationKind::Alternative,
                EmbeddedFileRelationship::Supplement => AssociationKind::Supplement,
                EmbeddedFileRelationship::Unspecified => AssociationKind::Unspecified,
            });
        } else {
            bail!(embed.span(), "embedded files must have a relationship in PDF/A-3")
        }
    }

    if let Some(description) = embed.description(StyleChain::default()) {
        file_spec.description(TextStr(description));
    }

    Ok(file_spec_dict_ref)
}
