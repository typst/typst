// Test different numbers of hashtags.

// Valid levels.
# One
### Three
###### Six

// Too many hashtags.
// Warning: 1:1-1:8 section depth should not exceed 6
####### Seven

---
// Test heading vs. no heading.

// Parsed as headings if at start of the context.
/**/ # Heading
{[## Heading]}
[box][### Heading]

// Not at the start of the context.
Text with # hashtag

// Escaped.
\# No heading

// Keyword.
// Error: 1:1-1:6 unexpected invalid token
#nope

// Not parsed as a keyword, but neither as a heading.
Nr#1

---
// Heading continuation over linebreak.

// Code blocks continue heading.
# This {
    "continues"
}

// Function call continues heading.
# [box][
    This,
] too

// Without some kind of block, headings end at a line break.
# This
not
