use ecow::{eco_format, EcoString};
use std::{
    collections::HashSet,
    fmt::{self, Display, Formatter},
    ops::Range,
};
use typst::syntax::{PackageVersion, Source};
use unscanny::Scanner;

struct TestConfiguration {
    compare_ref: Option<bool>,
    validate_hints: Option<bool>,
}

struct TestPartMetadata {
    part_configuration: TestConfiguration,
    annotations: HashSet<Annotation>,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
struct Annotation {
    range: Option<Range<usize>>,
    message: EcoString,
    kind: AnnotationKind,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
enum AnnotationKind {
    Error,
    Warning,
    Hint,
}

impl AnnotationKind {
    fn iter() -> impl Iterator<Item = Self> {
        [AnnotationKind::Error, AnnotationKind::Warning, AnnotationKind::Hint].into_iter()
    }

    fn as_str(self) -> &'static str {
        match self {
            AnnotationKind::Error => "Error",
            AnnotationKind::Warning => "Warning",
            AnnotationKind::Hint => "Hint",
        }
    }
}

impl Display for AnnotationKind {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.pad(self.as_str())
    }
}

fn parse_part_metadata(source: &Source) -> TestPartMetadata {
    let mut compare_ref = None;
    let mut validate_hints = None;
    let mut annotations = HashSet::default();

    let lines: Vec<_> = source.text().lines().map(str::trim).collect();
    for (i, line) in lines.iter().enumerate() {
        compare_ref = get_flag_metadata(line, "Ref").or(compare_ref);
        validate_hints = get_flag_metadata(line, "Hints").or(validate_hints);

        fn num(s: &mut Scanner) -> Option<isize> {
            let mut first = true;
            let n = &s.eat_while(|c: char| {
                let valid = first && c == '-' || c.is_numeric();
                first = false;
                valid
            });
            n.parse().ok()
        }

        let comments_until_code =
            lines[i..].iter().take_while(|line| line.starts_with("//")).count();

        let pos = |s: &mut Scanner| -> Option<usize> {
            let first = num(s)? - 1;
            let (delta, column) =
                if s.eat_if(':') { (first, num(s)? - 1) } else { (0, first) };
            let line = (i + comments_until_code).checked_add_signed(delta)?;
            source.line_column_to_byte(line, usize::try_from(column).ok()?)
        };

        let range = |s: &mut Scanner| -> Option<Range<usize>> {
            let start = pos(s)?;
            let end = if s.eat_if('-') { pos(s)? } else { start };
            Some(start..end)
        };

        for kind in AnnotationKind::iter() {
            let Some(expectation) = get_metadata(line, kind.as_str()) else { continue };
            let mut s = Scanner::new(expectation);
            let range = range(&mut s);
            let rest = if range.is_some() { s.after() } else { s.string() };
            let message = rest
                .trim()
                .replace("VERSION", &PackageVersion::compiler().to_string())
                .into();
            annotations.insert(Annotation { kind, range, message });
        }
    }

    TestPartMetadata {
        part_configuration: TestConfiguration { compare_ref, validate_hints },
        annotations,
    }
}

fn get_metadata<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    line.strip_prefix(eco_format!("// {key}: ").as_str())
}

fn get_flag_metadata(line: &str, key: &str) -> Option<bool> {
    get_metadata(line, key).map(|value| value == "true")
}
