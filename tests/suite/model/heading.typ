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

--- heading-trailing-whitespace ---
// Whether headings contain trailing whitespace with or without comments/labels.
// Labels are special cased to immediately end headings in the parser, but also
// #strike[have unique whitespace behavior] Now their behavior is consistent!

#let join(..xs) = xs.pos().join()
#let head(h) = heading(depth: 1, h)

// No whitespace.
#test(head[h], [= h])
#test(head[h], [= h/**/])
#test(head[h], [= h<a>])
#test(head[h], [= h/**/<b>])

// #strike[Label behaves differently than normal trailing space and comment.]
// Now they behave the same!
#test(join(head[h])[ ], [= h  ])
#test(join(head[h])[ ], [= h  /**/])
#test(join(head[h])[ ], [= h  <c>])

// Combinations.
#test(join(head[h])[ ][ ], [= h  /**/  ])
#test(join(head[h])[ ][ ], [= h  <d>  ])
#test(join(head[h])[ ], [= h  /**/<e>])
#test(join(head[h])[ ], [= h/**/  <f>])

// #strike[The first space attaches, but not the second] Now neither attaches!
#test(join(head(join[h]))[ ][ ], [= h  /**/  <g>])

--- heading-leading-whitespace ---
// Test that leading whitespace and comments don't matter.
#test[= h][=        h]
#test[= h][=   /**/  /**/   h]
#test[= h][=   /*
comment spans lines
*/   h]

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

--- heading-hanging-indent-auto ---
#set heading(numbering: "1.1.a.")
= State of the Art

--- heading-hanging-indent-zero ---
#set heading(numbering: "1.1.a.", hanging-indent: 0pt)
= State of the Art

--- heading-hanging-indent-length ---
#set heading(numbering: "1.1.a.", hanging-indent: 2em)
= State of the Art In Multi-Line

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

--- heading-html-basic html ---
// level 1 => h2
// ...
// level 5 => h6
// level 6 => div with role=heading and aria-level=7
//  ...

= Level 1
== Level 2
=== Level 3
==== Level 4
===== Level 5
// Warning: 1-15 heading of level 6 was transformed to <div role="heading" aria-level="7">, which is not supported by all assistive technology
// Hint: 1-15 HTML only supports <h1> to <h6>, not <h7>
// Hint: 1-15 you may want to restructure your document so that it doesn't contain deep headings
====== Level 6
// Warning: 1-16 heading of level 7 was transformed to <div role="heading" aria-level="8">, which is not supported by all assistive technology
// Hint: 1-16 HTML only supports <h1> to <h6>, not <h8>
// Hint: 1-16 you may want to restructure your document so that it doesn't contain deep headings
======= Level 7
