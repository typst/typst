--- disable-tags-artifact pdftags ---
#set pdf(standard: "ua-1")
= Heading 1
#pdf.artifact[
  #table(
    columns: 2,
    [a], [b],
    [c], [d],
  )
]

= Heading 2

--- disable-tags-tiling pdftags ---
#set pdf(standard: "ua-1")
= Rectangle

#let pat = tiling(size: (20pt, 20pt))[
  - a
  - b
    - c
]
#rect(fill: pat)

--- disable-tags-hide pdftags ---
#set pdf(standard: "ua-1")
= Hidden

#hide[
  - a
  - b
    - c
]

--- disable-tags-partially-hidden-list pdftags ---
#set pdf(standard: "ua-1")
// FIXME(accessibility): In realization, tags inside of list groupings aren't
// handled. Thus if the head of the list is visible, all tags of list items
// will be emitted before (outside) the hide element. And if the head is not
// visible, all tags of list items will be emitted inside the hide element.
= Tail hidden
- a
#hide[
- b
  - c
]

= Head hidden
#hide[
- a
]
- b
  - c

--- disable-tags-broken-paragraph-hide pdftags ---
#set pdf(standard: "ua-1")
This is the #hide[ first paragraph.

And this is the ] second paragraph.

--- disable-tags-broken-paragraph-artifact pdftags ---
#set pdf(standard: "ua-1")
This is the #pdf.artifact[ first paragraph.

And this is the ] second paragraph.

--- background-artifact-1-7 pdf ---
#set pdf(standard: "1.7")
#set page(fill: eastern)
#rect()
#pdf.artifact(kind: "background", circle())

--- background-artifact-2-0 pdf ---
#set pdf(standard: "2.0")
#set page(fill: eastern)
#rect()
#pdf.artifact(kind: "background", circle())
