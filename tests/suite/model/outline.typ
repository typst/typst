--- outline-spacing ---
#set heading(numbering: "1.a.")
#set outline.entry(fill: none)
#show outline.entry.where(level: 1): set block(above: 1.2em)

#outline()

#show heading: none
= A
== B
== C
= D
== E

--- outline-indent-auto ---
#set heading(numbering: "I.i.")
#set page(width: 150pt)
#show heading: none

#context test(outline.indent, auto)
#outline()

= A
== B
== C
== D
=== Title that breaks across lines
= E
== F
=== Aligned

--- outline-indent-auto-mixed-prefix ---
#show heading: none
#show outline.entry.where(level: 1): strong

#outline()

#set heading(numbering: "I.i.")
= A
== B
=== Title that breaks
= C
== D
= E
#[
  #set heading(numbering: none)
  = F
  == Numberless title that breaks
  === G
]
= H

--- outline-indent-auto-mixed-prefix-short ---
#show heading: none

#outline()

#set heading(numbering: "I.i.")
= A
#set heading(numbering: none)
= B

--- outline-indent-auto-no-prefix ---
#show heading: none

#outline()

= A
== B
=== Title that breaks across lines
= C
== D
=== E

--- outline-indent-zero ---
#set heading(numbering: "1.a.")
#show heading: none

#outline(indent: 0pt)

= A
== B
=== C
==== Title that breaks across lines
#set heading(numbering: none)
== E
= F

--- outline-indent-fixed ---
#set heading(numbering: "1.a.")
#show heading: none

#outline(indent: 1em)

= A
== B
=== C
==== Title that breaks
#set heading(numbering: none)
== E
= F

--- outline-indent-func ---
#set heading(numbering: "1.a.")
#show heading: none

#outline(indent: n => (0pt, 1em, 2.5em, 3em).at(n))

= A
== B
=== C
==== Title breaks
#set heading(numbering: none)
== E
= F

--- outline-indent-bad-type ---
// Error: 2-35 expected relative length, found dictionary
#outline(indent: n => (a: "dict"))

= Heading

--- outline-entry ---
#set page(width: 150pt)
#set heading(numbering: "1.")

#show outline.entry.where(level: 1): set block(above: 12pt)
#show outline.entry.where(level: 1): strong

#outline(indent: auto)

#show heading: none
= Introduction
= Background
== History
== State of the Art
= Analysis
== Setup

--- outline-entry-complex ---
#set page(width: 150pt, numbering: "I", margin: (bottom: 20pt))
#set heading(numbering: "1.")

#set outline.entry(fill: repeat[--])
#show outline.entry.where(level: 1): it => link(
  it.element.location(),
  it.indented(it.prefix(), {
    emph(it.body())
    [ ]
    text(luma(100), box(width: 1fr, repeat[--·--]))
    [ ]
    it.page()
  })
)

#counter(page).update(3)
#outline()

#show heading: none

= Top heading
== Not top heading
=== Lower heading
=== Lower too
== Also not top

#pagebreak()
#set page(numbering: "1")

= Another top heading
== Middle heading
=== Lower heading

--- outline-entry-inner ---
#set heading(numbering: "1.")
#show outline.entry: it => block(it.inner())
#show heading: none

#set outline.entry(fill: repeat[ -- ])
#outline()

= A
= B

--- outline-heading-start-of-page ---
#set page(width: 140pt, height: 200pt, margin: (bottom: 20pt), numbering: "1")
#set heading(numbering: "(1/a)")
#show heading.where(level: 1): set text(12pt)
#show heading.where(level: 2): set text(10pt)

#set outline.entry(fill: none)
#outline()

= A
= B
#lines(3)

// This heading is right at the start of the page, so that we can test
// whether the tag migrates properly.
#[
  #set heading(outlined: false)
  == C
]

A

== D
== F
==== G

--- outline-bookmark ---
// Ensure that `bookmarked` option doesn't affect the outline
#set heading(numbering: "(I)", bookmarked: false)
#set outline.entry(fill: none)
#show heading: none
#outline()

= A

--- outline-styled-text ---
#outline(title: none)

= #text(blue)[He]llo

--- outline-first-line-indent ---
#set par(first-line-indent: 1.5em)
#set heading(numbering: "1.1.a.")
#show outline.entry.where(level: 1): strong

#outline()

#show heading: none
= Introduction
= Background
== History
== State of the Art
= Analysis
== Setup

--- outline-bad-element ---
// Error: 2-27 cannot outline metadata
#outline(target: metadata)
#metadata("hello")


--- issue-2048-outline-multiline ---
// Without the word joiner between the dots and the page number,
// the page number would be alone in its line.
#set page(width: 125pt)
#set heading(numbering: "1.a.")
#show heading: none

#outline()

= A
== This just fits here

--- issue-2530-outline-entry-panic-text ---
// Outline entry (pre-emptive)
// Error: 2-27 cannot outline text
#outline.entry(1, [Hello])

--- issue-2530-outline-entry-panic-heading ---
// Outline entry (pre-emptive, improved error)
// Error: 2-34 heading must have a location
// Hint: 2-34 try using a show rule to customize the outline.entry instead
#outline.entry(1, heading[Hello])

--- issue-4476-outline-rtl-title-ending-in-ltr-text ---
#set text(lang: "he")
#outline()

#show heading: none
= הוקוס Pocus
= זוהי כותרת שתורגמה על ידי מחשב

--- issue-4859-outline-entry-show-set ---
#set heading(numbering: "1.a.")
#show outline.entry.where(level: 1): set outline.entry(fill: none)
#show heading: none

#outline()

= A
== B

--- issue-5176-outline-cjk-title ---
#set text(font: "Noto Serif CJK SC")
#show heading: none

#outline(title: none)

= 测
= 很
