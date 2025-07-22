use std::sync::Arc;

use krilla::Document;
use krilla::embed::{AssociationKind, EmbeddedFile};
use typst_library::diag::{SourceResult, bail};
use typst_library::foundations::{NativeElement, StyleChain};
use typst_library::layout::PagedDocument;
use typst_library::pdf::{EmbedElem, EmbeddedFileRelationship};

pub(crate) fn embed_files(
    typst_doc: &PagedDocument,
    document: &mut Document,
) -> SourceResult<()> {
    let elements = typst_doc.introspector.query(&EmbedElem::ELEM.select());

    for elem in &elements {
        let embed = elem.to_packed::<EmbedElem>().unwrap();
        let span = embed.span();
        let derived_path = &embed.path.derived;
        let path = derived_path.to_string();
        let mime_type = embed
            .mime_type
            .get_ref(StyleChain::default())
            .as_ref()
            .map(|s| s.to_string());
        let description = embed
            .description
            .get_ref(StyleChain::default())
            .as_ref()
            .map(|s| s.to_string());
        let association_kind = match embed.relationship.get(StyleChain::default()) {
            None => AssociationKind::Unspecified,
            Some(e) => match e {
                EmbeddedFileRelationship::Source => AssociationKind::Source,
                EmbeddedFileRelationship::Data => AssociationKind::Data,
                EmbeddedFileRelationship::Alternative => AssociationKind::Alternative,
                EmbeddedFileRelationship::Supplement => AssociationKind::Supplement,
            },
        };
        let data: Arc<dyn AsRef<[u8]> + Send + Sync> = Arc::new(embed.data.clone());
        let compress = should_compress(&embed.data);

        let file = EmbeddedFile {
            path,
            mime_type,
            description,
            association_kind,
            data: data.into(),
            compress,
            location: Some(span.into_raw().get()),
        };

        if document.embed_file(file).is_none() {
            bail!(span, "attempted to embed file {derived_path} twice");
        }
    }

    Ok(())
}

fn should_compress(data: &[u8]) -> Option<bool> {
    let ty = infer::get(data)?;
    match ty.matcher_type() {
        infer::MatcherType::App => None,
        infer::MatcherType::Archive => match ty.mime_type() {
            #[rustfmt::skip]
            "application/zip"
            | "application/vnd.rar"
            | "application/gzip"
            | "application/x-bzip2"
            | "application/vnd.bzip3"
            | "application/x-7z-compressed"
            | "application/x-xz"
            | "application/vnd.ms-cab-compressed"
            | "application/vnd.debian.binary-package"
            | "application/x-compress"
            | "application/x-lzip"
            | "application/x-rpm"
            | "application/zstd"
            | "application/x-lz4"
            | "application/x-ole-storage" => Some(false),
            _ => None,
        },
        infer::MatcherType::Audio => match ty.mime_type() {
            #[rustfmt::skip]
            "audio/mpeg"
            | "audio/m4a"
            | "audio/opus"
            | "audio/ogg"
            | "audio/x-flac"
            | "audio/amr"
            | "audio/aac"
            | "audio/x-ape" => Some(false),
            _ => None,
        },
        infer::MatcherType::Book => None,
        infer::MatcherType::Doc => None,
        infer::MatcherType::Font => None,
        infer::MatcherType::Image => match ty.mime_type() {
            #[rustfmt::skip]
            "image/jpeg"
            | "image/jp2"
            | "image/png"
            | "image/webp"
            | "image/vnd.ms-photo"
            | "image/heif"
            | "image/avif"
            | "image/jxl"
            | "image/vnd.djvu" => None,
            _ => None,
        },
        infer::MatcherType::Text => None,
        infer::MatcherType::Video => match ty.mime_type() {
            #[rustfmt::skip]
            "video/mp4"
            | "video/x-m4v"
            | "video/x-matroska"
            | "video/webm"
            | "video/quicktime"
            | "video/x-flv" => Some(false),
            _ => None,
        },
        infer::MatcherType::Custom => None,
    }
}
