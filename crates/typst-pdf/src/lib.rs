//! Exporting Typst documents to PDF.

mod convert;
mod embed;
mod image;
mod link;
mod metadata;
mod outline;
mod page;
mod paint;
mod shape;
mod text;
mod util;

use std::fmt::{self, Debug, Formatter};

use ecow::eco_format;
use serde::{Deserialize, Serialize};
use typst_library::diag::{bail, SourceResult, StrResult};
use typst_library::foundations::{Datetime, Smart};
use typst_library::layout::{PageRanges, PagedDocument};

/// Export a document into a PDF file.
///
/// Returns the raw bytes making up the PDF file.
#[typst_macros::time(name = "pdf")]
pub fn pdf(document: &PagedDocument, options: &PdfOptions) -> SourceResult<Vec<u8>> {
    convert::convert(document, options)
}

/// Settings for PDF export.
#[derive(Debug, Default)]
pub struct PdfOptions<'a> {
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
    pub ident: Smart<&'a str>,
    /// If not `None`, shall be the creation timestamp of the document. It will
    /// only be used if `set document(date: ..)` is `auto`.
    pub timestamp: Option<Timestamp>,
    /// Specifies which ranges of pages should be exported in the PDF. When
    /// `None`, all pages should be exported.
    pub page_ranges: Option<PageRanges>,
    /// A list of PDF standards that Typst will enforce conformance with.
    pub standards: PdfStandards,
}

/// Encapsulates a list of compatible PDF standards.
#[derive(Clone)]
pub struct PdfStandards {
    pub(crate) config: krilla::configure::Configuration,
}

impl PdfStandards {
    /// Validates a list of PDF standards for compatibility and returns their
    /// encapsulated representation.
    pub fn new(list: &[PdfStandard]) -> StrResult<Self> {
        use krilla::configure::{Configuration, PdfVersion, Validator};

        let mut version: Option<PdfVersion> = None;
        let mut set_version = |v: PdfVersion| -> StrResult<()> {
            if let Some(prev) = version {
                bail!(
                    "PDF cannot conform to {} and {} at the same time",
                    prev.as_str(),
                    v.as_str()
                );
            }
            version = Some(v);
            Ok(())
        };

        let mut validator = None;
        let mut set_validator = |v: Validator| -> StrResult<()> {
            if validator.is_some() {
                bail!("Typst currently only supports one PDF substandard at a time");
            }
            validator = Some(v);
            Ok(())
        };

        for standard in list {
            match standard {
                PdfStandard::V_1_4 => set_version(PdfVersion::Pdf14)?,
                PdfStandard::V_1_5 => set_version(PdfVersion::Pdf15)?,
                PdfStandard::V_1_6 => set_version(PdfVersion::Pdf16)?,
                PdfStandard::V_1_7 => set_version(PdfVersion::Pdf17)?,
                PdfStandard::V_2_0 => set_version(PdfVersion::Pdf20)?,
                PdfStandard::A_1b => set_validator(Validator::A1_B)?,
                PdfStandard::A_2b => set_validator(Validator::A2_B)?,
                PdfStandard::A_2u => set_validator(Validator::A2_U)?,
                PdfStandard::A_3b => set_validator(Validator::A3_B)?,
                PdfStandard::A_3u => set_validator(Validator::A3_U)?,
                PdfStandard::A_4 => set_validator(Validator::A4)?,
                PdfStandard::A_4f => set_validator(Validator::A4F)?,
                PdfStandard::A_4e => set_validator(Validator::A4E)?,
            }
        }

        let version = version.unwrap_or(PdfVersion::Pdf17);
        let validator = validator.unwrap_or_default();

        let config = Configuration::new_with(validator, version).ok_or_else(|| {
            eco_format!(
                "{} is not compatible with {}",
                version.as_str(),
                validator.as_str()
            )
        })?;

        Ok(Self { config })
    }
}

impl Debug for PdfStandards {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.pad("PdfStandards(..)")
    }
}

impl Default for PdfStandards {
    fn default() -> Self {
        use krilla::configure::{Configuration, PdfVersion};
        Self {
            config: Configuration::new_with_version(PdfVersion::Pdf17),
        }
    }
}

/// A PDF standard that Typst can enforce conformance with.
///
/// Support for more standards is planned.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[allow(non_camel_case_types)]
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
    /// PDF/A-2b.
    #[serde(rename = "a-2b")]
    A_2b,
    /// PDF/A-2u.
    #[serde(rename = "a-2u")]
    A_2u,
    /// PDF/A-3u.
    #[serde(rename = "a-3b")]
    A_3b,
    /// PDF/A-3u.
    #[serde(rename = "a-3u")]
    A_3u,
    /// PDF/A-4.
    #[serde(rename = "a-4")]
    A_4,
    /// PDF/A-4f.
    #[serde(rename = "a-4f")]
    A_4f,
    /// PDF/A-4e.
    #[serde(rename = "a-4e")]
    A_4e,
}

/// A timestamp with timezone information.
#[derive(Debug, Clone, Copy)]
pub struct Timestamp {
    /// The datetime of the timestamp.
    pub(crate) datetime: Datetime,
    /// The timezone of the timestamp.
    pub(crate) timezone: Timezone,
}

impl Timestamp {
    /// Create a new timestamp with a given datetime and UTC suffix.
    pub fn new_utc(datetime: Datetime) -> Self {
        Self { datetime, timezone: Timezone::UTC }
    }

    /// Create a new timestamp with a given datetime, and a local timezone offset.
    pub fn new_local(datetime: Datetime, whole_minute_offset: i32) -> Option<Self> {
        let hour_offset = (whole_minute_offset / 60).try_into().ok()?;
        // Note: the `%` operator in Rust is the remainder operator, not the
        // modulo operator. The remainder operator can return negative results.
        // We can simply apply `abs` here because we assume the `minute_offset`
        // will have the same sign as `hour_offset`.
        let minute_offset = (whole_minute_offset % 60).abs().try_into().ok()?;
        match (hour_offset, minute_offset) {
            // Only accept valid timezone offsets with `-23 <= hours <= 23`,
            // and `0 <= minutes <= 59`.
            (-23..=23, 0..=59) => Some(Self {
                datetime,
                timezone: Timezone::Local { hour_offset, minute_offset },
            }),
            _ => None,
        }
    }
}

/// A timezone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Timezone {
    /// The UTC timezone.
    UTC,
    /// The local timezone offset from UTC. And the `minute_offset` will have
    /// same sign as `hour_offset`.
    Local { hour_offset: i8, minute_offset: u8 },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timestamp_new_local() {
        let dummy_datetime = Datetime::from_ymd_hms(2024, 12, 17, 10, 10, 10).unwrap();
        let test = |whole_minute_offset, expect_timezone| {
            assert_eq!(
                Timestamp::new_local(dummy_datetime, whole_minute_offset)
                    .unwrap()
                    .timezone,
                expect_timezone
            );
        };

        // Valid timezone offsets
        test(0, Timezone::Local { hour_offset: 0, minute_offset: 0 });
        test(480, Timezone::Local { hour_offset: 8, minute_offset: 0 });
        test(-480, Timezone::Local { hour_offset: -8, minute_offset: 0 });
        test(330, Timezone::Local { hour_offset: 5, minute_offset: 30 });
        test(-210, Timezone::Local { hour_offset: -3, minute_offset: 30 });
        test(-720, Timezone::Local { hour_offset: -12, minute_offset: 0 }); // AoE

        // Corner cases
        test(315, Timezone::Local { hour_offset: 5, minute_offset: 15 });
        test(-225, Timezone::Local { hour_offset: -3, minute_offset: 45 });
        test(1439, Timezone::Local { hour_offset: 23, minute_offset: 59 });
        test(-1439, Timezone::Local { hour_offset: -23, minute_offset: 59 });

        // Invalid timezone offsets
        assert!(Timestamp::new_local(dummy_datetime, 1440).is_none());
        assert!(Timestamp::new_local(dummy_datetime, -1440).is_none());
        assert!(Timestamp::new_local(dummy_datetime, i32::MAX).is_none());
        assert!(Timestamp::new_local(dummy_datetime, i32::MIN).is_none());
    }
}
