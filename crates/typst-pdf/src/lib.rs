//! Exporting Typst documents to PDF.

mod attach;
mod convert;
mod image;
mod link;
mod metadata;
mod outline;
mod page;
mod paint;
mod shape;
mod tags;
mod text;
mod util;

pub use self::metadata::{Timestamp, Timezone};

use std::fmt::{self, Debug, Formatter};
use std::hash::{Hash, Hasher};

use comemo::Tracked;
use ecow::{EcoString, eco_format};
use krilla::configure::Accessibility;
use serde::{Deserialize, Serialize};
use typst_layout::PagedDocument;
use typst_library::diag::{HintedStrResult, HintedString, SourceResult, StrResult, bail};
use typst_library::foundations::Smart;
use typst_library::introspection::Location;
use typst_library::layout::PageRanges;
use typst_library::model::LateLinkResolver;

/// Export a document into a PDF file.
///
/// Returns the raw bytes making up the PDF file.
#[typst_macros::time(name = "pdf")]
pub fn pdf(document: &PagedDocument, options: &PdfOptions) -> SourceResult<Vec<u8>> {
    convert::convert(document, options, &[], None)
}

/// Export a document into a PDF file as part of a bundle.
///
/// Takes additional `anchor` locations that will be serialized as named
/// destinations. This enables other documents in the bundle to link into the
/// resulting PDF. Also takes a `link_resolver` for resolving cross-document
/// links.
#[typst_macros::time(name = "pdf in bundle")]
pub fn pdf_in_bundle(
    document: &PagedDocument,
    options: &PdfOptions,
    anchors: &[(Location, EcoString)],
    link_resolver: Tracked<LateLinkResolver>,
) -> SourceResult<Vec<u8>> {
    convert::convert(document, options, anchors, Some(link_resolver))
}

/// Settings for PDF export.
#[derive(Debug, Hash)]
pub struct PdfOptions {
    /// If not `Smart::Auto`, shall be a string that uniquely and stably
    /// identifies the document. It should not change between compilations of
    /// the same document.  **If you cannot provide such a stable identifier,
    /// just pass `Smart::Auto` rather than trying to come up with one.** The
    /// CLI, for example, does not have a well-defined notion of a long-lived
    /// project and as such just passes `Smart::Auto`.
    ///
    /// If an `ident` is given, the hash of it will be used to create a PDF
    /// document identifier (the identifier itself is not leaked). If `ident` is
    /// `Auto`, a hash of the document's title and author is used instead (which
    /// is reasonably unique and stable).
    pub ident: Smart<String>,
    /// Configures the `/Creator` metadata in the resulting PDF. When set to
    /// `Smart::Auto`, defaults to `Typst $version`.
    pub creator: Smart<Option<String>>,
    /// If not `None`, shall be the creation timestamp of the document. It will
    /// only be used if `set document(date: ..)` is `auto`.
    pub timestamp: Option<Timestamp>,
    /// Specifies which ranges of pages should be exported in the PDF. When
    /// `None`, all pages should be exported.
    pub page_ranges: Option<PageRanges>,
    /// A list of PDF standards that Typst will enforce conformance with.
    pub standards: PdfStandards,
    /// By default, even when not producing a `PDF/UA-1` document, a tagged PDF
    /// document is written to provide a baseline of accessibility. In some
    /// circumstances, for example when trying to reduce the size of a document,
    /// it can be desirable to disable tagged PDF.
    pub tagged: bool,
    /// Whether to format the PDF in a human-readable way.
    pub pretty: bool,
}

impl PdfOptions {
    /// Returns the accessibility validator. Returns `Some` for PDF/UA-1, and in
    /// the future maybe PDF/UA-2.
    pub(crate) fn accessibility_validator(&self) -> Option<Accessibility> {
        self.standards.config.validators().accessibility()
    }
}

impl Default for PdfOptions {
    fn default() -> Self {
        Self {
            ident: Smart::Auto,
            creator: Smart::Auto,
            timestamp: None,
            page_ranges: None,
            standards: PdfStandards::default(),
            tagged: true,
            pretty: false,
        }
    }
}

/// Encapsulates a list of compatible PDF standards.
#[derive(Clone)]
pub struct PdfStandards {
    pub(crate) config: krilla::configure::Configuration,
}

impl PdfStandards {
    /// Validates a list of PDF standards for compatibility and returns their
    /// encapsulated representation.
    pub fn new(list: &[PdfStandard]) -> HintedStrResult<Self> {
        use krilla::configure::{
            Accessibility, Archival, ConfigurationBuilder, ConfigurationError, PdfVersion,
        };

        use crate::util::ValidatorsExt;

        let mut version: Option<PdfVersion> = None;
        let mut set_version = |v: PdfVersion| -> StrResult<()> {
            if let Some(prev) = version {
                bail!(
                    "PDF cannot conform to {} and {} at the same time",
                    prev.as_str(),
                    v.as_str(),
                );
            }
            version = Some(v);
            Ok(())
        };

        let mut archival_validator = None;
        let mut set_archival_validator = |a: Archival| -> StrResult<()> {
            if archival_validator.is_some() {
                bail!("choose at most one PDF/A standard");
            }
            archival_validator = Some(a);
            Ok(())
        };

        let mut accessibility_validator = None;
        let mut set_accessibility_validator = |ua: Accessibility| -> StrResult<()> {
            if accessibility_validator.is_some() {
                bail!("choose at most one PDF/UA standard");
            }
            accessibility_validator = Some(ua);
            Ok(())
        };

        for standard in list {
            match standard {
                PdfStandard::V_1_4 => set_version(PdfVersion::Pdf14)?,
                PdfStandard::V_1_5 => set_version(PdfVersion::Pdf15)?,
                PdfStandard::V_1_6 => set_version(PdfVersion::Pdf16)?,
                PdfStandard::V_1_7 => set_version(PdfVersion::Pdf17)?,
                PdfStandard::V_2_0 => set_version(PdfVersion::Pdf20)?,
                PdfStandard::A_1b => set_archival_validator(Archival::A1_B)?,
                PdfStandard::A_1a => set_archival_validator(Archival::A1_A)?,
                PdfStandard::A_2b => set_archival_validator(Archival::A2_B)?,
                PdfStandard::A_2u => set_archival_validator(Archival::A2_U)?,
                PdfStandard::A_2a => set_archival_validator(Archival::A2_A)?,
                PdfStandard::A_3b => set_archival_validator(Archival::A3_B)?,
                PdfStandard::A_3u => set_archival_validator(Archival::A3_U)?,
                PdfStandard::A_3a => set_archival_validator(Archival::A3_A)?,
                PdfStandard::A_4 => set_archival_validator(Archival::A4)?,
                PdfStandard::A_4f => set_archival_validator(Archival::A4F)?,
                PdfStandard::A_4e => set_archival_validator(Archival::A4E)?,
                PdfStandard::Ua_1 => set_accessibility_validator(Accessibility::UA1)?,
            }
        }

        let mut builder = ConfigurationBuilder::new();

        if let Some(version) = version {
            builder = builder.with_version(version)
        }

        if let Some(archival_validator) = archival_validator {
            builder = builder.with_archival_validator(archival_validator)
        }

        if let Some(accessibility_validator) = accessibility_validator {
            builder = builder.with_accessibility_validator(accessibility_validator)
        }

        let config = builder.finish().map_err(|e| {
            let (message, validators) = match e {
                ConfigurationError::NoOverlappingValidatorsRange(validators) => {
                    let list = validators.to_and_list();
                    let message = eco_format!(
                        "{list} are mutually incompatible because \
                         they do not have any overlapping PDF versions"
                    );
                    (message, validators)
                }
                ConfigurationError::VersionDoesNotMatchValidatorsRange(
                    version,
                    validators,
                ) => {
                    let list = validators.to_and_list();
                    let message =
                        eco_format!("{} is not compatible with {list}", version.as_str());
                    (message, validators)
                }
            };
            HintedString::new(message)
                .with_hints(validators.into_iter().map(version_hint))
        })?;

        Ok(Self { config })
    }
}

/// A hint specifying which PDF version a validator is compatible with.
fn version_hint(validator: krilla::configure::Validator) -> EcoString {
    let min = validator.min();
    let max = validator.max();
    if let Some(min) = min {
        if min == max {
            eco_format!("{} requires version {}", validator.as_str(), min.as_str())
        } else {
            eco_format!(
                "{} requires a version between {} and {}",
                validator.as_str(),
                min.as_str(),
                max.as_str()
            )
        }
    } else {
        eco_format!("{} requires at least {}", validator.as_str(), max.as_str())
    }
}

impl Debug for PdfStandards {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.pad("PdfStandards(..)")
    }
}

impl Default for PdfStandards {
    fn default() -> Self {
        use krilla::configure::{ConfigurationBuilder, PdfVersion};
        Self {
            config: ConfigurationBuilder::new()
                .with_version(PdfVersion::Pdf17)
                .finish()
                .unwrap(),
        }
    }
}

// Could be turned into a derive if krilla's `Configuration` ever implements
// `Hash`.
impl Hash for PdfStandards {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (self.config.version() as usize).hash(state);
        for validator in self.config.validators() {
            validator.hash(state);
        }
    }
}

/// A PDF standard that Typst can enforce conformance with.
///
/// Support for more standards is planned.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[expect(non_camel_case_types)]
#[non_exhaustive]
pub enum PdfStandard {
    /// PDF 1.4.
    #[serde(rename = "1.4")]
    V_1_4,
    /// PDF 1.5.
    #[serde(rename = "1.5")]
    V_1_5,
    /// PDF 1.5.
    #[serde(rename = "1.6")]
    V_1_6,
    /// PDF 1.7.
    #[serde(rename = "1.7")]
    V_1_7,
    /// PDF 2.0.
    #[serde(rename = "2.0")]
    V_2_0,
    /// PDF/A-1b.
    #[serde(rename = "a-1b")]
    A_1b,
    /// PDF/A-1a.
    #[serde(rename = "a-1a")]
    A_1a,
    /// PDF/A-2b.
    #[serde(rename = "a-2b")]
    A_2b,
    /// PDF/A-2u.
    #[serde(rename = "a-2u")]
    A_2u,
    /// PDF/A-2a.
    #[serde(rename = "a-2a")]
    A_2a,
    /// PDF/A-3b.
    #[serde(rename = "a-3b")]
    A_3b,
    /// PDF/A-3u.
    #[serde(rename = "a-3u")]
    A_3u,
    /// PDF/A-3a.
    #[serde(rename = "a-3a")]
    A_3a,
    /// PDF/A-4.
    #[serde(rename = "a-4")]
    A_4,
    /// PDF/A-4f.
    #[serde(rename = "a-4f")]
    A_4f,
    /// PDF/A-4e.
    #[serde(rename = "a-4e")]
    A_4e,
    /// PDF/UA-1.
    #[serde(rename = "ua-1")]
    Ua_1,
}
