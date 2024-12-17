//! Exporting Typst documents to PDF.

mod convert;
mod image;
mod link;
mod metadata;
mod outline;
mod page;
mod paint;
mod shape;
mod text;
mod util;

use typst_library::diag::SourceResult;
use typst_library::foundations::{Datetime, Smart};
use typst_library::layout::{PageRanges, PagedDocument};

pub use ::krilla::validation::Validator;
pub use ::krilla::version::PdfVersion;

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
    /// The version that should be used to export the PDF.
    pub pdf_version: Option<PdfVersion>,
    /// A standard the PDF should conform to.
    pub validator: Validator,
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
