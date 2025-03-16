use std::sync::Arc;

use krilla::Document;
use krilla::interchange::embed::{AssociationKind, EmbeddedFile};
use typst_library::diag::{bail, SourceResult};
use typst_library::foundations::{NativeElement, StyleChain};
use typst_library::layout::PagedDocument;
use typst_library::pdf::{EmbedElem, EmbeddedFileRelationship};
use typst_syntax::Span;

pub(crate) fn embed_files(
    typst_doc: &PagedDocument,
    document: &mut Document,
) -> SourceResult<()> {
    let elements = typst_doc.introspector.query(&EmbedElem::elem().select());

    for elem in &elements {
        let embed = elem.to_packed::<EmbedElem>().unwrap();
        let derived_path = &embed.path.derived;
        let path = derived_path.to_string();
        let mime_type =
            embed.mime_type(StyleChain::default()).clone().map(|s| s.to_string());
        let description = embed
            .description(StyleChain::default())
            .clone()
            .map(|s| s.to_string());
        let association_kind = match embed.relationship(StyleChain::default()) {
            None => AssociationKind::Unspecified,
            Some(e) => match e {
                EmbeddedFileRelationship::Source => AssociationKind::Source,
                EmbeddedFileRelationship::Data => AssociationKind::Data,
                EmbeddedFileRelationship::Alternative => AssociationKind::Alternative,
                EmbeddedFileRelationship::Supplement => AssociationKind::Supplement,
            },
        };
        let data: Arc<dyn AsRef<[u8]> + Send + Sync> = Arc::new(embed.data.clone());

        let file = EmbeddedFile {
            path,
            mime_type,
            description,
            association_kind,
            data: data.into(),
            compress: true,
            location: Some(embed.span().into_raw().get()),
        };

        if document.embed_file(file).is_none() {
            bail!(Span::detached(), "attempted to embed file {derived_path} twice");
        }
    }

    Ok(())
}
