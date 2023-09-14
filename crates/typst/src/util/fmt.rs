/// Format pieces separated with commas and a final "and" or "or".
pub fn separated_list(pieces: &[impl AsRef<str>], last: &str) -> String {
    let mut buf = String::new();
    for (i, part) in pieces.iter().enumerate() {
        match i {
            0 => {}
            1 if pieces.len() == 2 => {
                buf.push(' ');
                buf.push_str(last);
                buf.push(' ');
            }
            i if i + 1 == pieces.len() => {
                buf.push_str(", ");
                buf.push_str(last);
                buf.push(' ');
            }
            _ => buf.push_str(", "),
        }
        buf.push_str(part.as_ref());
    }
    buf
}

/// Format a comma-separated list.
///
/// Tries to format horizontally, but falls back to vertical formatting if the
/// pieces are too long.
pub fn pretty_comma_list(pieces: &[impl AsRef<str>], trailing_comma: bool) -> String {
    const MAX_WIDTH: usize = 50;

    let mut buf = String::new();
    let len = pieces.iter().map(|s| s.as_ref().len()).sum::<usize>()
        + 2 * pieces.len().saturating_sub(1);

    if len <= MAX_WIDTH {
        for (i, piece) in pieces.iter().enumerate() {
            if i > 0 {
                buf.push_str(", ");
            }
            buf.push_str(piece.as_ref());
        }
        if trailing_comma {
            buf.push(',');
        }
    } else {
        for piece in pieces {
            buf.push_str(piece.as_ref().trim());
            buf.push_str(",\n");
        }
    }

    buf
}

/// Format an array-like construct.
///
/// Tries to format horizontally, but falls back to vertical formatting if the
/// pieces are too long.
pub fn pretty_array_like(parts: &[impl AsRef<str>], trailing_comma: bool) -> String {
    let list = pretty_comma_list(parts, trailing_comma);
    let mut buf = String::new();
    buf.push('(');
    if list.contains('\n') {
        buf.push('\n');
        for (i, line) in list.lines().enumerate() {
            if i > 0 {
                buf.push('\n');
            }
            buf.push_str("  ");
            buf.push_str(line);
        }
        buf.push('\n');
    } else {
        buf.push_str(&list);
    }
    buf.push(')');
    buf
}
