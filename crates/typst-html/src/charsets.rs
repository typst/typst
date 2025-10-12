//! Defines syntactical properties of HTML tags, attributes, and text.

/// Check whether a character is in a tag name.
pub const fn is_valid_in_tag_name(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '-'
}

/// Check whether a character is valid in an attribute name.
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

/// Check whether a character can be an used in an attribute value without
/// escaping.
///
/// See <https://html.spec.whatwg.org/multipage/syntax.html#attributes-2>
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

const fn is_whatwg_non_char(c: char) -> bool {
    match c {
        '\u{fdd0}'..='\u{fdef}' => true,
        // Non-characters matching xxFFFE or xxFFFF up to x10FFFF (inclusive).
        c if c as u32 & 0xfffe == 0xfffe && c as u32 <= 0x10ffff => true,
        _ => false,
    }
}

const fn is_whatwg_control_char(c: char) -> bool {
    match c {
        // C0 control characters.
        '\u{00}'..='\u{1f}' => true,
        // Other control characters.
        '\u{7f}'..='\u{9f}' => true,
        _ => false,
    }
}
