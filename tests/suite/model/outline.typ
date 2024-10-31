--- outline ---
#set page(height: 200pt, margin: (bottom: 20pt), numbering: "1")
#set heading(numbering: "(1/a)")
#show heading.where(level: 1): set text(12pt)
#show heading.where(level: 2): set text(10pt)

#outline(fill: none)

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

--- outline-styled-text ---
#outline(title: none)

= #text(blue)[He]llo

--- outline-bookmark ---
#outline(title: none, fill: none)

// Ensure 'bookmarked' option doesn't affect the outline
#set heading(numbering: "(I)", bookmarked: false)
= A

--- outline-indent-numbering ---
// With heading numbering
#set page(width: 200pt)
#set heading(numbering: "1.a.")
#show heading: none
#set outline(fill: none)

#context test(outline.indent, none)
#outline(indent: false)
#outline(indent: true)
#outline(indent: none)
#outline(indent: auto)
#outline(indent: 2em)
#outline(indent: n => ([-], [], [==], [====]).at(n))

= A
== B
== C
=== D
==== E

--- outline-indent-no-numbering ---
// Without heading numbering
#set page(width: 200pt)
#show heading: none
#set outline(fill: none)

#outline(indent: false)
#outline(indent: true)
#outline(indent: none)
#outline(indent: auto)
#outline(indent: n => 2em * n)

= About
== History

--- outline-indent-bad-type ---
// Error: 2-35 expected relative length or content, found dictionary
#outline(indent: n => (a: "dict"))

= Heading

--- outline-first-line-indent ---
#set par(first-line-indent: 1.5em)
#set heading(numbering: "1.1.a.")
#show outline.entry.where(level: 1): it => {
  v(0.5em, weak: true)
  strong(it)
}

#outline()

= Introduction
= Background
== History
== State of the Art
= Analysis
== Setup

--- outline-entry ---
#set page(width: 150pt)
#set heading(numbering: "1.")

#show outline.entry.where(
  level: 1
): it => {
  v(12pt, weak: true)
  strong(it)
}

#outline(indent: auto)
#v(1.2em, weak: true)

#set text(8pt)
#show heading: set block(spacing: 0.65em)

= Introduction
= Background
== History
== State of the Art
= Analysis
== Setup

--- outline-entry-complex ---
#set page(width: 150pt, numbering: "I", margin: (bottom: 20pt))
#set heading(numbering: "1.")
#show outline.entry.where(level: 1): it => [
  #let loc = it.element.location()
  #let num = numbering(loc.page-numbering(), ..counter(page).at(loc))
  #emph(link(loc, it.body))
  #text(luma(100), box(width: 1fr, repeat[#it.fill.body;·]))
  #link(loc, num)
]

#counter(page).update(3)
#outline(indent: auto, fill: repeat[--])
#v(1.2em, weak: true)

#set text(8pt)
#show heading: set block(spacing: 0.65em)

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

--- outline-bad-element ---
// Error: 2-27 cannot outline metadata
#outline(target: metadata)
#metadata("hello")

--- issue-2530-outline-entry-panic-text ---
// Outline entry (pre-emptive)
// Error: 2-48 cannot outline text
#outline.entry(1, [Hello], [World!], none, [1])

--- issue-2530-outline-entry-panic-heading ---
// Outline entry (pre-emptive, improved error)
// Error: 2-55 heading must have a location
// Hint: 2-55 try using a query or a show rule to customize the outline.entry instead
#outline.entry(1, heading[Hello], [World!], none, [1])

--- issue-4476-rtl-title-ending-in-ltr-text ---
#set text(lang: "he")
#outline()

= הוקוס Pocus
= זוהי כותרת שתורגמה על ידי מחשב

--- issue-5176-cjk-title ---
#set text(font: "Noto Serif CJK SC")
#show heading: none

#outline(title: none)

= 测
= 很
