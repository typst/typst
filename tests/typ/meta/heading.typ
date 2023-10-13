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
// Edge cases.
#set heading(numbering: "1.")
=
Not in heading
=Nope
