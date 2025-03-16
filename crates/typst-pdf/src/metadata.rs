use ecow::EcoString;
use krilla::interchange::metadata::Metadata;
use typst_library::foundations::{Datetime, Smart};

use crate::convert::GlobalContext;
use crate::Timezone;

pub(crate) fn build_metadata(gc: &GlobalContext) -> Metadata {
    let creator = format!("Typst {}", env!("CARGO_PKG_VERSION"));

    let mut metadata = Metadata::new()
        .creator(creator)
        .keywords(gc.document.info.keywords.iter().map(EcoString::to_string).collect())
        .authors(gc.document.info.author.iter().map(EcoString::to_string).collect());

    let lang = gc.languages.iter().max_by_key(|(_, &count)| count).map(|(&l, _)| l);

    if let Some(lang) = lang {
        metadata = metadata.language(lang.as_str().to_string());
    }

    if let Some(title) = &gc.document.info.title {
        metadata = metadata.title(title.to_string());
    }

    if let Some(subject) = &gc.document.info.description {
        metadata = metadata.subject(subject.to_string());
    }

    if let Some(ident) = gc.options.ident.custom() {
        metadata = metadata.subject(ident.to_string());
    }

    // (1) If the `document.date` is set to specific `datetime` or `none`, use it.
    // (2) If the `document.date` is set to `auto` or not set, try to use the
    //     date from the options.
    // (3) Otherwise, we don't write date metadata.
    let (date, tz) = match (gc.document.info.date, gc.options.timestamp) {
        (Smart::Custom(date), _) => (date, None),
        (Smart::Auto, Some(timestamp)) => {
            (Some(timestamp.datetime), Some(timestamp.timezone))
        }
        _ => (None, None),
    };

    if let Some(date) = date.and_then(|d| convert_date(d, tz)) {
        metadata = metadata.modification_date(date).creation_date(date);
    }

    metadata
}

fn convert_date(
    datetime: Datetime,
    tz: Option<Timezone>,
) -> Option<krilla::interchange::metadata::DateTime> {
    let year = datetime.year().filter(|&y| y >= 0)? as u16;

    let mut kd = krilla::interchange::metadata::DateTime::new(year);

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
