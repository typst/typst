use std::sync::Arc;

use krilla::Document;
use krilla::embed::{AssociationKind, EmbeddedFile, MimeType};
use typst_library::diag::{SourceResult, bail};
use typst_library::foundations::{NativeElement, StyleChain};
use typst_library::layout::PagedDocument;
use typst_library::pdf::{AttachElem, AttachedFileRelationship};

pub(crate) fn attach_files(
    typst_doc: &PagedDocument,
    document: &mut Document,
) -> SourceResult<()> {
    let elements = typst_doc.introspector.query(&AttachElem::ELEM.select());

    for elem in &elements {
        let elem = elem.to_packed::<AttachElem>().unwrap();
        let span = elem.span();
        let derived_path = &elem.path.derived;
        let path = derived_path.to_string();
        let mime_type = elem
            .mime_type
            .get_ref(StyleChain::default())
            .as_ref()
            .map(|s| s.to_string());
        let description = elem
            .description
            .get_ref(StyleChain::default())
            .as_ref()
            .map(|s| s.to_string());
        let association_kind = match elem.relationship.get(StyleChain::default()) {
            None => AssociationKind::Unspecified,
            Some(e) => match e {
                AttachedFileRelationship::Source => AssociationKind::Source,
                AttachedFileRelationship::Data => AssociationKind::Data,
                AttachedFileRelationship::Alternative => AssociationKind::Alternative,
                AttachedFileRelationship::Supplement => AssociationKind::Supplement,
            },
        };
        let data: Arc<dyn AsRef<[u8]> + Send + Sync> = Arc::new(elem.data.clone());
        let compress = should_compress(&elem.data);

        let file = EmbeddedFile {
            path,
            mime_type: mime_type.map(|m| MimeType::new(&m).unwrap()),
            modification_date: None,
            description,
            association_kind,
            data: data.into(),
            compress,
            location: Some(span.into_raw()),
        };

        if document.embed_file(file).is_none() {
            bail!(span, "attempted to attach file {derived_path} twice");
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
