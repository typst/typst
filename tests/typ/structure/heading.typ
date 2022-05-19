// Test headings.

---
#show node: heading as text(blue, node.body)

=
No heading

---
// Different number of equals signs.

= Level 1
== Level 2
=== Level 3

// After three, it stops shrinking.
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
#show it: heading as it.body
#show it: strong as it.body + [!]

===== Heading 🌍
#heading(level: 5)[Heading]
