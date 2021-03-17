// Test headings.

---
// Different number of hashtags.

// Valid levels.
=1
===2
======6

// Too many hashtags.
// Warning: 1-8 should not exceed depth 6
=======7

---
// Heading continuation over linebreak.

// Code blocks continue heading.
= A{
    "B"
}

// Function call continues heading.
= #rect[
    A
] B

// Without some kind of block, headings end at a line break.
= A
B

---
// Heading vs. no heading.

// Parsed as headings if at start of the context.
/**/ = Ok
{[== Ok]}
#rect[=== Ok]

// Not at the start of the context.
No = heading

// Escaped.
\= No heading
