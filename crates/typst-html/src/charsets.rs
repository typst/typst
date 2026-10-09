//! Defines syntactical properties of HTML tags, attributes, and text.

/// Check whether a character is in a tag name.
pub const fn is_valid_in_tag_name(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '-'
}

/// Check whether a character is valid in an attribute name.
///
/// See <https://html.spec.whatwg.org/multipage/syntax.html#syntax-attribute-name>
pub const fn is_valid_in_attribute_name(c: char) -> bool {
    match c {
        // These are forbidden.
        '\0' | ' ' | '"' | '\'' | '>' | '/' | '=' => false,
        c if is_whatwg_control_char(c) => false,
        c if is_whatwg_non_char(c) => false,
        // _Everything_ else is allowed, including U+2029 paragraph
        // separator. Go wild.
        _ => true,
    }
}

/// Check whether a character is valid in an attribute local name.
///
/// See <https://dom.spec.whatwg.org/#valid-attribute-local-name>
pub const fn is_valid_in_attribute_local_name(c: char) -> bool {
    match c {
        '\0' | '/' | '=' | '>' => false,
        c if c.is_ascii_whitespace() => false,
        _ => true,
    }
}

/// Check whether a character can be an used in an attribute value without
/// escaping.
///
/// See <https://html.spec.whatwg.org/multipage/syntax.html#syntax-attribute-value>
pub const fn is_valid_in_attribute_value(c: char) -> bool {
    match c {
        // Ampersands are sometimes legal (i.e. when they are not _ambiguous
        // ampersands_) but it is not worth the trouble to check for that.
        '&' => false,
        // Quotation marks are not allowed in double-quote-delimited attribute
        // values.
        '"' => false,
        // All other text characters are allowed.
        c => is_w3c_text_char(c),
    }
}

/// Check whether a character can be an used in normal text without
/// escaping.
pub const fn is_valid_in_normal_element_text(c: char) -> bool {
    match c {
        // Ampersands are sometimes legal (i.e. when they are not _ambiguous
        // ampersands_) but it is not worth the trouble to check for that.
        '&' => false,
        // Less-than signs are not allowed in text.
        '<' => false,
        // All other text characters are allowed.
        c => is_w3c_text_char(c),
    }
}

/// Check if something is valid text in HTML.
pub const fn is_w3c_text_char(c: char) -> bool {
    match c {
        // Non-characters are obviously not text characters.
        c if is_whatwg_non_char(c) => false,
        // Control characters are disallowed, except for whitespace.
        c if is_whatwg_control_char(c) => c.is_ascii_whitespace(),
        // Everything else is allowed.
        _ => true,
    }
}

/// See <https://infra.spec.whatwg.org/#noncharacter>
const fn is_whatwg_non_char(c: char) -> bool {
    match c {
        '\u{fdd0}'..='\u{fdef}' => true,
        // Non-characters matching xxFFFE or xxFFFF up to x10FFFF (inclusive).
        c if c as u32 & 0xfffe == 0xfffe && c as u32 <= 0x10ffff => true,
        _ => false,
    }
}

/// See <https://infra.spec.whatwg.org/#control>
const fn is_whatwg_control_char(c: char) -> bool {
    match c {
        // C0 control characters.
        '\u{00}'..='\u{1f}' => true,
        // Other control characters.
        '\u{7f}'..='\u{9f}' => true,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_attr_name() {
        let assert_valid = |c: char, x: bool, y: bool| {
            let expected = (x, y);
            let got =
                (is_valid_in_attribute_name(c), is_valid_in_attribute_local_name(c));
            assert_eq!(
                got, expected,
                "char: {c:?}, expected: ({expected:?}), got: ({got:?})",
            );
        };

        assert_valid('a', true, true);
        assert_valid('-', true, true);
        assert_valid('*', true, true);

        assert_valid('\0', false, false);
        assert_valid('=', false, false);
        assert_valid('\t', false, false);

        assert_valid('"', false, true);
        assert_valid('\'', false, true);
    }
}
