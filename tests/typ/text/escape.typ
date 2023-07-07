// Test escape sequences.

---
// Escapable symbols.
\\ \/ \[ \] \{ \} \# \* \_ \+ \= \~ \
\` \$ \" \' \< \> \@ \( \) \A

// No need to escape.
( ) ;

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

// Escaped dot.
10\. May

---
// Unicode codepoint does not exist.
// Error: 1-11 invalid Unicode codepoint: FFFFFF
\u{FFFFFF}

---
// Unterminated.
// Error: 1-6 unclosed Unicode escape sequence
\u{41[*Bold*]
