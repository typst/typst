// Test headings.

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
#[== Level 2]
#box[=== Level 3]

// Not at the start of the context.
No = heading

// Escaped.
\= No heading

---
// Blocks can continue the heading.

= #[This
is
multiline.
]

= This
  is not.

---
// Test styling.
#show heading.where(level: 5): it => block(
  text(font: "Roboto", fill: eastern, it.body + [!])
)

= Heading
===== Heading 🌍
#heading(level: 5)[Heading]

---
// Test setting the starting offset.
#set heading(numbering: "1.1")
#show heading.where(level: 2): set text(blue)
= Level 1

#heading(depth: 1)[We're twins]
#heading(level: 1)[We're twins]

== Real level 2

#set heading(offset: 1)
= Fake level 2
== Fake level 3

---
// Passing level directly still overrides all other set values
#set heading(numbering: "1.1", offset: 1)
#heading(level: 1)[Still level 1]

---
// Edge cases.
#set heading(numbering: "1.")
=
Not in heading
=Nope
