use std::fmt::{Display, Formatter};
use std::path::Path;

use ecow::EcoString;
use similar::{ChangeTag, InlineChange, TextDiff};

static DIFF_STYLE: &str = include_str!("diff.css");

pub struct TextFileDiff {
    name: EcoString,
    html: String,
}

pub fn diff_doc(mut diffs: Vec<TextFileDiff>) -> String {
    diffs.sort_by(|a, b| a.name.cmp(&b.name));

    let diff_content = typst_utils::display(|f| {
        for diff in diffs.iter() {
            f.write_str(&diff.html)?;
        }
        Ok(())
    });

    let diff_list = typst_utils::display(|f| {
        for test in diffs.iter() {
            let name = Escaped(test.name.as_str());
            writeln!(f, r##"<a href="#{name}" class="test-link">{name}</a>"##)?;
        }
        Ok(())
    });

    let style = Escaped(DIFF_STYLE);
    format!(
        "
<!DOCTYPE html>
<html>
    <head>
        <meta charset=\"utf-8\" />
        <title>Typst tests</title>
        <style>\n\
            {style}
        </style>
    </head>
<body>
    <div class=\"container\">
        <div class=\"sidebar-container\">
            <div class=\"sidebar\">\n\
                {diff_list}
            </div>
        </div>
        <div class=\"diff-container\">
            {diff_content}
        </div>
    </div>
</body>
</html>
"
    )
}

/// Create a rich HTML text diff.
pub fn text_diff(
    name: EcoString,
    (path_a, a): (&Path, &str),
    (path_b, b): (&Path, &str),
) -> TextFileDiff {
    let diff = TextDiff::from_lines(a, b);

    let mut left = Vec::new();
    let mut right = Vec::new();

    for (i, group) in diff.grouped_ops(3).iter().enumerate() {
        if i != 0 {
            left.push(line_gap());
            right.push(line_gap());
        }

        for op in group.iter() {
            for change in diff.iter_inline_changes(op) {
                match change.tag() {
                    ChangeTag::Equal => {
                        while left.len() < right.len() {
                            left.push(line_empty());
                        }
                        while right.len() < left.len() {
                            right.push(line_empty());
                        }

                        let left_line_nr = change.old_index().unwrap();
                        let line = display_line_text(&change, span_unchanged);
                        left.push(line_unchanged(left_line_nr, line));

                        let right_line_nr = change.new_index().unwrap();
                        let line = display_line_text(&change, span_unchanged);
                        right.push(line_unchanged(right_line_nr, line));
                    }
                    ChangeTag::Delete => {
                        let left_line_nr = change.old_index().unwrap();
                        let line = display_line_text(&change, span_del);
                        left.push(line_del(left_line_nr, line));
                    }
                    ChangeTag::Insert => {
                        let right_line_nr = change.new_index().unwrap();
                        let line = display_line_text(&change, span_add);
                        right.push(line_add(right_line_nr, line));
                    }
                }
            }
        }

        while left.len() < right.len() {
            left.push(line_empty());
        }
        while right.len() < left.len() {
            right.push(line_empty());
        }
    }
    left.push(line_end());
    right.push(line_end());

    let thead = typst_utils::display(|f| table_header(f, path_a, path_b));
    let rows = typst_utils::display(|f| {
        for (l, r) in left.iter().zip(right.iter()) {
            writeln!(
                f,
                "\
<tr class=\"diff-line\">
{l}
{r}
</tr>\
                "
            )?;
        }
        Ok(())
    });

    let html = format!(
        r#"<div class="file-diff" id="{name}">
        <input type="checkbox" class="collapse-diff"/>
    <table columns="4" class="diff-area">
        <colgroup>
            <col span="1" class="col-line-gutter">
            <col span="1" class="col-line-body">
            <col span="1" class="col-line-gutter">
            <col span="1" class="col-line-body">
        </colgroup>
{thead}
<tbody class="diff-body">
{rows}
</tbody>
<t>
    </table>
</div>
"#
    );
    TextFileDiff { name, html }
}

fn table_header(f: &mut Formatter, a: &Path, b: &Path) -> std::fmt::Result {
    let a = a.display();
    let b = b.display();
    writeln!(
        f,
        r#"<thead class="diff-header">
    <tr>
        <th colspan="2"><a href="../../{a}">{a}</a></th>
        <th colspan="2"><a href="../../{a}">{b}</a></th>
    </tr>
</thead>"#
    )
}

fn line_empty() -> String {
    diff_line("empty", "", "")
}

fn line_del(line_nr: usize, line: impl Display) -> String {
    diff_line("del", line_nr, line)
}

fn line_add(line_nr: usize, line: impl Display) -> String {
    diff_line("add", line_nr, line)
}

fn line_unchanged(line_nr: usize, line: impl Display) -> String {
    diff_line("unchanged", line_nr, line)
}

fn line_gap() -> String {
    "<td colspan=\"2\" class=\"diff-gap\">\u{22ef}</td>".into()
}

fn line_end() -> String {
    diff_line("end", "", "")
}

fn diff_line(kind: &str, line_nr: impl Display, line: impl Display) -> String {
    format!(
        "\
<td class=\"line-gutter diff-{kind}\">{line_nr}</td>
<td class=\"line-body diff-{kind}\"><pre class=\"line-text\"><code>{line}</code></pre></td>\
        "
    )
}

fn display_line_text<'a>(
    change: &InlineChange<str>,
    write_emph: fn(&mut Formatter, span: &str) -> std::fmt::Result,
) -> impl Display {
    typst_utils::display(move |f| {
        for (emph, span) in change.iter_strings_lossy() {
            let span = span.trim_end_matches('\n');
            if emph {
                write_emph(f, span.as_ref())?;
            } else {
                span_unchanged(f, span.as_ref())?;
            }
        }
        Ok(())
    })
}

fn span_unchanged(f: &mut Formatter, span: &str) -> std::fmt::Result {
    Display::fmt(&Escaped(span), f)
}

fn span_del(f: &mut Formatter, span: &str) -> std::fmt::Result {
    let span = Escaped(span);
    write!(f, r#"<span class="span-del">{span}</span>"#)
}

fn span_add(f: &mut Formatter, span: &str) -> std::fmt::Result {
    let span = Escaped(span);
    write!(f, r#"<span class="span-add">{span}</span>"#)
}

/// Escape text for inclusion in HTML.
struct Escaped<'a>(&'a str);

impl std::fmt::Display for Escaped<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut remainder = self.0;
        while let Some(i) = remainder.find(['<', '&', '>']) {
            f.write_str(&remainder[..i])?;
            let replacement = match remainder.as_bytes()[i] {
                b'<' => "&lt",
                b'&' => "&amp",
                b'>' => "&gt",
                _ => unreachable!(),
            };
            f.write_str(replacement)?;

            remainder = &remainder[i + 1..];
        }
        f.write_str(remainder)
    }
}
