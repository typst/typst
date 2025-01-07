use std::collections::BTreeMap;

use ecow::EcoString;
use pdf_writer::types::AssociationKind;
use pdf_writer::{Filter, Finish, Name, Ref, Str, TextStr};
use typst_library::diag::{bail, SourceResult};
use typst_library::foundations::{NativeElement, Packed, StyleChain};
use typst_library::pdf::{EmbedElem, EmbeddedFileRelationship};

use crate::catalog::{document_date, pdf_date};
use crate::{deflate, NameExt, PdfChunk, StrExt, TextStrExt, WithGlobalRefs};

/// Query for all [`EmbedElem`] and write them and their file specifications.
///
/// This returns a map of embedding names and references so that we can later
/// add them to the catalog's name dictionary.
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
        if embed.resolved_path.len() > Str::PDFA_LIMIT {
            bail!(embed.span(), "embedded file path is too long");
        }

        let id = embed_file(ctx, &mut chunk, embed)?;
        if embedded_files.insert(embed.resolved_path.clone(), id).is_some() {
            bail!(
                elem.span(),
                "duplicate embedded file for path `{}`", embed.resolved_path;
                hint: "embedded file paths must be unique",
            );
        }
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

    let data = embed.data().as_slice();
    let compressed = deflate(data);

    let mut embedded_file = chunk.embedded_file(embedded_file_stream_ref, &compressed);
    embedded_file.filter(Filter::FlateDecode);

    if let Some(mime_type) = embed.mime_type(StyleChain::default()) {
        if mime_type.len() > Name::PDFA_LIMIT {
            bail!(embed.span(), "MIME type is too long");
        }
        embedded_file.subtype(Name(mime_type.as_bytes()));
    } else if ctx.options.standards.pdfa {
        bail!(embed.span(), "embedded files must have a MIME type in PDF/A-3");
    }

    let mut params = embedded_file.params();
    params.size(data.len() as i32);

    let (date, tz) = document_date(ctx.document.info.date, ctx.options.timestamp);
    if let Some(pdf_date) = date.and_then(|date| pdf_date(date, tz)) {
        params.modification_date(pdf_date);
    } else if ctx.options.standards.pdfa {
        bail!(
            embed.span(),
            "the document must have a date when embedding files in PDF/A-3";
            hint: "`set document(date: none)` must not be used in this case"
        );
    }

    params.finish();
    embedded_file.finish();

    let mut file_spec = chunk.file_spec(file_spec_dict_ref);
    file_spec.path(Str::trimmed(embed.resolved_path.as_bytes()));
    file_spec.unic_file(TextStr::trimmed(&embed.resolved_path));
    file_spec
        .insert(Name(b"EF"))
        .dict()
        .pair(Name(b"F"), embedded_file_stream_ref)
        .pair(Name(b"UF"), embedded_file_stream_ref);

    if ctx.options.standards.pdfa {
        // PDF 2.0, but ISO 19005-3 (PDF/A-3) Annex E allows it for PDF/A-3.
        file_spec.association_kind(match embed.relationship(StyleChain::default()) {
            Some(EmbeddedFileRelationship::Source) => AssociationKind::Source,
            Some(EmbeddedFileRelationship::Data) => AssociationKind::Data,
            Some(EmbeddedFileRelationship::Alternative) => AssociationKind::Alternative,
            Some(EmbeddedFileRelationship::Supplement) => AssociationKind::Supplement,
            None => AssociationKind::Unspecified,
        });
    }

    if let Some(description) = embed.description(StyleChain::default()) {
        if description.len() > Str::PDFA_LIMIT {
            bail!(embed.span(), "embedded file description is too long");
        }
        file_spec.description(TextStr::trimmed(description));
    }

    Ok(file_spec_dict_ref)
}
