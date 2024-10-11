// Test the `place` function.

--- place-basic ---
#set page("a8")
#place(bottom + center)[E]

= A
#place(right, rect(width: 1.8cm))
#lines(5)

#stack(
  rect(fill: eastern, height: 10pt, width: 100%),
  place(right, dy: 1.5pt)[ABC],
  rect(fill: conifer, height: 10pt, width: 80%),
  rect(fill: forest, height: 10pt, width: 100%),
  10pt,
  block[
    #place(center, dx: -7pt, dy: -5pt)[A]
    #place(center, dx: 7pt, dy: 5pt)[B]
    C #h(1fr) D
  ]
)

--- place-block-spacing ---
// Test how the placed element interacts with paragraph spacing around it.
#set page("a8", height: 60pt)

First

#place(bottom + right)[Placed]

Second

--- place-bottom-in-box ---
#box(
  fill: aqua,
  width: 30pt,
  height: 30pt,
  place(bottom,
    place(line(start: (0pt, 0pt), end: (20pt, 0pt), stroke: red + 3pt))
  )
)

--- place-horizon-in-boxes ---
#box(
  fill: aqua,
  width: 30pt,
  height: 30pt,
  {
    box(fill: yellow, {
      [Hello]
      place(horizon, line(start: (0pt, 0pt), end: (20pt, 0pt), stroke: red + 2pt))
    })
    place(horizon, line(start: (0pt, 0pt), end: (20pt, 0pt), stroke: green + 3pt))
  }
)

--- place-bottom-right-in-box ---
#box(fill: aqua)[
  #place(bottom + right)[Hi]
  Hello World \
  How are \
  you?
]

--- place-top-left-in-box ---
#box(fill: aqua)[
  #place(top + left, dx: 50%, dy: 50%)[Hi]
  #v(30pt)
  #line(length: 50pt)
]

--- place-float-flow-around ---
#set page(height: 80pt)
#set place(float: true)
#place(bottom + center, rect(height: 20pt))
#lines(4)

--- place-float-queued ---
#set page(height: 180pt)
#set figure(placement: auto)

#figure(rect(height: 60pt), caption: [I])
#figure(rect(height: 40pt), caption: [II])
#figure(rect(), caption: [III])
A
#figure(rect(), caption: [IV])

--- place-float-align-auto ---
#set page(height: 140pt)
#set place(auto, float: true, clearance: 5pt)

#place(rect[A])
#place(rect[B])
1 \ 2
#place(rect[C])
#place(rect[D])

--- place-float-delta ---
#place(top + center, float: true, dx: 10pt, rect[I])
A
#place(bottom + center, float: true, dx: -10pt, rect[II])

--- place-float-flow-size ---
#set page(width: auto, height: auto)
#set place(float: true, clearance: 5pt)

#place(bottom, rect(width: 80pt, height: 10pt))
#place(top + center, rect(height: 20pt))
#align(center)[A]
#pagebreak()
#align(center)[B]
#place(bottom, scope: "parent", rect(height: 10pt))

--- place-float-flow-size-alone ---
#set page(width: auto, height: auto)
#set place(float: true, clearance: 5pt)
#place(auto)[A]

--- place-float-fr ---
#set page(height: 120pt, columns: 2)
#set place(float: true, clearance: 10pt)
#set rect(width: 70%)

#place(top + center, rect[I])
#place(bottom + center, scope: "parent", rect[II])

A
#v(1fr)
B
#colbreak()
C
#align(bottom)[D]

--- place-float-rel-sizing ---
#set page(height: 100pt, columns: 2)
#set place(float: true, clearance: 10pt)
#set rect(width: 70%)

#place(top + center, scope: "parent", rect[I])
#place(top + center, rect[II])

// This test result is not ideal: The first column takes 30% of the full page,
// while the second takes 30% of the remaining space since there is no concept
// of `full` for followup pages.
#set align(bottom)
#rect(width: 100%, height: 30%)
#rect(width: 100%, height: 30%)

--- place-float-block-backlog ---
#set page(height: 100pt)
#v(60pt)
#place(top, float: true, rect())
#list(.."ABCDEFGHIJ".clusters())

--- place-float-clearance-empty ---
// Check that we don't require space for clearance if there is no content.
#set page(height: 100pt)
#v(1fr)
#table(
  columns: (1fr, 1fr),
  lines(2),
  [],
  lines(8),
  place(auto, float: true, block(width: 100%, height: 100%, fill: aqua))
)

--- place-float-column-align-auto ---
#set page(height: 150pt, columns: 2)
#set place(auto, float: true, clearance: 10pt)
#set rect(width: 75%)

#place(rect[I])
#place(rect[II])
#place(rect[III])
#place(rect[IV])

#lines(6)

#place(rect[V])
#place(rect[VI])

--- place-float-column-queued ---
#set page(height: 100pt, columns: 2)
#set place(float: true, clearance: 10pt)
#set rect(width: 75%)
#set text(costs: (widow: 0%, orphan: 0%))

#lines(3)

#place(top, rect[I])
#place(top, rect[II])
#place(bottom, rect[III])

#lines(3)

--- place-float-twocolumn ---
#set page(height: 100pt, columns: 2)
#set place(float: true, clearance: 10pt)
#set rect(width: 70%)

#place(top + center, scope: "parent", rect[I])
#place(top + center, rect[II])
#lines(4)
#place(top + center, rect[III])
#block(width: 100%, height: 70pt, fill: conifer)
#place(bottom + center, scope: "parent", rect[IV])
#place(bottom + center, rect[V])
#v(1pt, weak: true)
#block(width: 100%, height: 60pt, fill: aqua)

--- place-float-twocolumn-queued ---
#set page(height: 100pt, columns: 2)
#set place(float: true, scope: "parent", clearance: 10pt)
#let t(align, fill) = place(top + align, rect(fill: fill, height: 25pt))

#t(left, aqua)
#t(center, forest)
#t(right, conifer)
#lines(7)

--- place-float-twocolumn-align-auto ---
#set page(height: 100pt, columns: 2)
#set place(float: true, clearance: 10pt)
#set rect(width: 70%)

#place(auto, scope: "parent", rect[I]) // Should end up `top`
#lines(4)
#place(auto, scope: "parent", rect[II])  // Should end up `bottom`
#lines(4)

--- place-float-twocolumn-fits ---
#set page(height: 100pt, columns: 2)
#set place(float: true, clearance: 10pt)
#set rect(width: 70%)

#lines(6)
#place(auto, scope: "parent", rect[I])
#lines(12, "1")

--- place-float-twocolumn-fits-not ---
#set page(height: 100pt, columns: 2)
#set place(float: true, clearance: 10pt)
#set rect(width: 70%)

#lines(10)
#place(auto, scope: "parent", rect[I])
#lines(10, "1")

--- place-float-threecolumn ---
#set page(height: 100pt, columns: 3)
#set place(float: true, clearance: 10pt)
#set rect(width: 70%)

#place(bottom + center, scope: "parent", rect[I])
#lines(21)
#place(top + center, scope: "parent", rect[II])

--- place-float-threecolumn-block-backlog ---
#set page(height: 100pt, columns: 3)
#set place(float: true, clearance: 10pt)
#set rect(width: 70%)

// The most important part of this test is that we get the backlog of the
// conifer (green) block right.
#place(top + center, scope: "parent", rect[I])
#block(fill: aqua, width: 100%, height: 70pt)
#block(fill: conifer, width: 100%, height: 160pt)
#place(bottom + center, scope: "parent", rect[II])
#place(top, rect(height: 40%)[III])
#block(fill: yellow, width: 100%, height: 60pt)

--- place-float-counter ---
#let c = counter("c")
#let cd = context c.display()

#set page(
  height: 100pt,
  margin: (y: 20pt),
  header: [H: #cd],
  footer: [F: #cd],
  columns: 2,
)

#let t(align, scope: "column", n) = place(
  align,
  float: true,
  scope: scope,
  clearance: 10pt,
  line(length: 100%) + c.update(n),
)

#t(bottom, 6)
#cd
#t(top, 3)
#colbreak()
#cd
#t(scope: "parent", bottom, 11)
#colbreak()
#cd
#t(top, 12)

--- place-float-missing ---
// Error: 2-20 automatic positioning is only available for floating placement
// Hint: 2-20 you can enable floating placement with `place(float: true, ..)`
#place(auto)[Hello]

--- place-float-center-horizon ---
// Error: 2-45 vertical floating placement must be `auto`, `top`, or `bottom`
#place(center + horizon, float: true)[Hello]

--- place-float-horizon ---
// Error: 2-36 vertical floating placement must be `auto`, `top`, or `bottom`
#place(horizon, float: true)[Hello]

--- place-float-default ---
// Error: 2-27 vertical floating placement must be `auto`, `top`, or `bottom`
#place(float: true)[Hello]

--- place-float-right ---
// Error: 2-34 vertical floating placement must be `auto`, `top`, or `bottom`
#place(right, float: true)[Hello]

--- place-flush ---
#set page(height: 120pt)
#let floater(align, height) = place(
  align,
  float: true,
  rect(width: 100%, height: height),
)

#floater(top, 30pt)
A

#floater(bottom, 50pt)
#place.flush()
B // Should be on the second page.

--- place-flush-figure ---
#set page(height: 120pt)
#let floater(align, height, caption) = figure(
  placement: align,
  caption: caption,
  rect(width: 100%, height: height),
)

#floater(top, 30pt)[I]
A

#floater(bottom, 50pt)[II]
#place.flush()
B // Should be on the second page.

--- issue-place-base ---
// Test that placement is relative to container and not itself.
#set page(height: 80pt, margin: 0pt)
#place(right, dx: -70%, dy: 20%, [First])
#place(left, dx: 20%, dy: 60%, [Second])
#place(center + horizon, dx: 25%, dy: 25%, [Third])

--- issue-1368-place-pagebreak ---
// Test placing on an already full page.
// It shouldn't result in a page break.
#set page(height: 40pt)
#block(height: 100%)
#place(bottom + right)[Hello world]

--- issue-2199-place-spacing-bottom ---
// Test that placed elements don't add extra block spacing.
#show figure: set block(spacing: 4em)

Paragraph before float.
#figure(rect(), placement: bottom)
Paragraph after float.

--- issue-2199-place-spacing-default ---
#show place: set block(spacing: 4em)

Paragraph before place.
#place(rect())
Paragraph after place.

--- issue-2595-float-overlap ---
#set page(height: 80pt)

1
#place(auto, float: true, block(height: 100%, width: 100%, fill: aqua))
#place(auto, float: true, block(height: 100%, width: 100%, fill: red))
#lines(7)
