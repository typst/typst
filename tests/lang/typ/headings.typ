// Test different numbers of hashtags.

// Valid levels.
= One
=== Three
====== Six

// Too many hashtags.
// Warning: 1-8 should not exceed depth 6
======= Seven

---
// Test heading vs. no heading.

// Parsed as headings if at start of the context.
/**/ = Heading
{[== Heading]}
#[box][=== Heading]

// Not at the start of the context.
Text = with=sign

// Escaped.
\= No heading

---
// Heading continuation over linebreak.

// Code blocks continue heading.
= This {
    "continues"
}

// Function call continues heading.
= #[box][
    This,
] too

// Without some kind of block, headings end at a line break.
= This
not
