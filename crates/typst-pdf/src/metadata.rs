use ecow::EcoString;
use krilla::metadata::Metadata;
use typst_library::foundations::Datetime;

use crate::krilla::GlobalContext;

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

    let tz = gc.document.info.date.is_auto();
    if let Some(date) = gc
        .document
        .info
        .date
        .unwrap_or(gc.options.timestamp)
        .and_then(|d| convert_date(d, tz))
    {
        metadata = metadata.modification_date(date).creation_date(date);
    }

    metadata
}

// TODO: Sync with recent PR
fn convert_date(datetime: Datetime, tz: bool) -> Option<krilla::metadata::DateTime> {
    let year = datetime.year().filter(|&y| y >= 0)? as u16;

    let mut krilla_date = krilla::metadata::DateTime::new(year);

    if let Some(month) = datetime.month() {
        krilla_date = krilla_date.month(month);
    }

    if let Some(day) = datetime.day() {
        krilla_date = krilla_date.day(day);
    }

    if let Some(h) = datetime.hour() {
        krilla_date = krilla_date.hour(h);
    }

    if let Some(m) = datetime.minute() {
        krilla_date = krilla_date.minute(m);
    }

    if let Some(s) = datetime.second() {
        krilla_date = krilla_date.second(s);
    }

    if tz {
        krilla_date = krilla_date.utc_offset_hour(0).utc_offset_minute(0);
    }

    Some(krilla_date)
}
