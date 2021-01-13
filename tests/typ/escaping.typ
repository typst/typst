// Escapable symbols.
\\ \/ \[ \] \{ \} \* \_ \# \~ \` \$

// No need to escape.
( ) = ;

// Unescapable.
\a \: \; \( \)

// Escaped comments.
\//
\/\* \*\/
\/* \*/ *

// Test unicode escape sequence.
\u{1F3D5} == 🏕

// Escaped escape sequence.
\u{41} vs. \\u\{41\}

// Error: 1:1-1:11 invalid unicode escape sequence
\u{FFFFFF}

// Error: 1:6-1:6 expected closing brace
\u{41*Bold*
