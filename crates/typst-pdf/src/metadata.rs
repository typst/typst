use ecow::EcoString;
use krilla::metadata::{Metadata, TextDirection};
use typst_library::foundations::{Datetime, Smart, StyleChain};
use typst_library::layout::Dir;
use typst_library::text::TextElem;

use crate::convert::GlobalContext;

pub(crate) fn build_metadata(gc: &GlobalContext) -> Metadata {
    let creator = format!("Typst {}", env!("CARGO_PKG_VERSION"));

    // Always write a language, PDF/UA-1 implicitly requires a document language
    // so the metadata and outline entries have an applicable language.
    let lang = gc.tags.doc_lang.unwrap_or(StyleChain::default().get(TextElem::lang));

    let dir = if lang.dir() == Dir::RTL {
        TextDirection::RightToLeft
    } else {
        TextDirection::LeftToRight
    };

    let mut metadata = Metadata::new()
        .creator(creator)
        .keywords(gc.document.info.keywords.iter().map(EcoString::to_string).collect())
        .authors(gc.document.info.author.iter().map(EcoString::to_string).collect())
        .language(lang.as_str().to_string());

    if let Some(title) = &gc.document.info.title {
        metadata = metadata.title(title.to_string());
    }

    if let Some(subject) = &gc.document.info.description {
        metadata = metadata.subject(subject.to_string());
    }

    if let Some(ident) = gc.options.ident.custom() {
        metadata = metadata.document_id(ident.to_string());
    }

    if let Some(date) = creation_date(gc) {
        metadata = metadata.creation_date(date);
    }

    metadata = metadata.text_direction(dir);

    metadata
}

/// (1) If the `document.date` is set to specific `datetime` or `none`, use it.
/// (2) If the `document.date` is set to `auto` or not set, try to use the
///     date from the options.
/// (3) Otherwise, we don't write date metadata.
pub fn creation_date(gc: &GlobalContext) -> Option<krilla::metadata::DateTime> {
    let (datetime, tz) = match (gc.document.info.date, gc.options.timestamp) {
        (Smart::Custom(Some(date)), _) => (date, None),
        (Smart::Auto, Some(timestamp)) => (timestamp.datetime, Some(timestamp.timezone)),
        _ => return None,
    };

    let year = datetime.year().filter(|&y| y >= 0)? as u16;

    let mut kd = krilla::metadata::DateTime::new(year);

    if let Some(month) = datetime.month() {
        kd = kd.month(month);
    }

    if let Some(day) = datetime.day() {
        kd = kd.day(day);
    }

    if let Some(h) = datetime.hour() {
        kd = kd.hour(h);
    }

    if let Some(m) = datetime.minute() {
        kd = kd.minute(m);
    }

    if let Some(s) = datetime.second() {
        kd = kd.second(s);
    }

    match tz {
        Some(Timezone::UTC) => kd = kd.utc_offset_hour(0).utc_offset_minute(0),
        Some(Timezone::Local { hour_offset, minute_offset }) => {
            kd = kd.utc_offset_hour(hour_offset).utc_offset_minute(minute_offset)
        }
        None => {}
    }

    Some(kd)
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
