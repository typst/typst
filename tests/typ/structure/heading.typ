// Test headings.

---
// Different number of hashtags.

// Valid levels.
= Level 1
=== Level 2
====== Level 6

// At some point, it should stop shrinking.
=========== Level 11

---
// Heading vs. no heading.

// Parsed as headings if at start of the context.
/**/ = Level 1
{[== Level 2]}
#box[=== Level 3]

// Not at the start of the context.
No = heading

// Escaped.
\= No heading

---
// Blocks can continue the heading.

= [This
is
multiline.
]

= This
  is not.

---
// Test styling.
= Heading

#set heading(family: "Roboto", fill: eastern)

===== Heading 🌍
#heading(level: 5)[Heading]
