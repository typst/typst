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
