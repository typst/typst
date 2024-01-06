use ecow::EcoString;
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
    // the annotation has an invalid message, range etc.
}

/// Parsing error when the metadata is invalid.
pub(crate) enum InvalidMetadata {
    /// An invalid annotation and it's error message.
    InvalidAnnotation(Annotation, String),
    /// Setting metadata can only be done with `true` or `false` as a value.
    InvalidSet(String),
}

impl InvalidMetadata {
    pub(crate) fn write(
        invalid_data: Vec<InvalidMetadata>,
        output: &mut String,
        print_annotation: &mut impl FnMut(&Annotation, &mut String),
    ) {
        use std::fmt::Write;
        for data in invalid_data.into_iter() {
            let (annotation, error) = match data {
                InvalidMetadata::InvalidAnnotation(a, e) => (Some(a), e),
                InvalidMetadata::InvalidSet(e) => (None, e),
            };
            write!(output, "{error}",).unwrap();
            if let Some(annotation) = annotation {
                print_annotation(&annotation, output)
            } else {
                writeln!(output).unwrap();
            }
        }
    }
}
/// Valid metadata keys are `Hint`, `Ref`, `Autocomplete`.
/// Example : `// Ref: true`
///
/// Any value not equal to `true` or `false` is invalid and will fail the test.
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

impl Default for TestConfiguration {
    fn default() -> Self {
        Self {
            compare_ref: Some(true),
            validate_hints: Some(true),
            validate_autocomplete: Some(false),
        }
    }
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
    pub fn as_str(self) -> &'static str {
        match self {
            AnnotationKind::Error => "Error",
            AnnotationKind::Warning => "Warning",
            AnnotationKind::Hint => "Hint",
            AnnotationKind::AutocompleteContains => "Autocomplete contains",
            AnnotationKind::AutocompleteExcludes => "Autocomplete excludes",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        let kind = match s {
            "Error" => AnnotationKind::Error,
            "Warning" => AnnotationKind::Warning,
            "Hint" => AnnotationKind::Hint,
            "Autocomplete contains" => AnnotationKind::AutocompleteContains,
            "Autocomplete excludes" => AnnotationKind::AutocompleteExcludes,
            _ => return None,
        };
        Some(kind)
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
pub fn parse_part_metadata(
    source: &Source,
    is_header: bool,
) -> Result<TestPartMetadata, Vec<InvalidMetadata>> {
    let mut compare_ref = None;
    let mut validate_hints = None;
    let mut validate_autocomplete = None;
    let mut annotations = HashSet::default();
    let mut invalid_data = vec![];

    let lines = get_lines(source);

    fn get_lines(source: &Source) -> Vec<&str> {
        source.text().lines().map(str::trim).collect()
    }

    fn num(s: &mut Scanner) -> Option<isize> {
        let mut first = true;
        let n = &s.eat_while(|c: char| {
            let valid = first && c == '-' || c.is_numeric();
            first = false;
            valid
        });
        n.parse().ok()
    }

    fn comments_until_code(source: &Source, i: usize) -> usize {
        get_lines(source)[i..]
            .iter()
            .take_while(|line| line.starts_with("//"))
            .count()
    }

    fn pos(s: &mut Scanner, i: usize, source: &Source) -> Option<usize> {
        let first = num(s)? - 1;
        let (delta, column) =
            if s.eat_if(':') { (first, num(s)? - 1) } else { (0, first) };
        let line = (i + comments_until_code(source, i)).checked_add_signed(delta)?;
        source.line_column_to_byte(line, usize::try_from(column).ok()?)
    }

    fn range(s: &mut Scanner, i: usize, source: &Source) -> Option<Range<usize>> {
        let lines = get_lines(source);
        s.eat_whitespace();
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
            s.eat_whitespace();
            return Some(index..index);
        }
        let start = pos(s, i, source)?;
        let end = if s.eat_if('-') { pos(s, i, source)? } else { start };
        s.eat_whitespace();
        Some(start..end)
    }

    for (i, line) in lines.iter().enumerate() {
        if let Some((key, value)) = get_metadata(line) {
            let key = key.trim();
            match key {
                "Ref" => {
                    validate_set_annotation(value, &mut compare_ref, &mut invalid_data)
                }
                "Hints" => {
                    validate_set_annotation(value, &mut validate_hints, &mut invalid_data)
                }
                "Autocomplete" => validate_set_annotation(
                    value,
                    &mut validate_autocomplete,
                    &mut invalid_data,
                ),
                annotation_key if AnnotationKind::from_str(annotation_key).is_some() => {
                    let kind = AnnotationKind::from_str(annotation_key).unwrap();
                    let mut s = Scanner::new(value);
                    let range = range(&mut s, i, source);
                    let rest = if range.is_some() { s.after() } else { s.string() };
                    let message = rest
                        .trim()
                        .replace("VERSION", &PackageVersion::compiler().to_string())
                        .into();

                    let annotation = Annotation { kind, range: range.clone(), message };

                    if is_header {
                        invalid_data.push(InvalidMetadata::InvalidAnnotation(
                            annotation,
                            format!(
                                "Error: header may not contain annotations of type {}",
                                kind
                            ),
                        ));
                        continue;
                    }

                    if matches!(
                        kind,
                        AnnotationKind::AutocompleteContains
                            | AnnotationKind::AutocompleteExcludes
                    ) {
                        if let Some(range) = range {
                            if range.start != range.end {
                                invalid_data.push(InvalidMetadata::InvalidAnnotation(
                                    annotation,
                                    "Error: found range in Autocomplete annotation where range.start != range.end, range.end would be ignored."
                                        .to_string()
                                    ));
                                continue;
                            }
                        } else {
                            invalid_data.push(InvalidMetadata::InvalidAnnotation(
                                annotation,
                                "Error: autocomplete annotation but no range specified"
                                    .to_string(),
                            ));
                            continue;
                        }
                    }
                    annotations.insert(annotation);
                }
                _ => (),
            }
        }
    }
    if invalid_data.is_empty() {
        Ok(TestPartMetadata {
            part_configuration: TestConfiguration {
                compare_ref,
                validate_hints,
                validate_autocomplete,
            },
            annotations,
        })
    } else {
        Err(invalid_data)
    }
}

fn validate_set_annotation(
    value: &str,
    flag: &mut Option<bool>,
    invalid_data: &mut Vec<InvalidMetadata>,
) {
    let value = value.trim();
    if value != "false" && value != "true" {
        invalid_data.push(
            InvalidMetadata::InvalidSet(format!("Error: trying to set Ref, Hints, or Autocomplete with value {value:?} != true, != false.")))
    } else {
        *flag = Some(value == "true")
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

/// returns key value for any metadata like line
/// Metadata lines are in the form
/// `// {key}[ {key}]?: {msg}`
/// We eat up to two words
pub fn get_metadata<'a>(line: &'a str) -> Option<(&'a str, &'a str)> {
    let mut s = Scanner::new(line);
    let metadata = |s: &mut Scanner<'a>| -> Option<(&'a str, &'a str)> {
        if !s.eat_if("//") {
            return None;
        }
        if s.eat_if('/') {
            return None;
        }

        let key = s.eat_until(':');
        if key.split_ascii_whitespace().count() > 2 {
            return None;
        }
        if !s.eat_if(':') {
            return None;
        }
        let value = s.eat_until('\n');
        Some((key, value))
    };
    metadata(&mut s)
}
