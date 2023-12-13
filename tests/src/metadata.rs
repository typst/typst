use ecow::{eco_format, EcoString};
use std::collections::HashSet;
use std::fmt::{self, Display, Formatter};
use std::ops::Range;
use typst::syntax::{PackageVersion, Source};
use unscanny::Scanner;

/// Each typst test and test header may contain metadata.
/// Metadata either:
/// - influences the test behavior: [TestConfiguration]
/// - declares a propriety that your test must hold: [Annotation].
///     e.g. `// Warning: 1-3 no text within underscores`
///     will fail the test if the warning isn't generated
///     by your test.
/// [parse_part_metadata]
#[derive(Debug)]
pub struct TestPartMetadata {
    pub part_configuration: TestConfiguration,
    pub annotations: HashSet<Annotation>,
}

/// Valid metadata keys are `Hint`, `Ref`, `Autocomplete`.
/// Example : `// Ref: true`
///
/// Any value not equal to `true` or `false` will be ignored and throw a warning in stdout.
///
/// Changing these values modify the behavior of the test:
/// - compare_ref: reference images will be generated and compared.
/// - validate_hints: compiler hints will be recorded and compared to test hints annotations.
/// - validate_autocomplete autocomplete will be recorded and compared to test autocomplete annotations.
///     this is mutually exclusive with Errors and Hints, autocomplete test shall not contain Error metadata
///     as they would be ignored.
#[derive(Debug)]
pub struct TestConfiguration {
    pub compare_ref: Option<bool>,
    pub validate_hints: Option<bool>,
    pub validate_autocomplete: Option<bool>,
}

/// Annotation may be written in the form:
///
/// `// {key}: {range} msg`
///
/// where:
/// - valid keys are `Hint`, `Error`, `Warning`, `Autocomplete contains`, `Autocomplete excludes`
/// - range is parsed in [parse_part_metadata]
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Annotation {
    pub range: Option<Range<usize>>,
    pub message: EcoString,
    pub kind: AnnotationKind,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum AnnotationKind {
    Error,
    Warning,
    Hint,
    AutocompleteContains,
    AutocompleteExcludes,
}

impl AnnotationKind {
    pub fn iter() -> impl Iterator<Item = Self> {
        [
            AnnotationKind::Error,
            AnnotationKind::Warning,
            AnnotationKind::Hint,
            AnnotationKind::AutocompleteContains,
            AnnotationKind::AutocompleteExcludes,
        ]
        .into_iter()
    }

    pub fn as_str(self) -> &'static str {
        match self {
            AnnotationKind::Error => "Error",
            AnnotationKind::Warning => "Warning",
            AnnotationKind::Hint => "Hint",
            AnnotationKind::AutocompleteContains => "Autocomplete contains",
            AnnotationKind::AutocompleteExcludes => "Autocomplete excludes",
        }
    }
}

impl Display for AnnotationKind {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.pad(self.as_str())
    }
}

/// Metadata always start with `// {key}`
///
/// Valid keys may be any of [TestConfiguration] valid keys and [AnnotationKind] valid keys.
///
/// Parsing:
/// - Range may be written as:
///     - `{line}:{col}-{line}:{col}`
///         example : `0:4-0:6`
///     - `{col}-{col}`: in which case the line is assumed to be the line after the annotation.
///         example: `4-6`
///     - `-1` in which case, it is the range cursor..cursor where cursor is at the end of the next line,
///         skipping comments line. (Mostly useful for autocompletion which requires an index).
pub fn parse_part_metadata(source: &Source) -> TestPartMetadata {
    let mut compare_ref = None;
    let mut validate_hints = None;
    let mut validate_autocomplete = None;
    let mut annotations = HashSet::default();

    let lines: Vec<_> = source.text().lines().map(str::trim).collect();
    for (i, line) in lines.iter().enumerate() {
        compare_ref = get_flag_metadata(line, "Ref").or(compare_ref);
        validate_hints = get_flag_metadata(line, "Hints").or(validate_hints);
        validate_autocomplete =
            get_flag_metadata(line, "Autocomplete").or(validate_autocomplete);

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
            if s.eat_if("-1") {
                let mut add = 1;
                while let Some(line) = lines.get(i + add) {
                    if !line.starts_with("//") {
                        break;
                    }
                    add += 1;
                }
                let next_line = lines.get(i + add)?;
                let col = next_line.chars().count();

                let index = source.line_column_to_byte(i + add, col)?;
                return Some(index..index);
            }
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
        part_configuration: TestConfiguration {
            compare_ref,
            validate_hints,
            validate_autocomplete,
        },
        annotations,
    }
}

pub fn parse_autocomplete_message<'a>(message: &'a str) -> HashSet<&'a str> {
    let string = |s: &mut Scanner<'a>| -> Option<&'a str> {
        if s.eat_if('"') {
            let sub = s.eat_until('"');
            if !s.eat_if('"') {
                return None;
            }
            Some(sub)
        } else {
            None
        }
    };
    let list = |s: &mut Scanner<'a>| -> HashSet<&'a str> {
        let mut result = HashSet::new();
        loop {
            let Some(sub) = string(s) else { break };
            result.insert(sub);
            s.eat_while(|c: char| c.is_whitespace());
            if !s.eat_if(",") {
                break;
            }
            s.eat_while(|c: char| c.is_whitespace());
        }
        result
    };
    let mut s = Scanner::new(message);

    list(&mut s)
}

pub fn get_metadata<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    line.strip_prefix(eco_format!("// {key}: ").as_str())
}

pub fn get_flag_metadata(line: &str, key: &str) -> Option<bool> {
    get_metadata(line, key)
        .map(|value| {
            if !(value == "true" || value == "false") {
            println!("WARNING: invalid use of get_flag_metadata: flag should be `true` or `false` but is `{value}`");
            }
            value
        }).filter(|&value| value == "true" || value == "false")
        .map(|value| value == "true")
}
