// Test escape sequences.

---
// Escapable symbols.
\\ \/ \[ \] \{ \} \# \* \_ \= \~ \` \$

// No need to escape.
( ) ; < >

// Unescapable.
\a \: \; \( \)

// Escaped comments.
\//
\/\* \*\/
\/* \*/ *

// Unicode escape sequence.
\u{1F3D5} == 🏕

// Escaped escape sequence.
\u{41} vs. \\u\{41\}

// Some code stuff in text.
let f() , ; : | + - /= == 12 "string"

---
// Unicode codepoint does not exist.
// Error: 1-11 invalid unicode escape sequence
\u{FFFFFF}

---
// Unterminated.
// Error: 6 expected closing brace
\u{41[*Bold*]
