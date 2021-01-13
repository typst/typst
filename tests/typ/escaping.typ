// Test basic symbol escapes.

// Escapable
\\ \/ \[ \] \{ \} \* \_ \# \~ \` \$

// No need to escape.
( ) = ;

// Unescapable.
\a \: \; \( \)

// Escaped comments.
\//
\/\* \*\/
\/* \*/

---
// Test unicode escapes.
//
// error: 5:1-5:11 invalid unicode escape sequence
// error: 8:6-8:6 expected closing brace

\u{1F3D5} == 🏕

// Bad sequence.
\u{FFFFFF}

// Missing closing brace.
\u{41*Bold*

// Escaped escape sequence.
\\u\{ABC\}
