// Test headings.

--- heading-basic ---
// Different number of equals signs.

= Level 1
== Level 2
=== Level 3

// After three, it stops shrinking.
=========== Level 11

--- heading-syntax-at-start ---
// Heading vs. no heading.

// Parsed as headings if at start of the context.
/**/ = Level 1
#[== Level 2]
#box[=== Level 3]

// Not at the start of the context.
No = heading

// Escaped.
\= No heading

--- heading-block ---
// Blocks can continue the heading.

= #[This
is
multiline.
]

= This
  is not.

--- heading-show-where ---
// Test styling.
#show heading.where(level: 5): it => block(
  text(font: "Roboto", fill: eastern, it.body + [!])
)

= Heading
===== Heading 🌍
#heading(level: 5)[Heading]

--- heading-offset ---
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

--- heading-offset-and-level ---
// Passing level directly still overrides all other set values
#set heading(numbering: "1.1", offset: 1)
#heading(level: 1)[Still level 1]

--- heading-syntax-edge-cases ---
// Edge cases.
#set heading(numbering: "1.")
=
Not in heading
=Nope

--- heading-numbering-hint ---
= Heading <intro>

// Error: 1:19-1:25 cannot reference heading without numbering
// Hint: 1:19-1:25 you can enable heading numbering with `#set heading(numbering: "1.")`
Cannot be used as @intro
