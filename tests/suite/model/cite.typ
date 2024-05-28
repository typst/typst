--- cite-footnote ---
Hello @netwok
And again: @netwok

#pagebreak()
#bibliography("/assets/bib/works.bib", style: "chicago-notes")

--- cite-form ---
#set page(width: 200pt)

Nothing: #cite(<arrgh>, form: none)

#cite(<netwok>, form: "prose") say stuff.

#bibliography("/assets/bib/works.bib", style: "apa")

--- cite-group ---
A#[@netwok@arrgh]B \
A@netwok@arrgh B \
A@netwok @arrgh B \
A@netwok @arrgh. B \

A @netwok#[@arrgh]B \
A @netwok@arrgh, B \
A @netwok @arrgh, B \
A @netwok @arrgh. B \

A#[@netwok @arrgh @quark]B. \
A @netwok @arrgh @quark B. \
A @netwok @arrgh @quark, B.

#set text(0pt)
#bibliography("/assets/bib/works.bib")

--- cite-grouping-and-ordering ---
@mcintosh_anxiety
@psychology25
@netwok
@issue201
@arrgh
@quark
@distress,
@glacier-melt
@issue201
@tolkien54
@sharing
@restful

#show bibliography: none
#bibliography("/assets/bib/works.bib")

--- issue-785-cite-locate ---
// Test citation in other introspection.
#set page(width: 180pt)
#set heading(numbering: "1")

#outline(
  title: [List of Figures],
  target: figure.where(kind: image),
)

#pagebreak()

= Introduction <intro>
#figure(
  rect[-- PIRATE --],
  caption: [A pirate @arrgh in @intro],
)

#context [Citation @distress on page #here().page()]

#pagebreak()
#bibliography("/assets/bib/works.bib", style: "chicago-notes")

--- issue-1597-cite-footnote ---
// Tests that when a citation footnote is pushed to next page, things still
// work as expected.
#set page(height: 60pt)
#lorem(4)

#footnote[@netwok]
#show bibliography: none
#bibliography("/assets/bib/works.bib")

--- issue-2531-cite-show-set ---
// Test show set rules on citations.
#show cite: set text(red)
A @netwok @arrgh.
B #cite(<netwok>) #cite(<arrgh>).

#show bibliography: none
#bibliography("/assets/bib/works.bib")

--- issue-3481-cite-location ---
// The locator was cloned in the wrong location, leading to inconsistent
// citation group locations in the second footnote attempt.
#set page(height: 60pt)

// First page shouldn't be empty because otherwise we won't skip the first
// region which causes the bug in the first place.
#v(10pt)

// Everything moves to the second page because we want to keep the line and
// its footnotes together.
#footnote[@netwok]
#footnote[A]

#show bibliography: none
#bibliography("/assets/bib/works.bib")

--- issue-3699-cite-twice-et-al ---
// Citing a second time showed all authors instead of "et al".
@mcintosh_anxiety \
@mcintosh_anxiety
#show bibliography: none
#bibliography("/assets/bib/works.bib", style: "chicago-author-date")
