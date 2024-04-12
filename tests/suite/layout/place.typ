// Test the `place` function.

--- place-basic ---
#set page("a8")
#place(bottom + center)[© Typst]

= Placement
#place(right, image("/assets/images/tiger.jpg", width: 1.8cm))
Hi there. This is \
a placed element. \
Unfortunately, \
the line breaks still had to be inserted manually.

#stack(
  rect(fill: eastern, height: 10pt, width: 100%),
  place(right, dy: 1.5pt)[ABC],
  rect(fill: conifer, height: 10pt, width: 80%),
  rect(fill: forest, height: 10pt, width: 100%),
  10pt,
  block[
    #place(center, dx: -7pt, dy: -5pt)[Hello]
    #place(center, dx: 7pt, dy: 5pt)[Hello]
    Hello #h(1fr) Hello
  ]
)

--- place-block-spacing ---
// Test how the placed element interacts with paragraph spacing around it.
#set page("a8", height: 60pt)

First

#place(bottom + right)[Placed]

Second

--- place-background ---
#set page(paper: "a10", flipped: true)
#set text(fill: white)
#place(
  dx: -10pt,
  dy: -10pt,
  image(
    "/assets/images/tiger.jpg",
    fit: "cover",
    width: 100% + 20pt,
    height: 100% + 20pt,
  )
)
#align(bottom + right)[
  _Welcome to_ #underline[*Tigerland*]
]

--- place-float ---
#set page(height: 140pt)
#set place(clearance: 5pt)
#lorem(6)
#place(auto, float: true, rect[A])
#place(auto, float: true, rect[B])
#place(auto, float: true, rect[C])
#place(auto, float: true, rect[D])

--- place-float-missing ---
// Error: 2-20 automatic positioning is only available for floating placement
// Hint: 2-20 you can enable floating placement with `place(float: true, ..)`
#place(auto)[Hello]

--- place-float-center-horizon ---
// Error: 2-45 floating placement must be `auto`, `top`, or `bottom`
#place(center + horizon, float: true)[Hello]

--- place-float-horizon ---
// Error: 2-36 floating placement must be `auto`, `top`, or `bottom`
#place(horizon, float: true)[Hello]

--- place-float-default ---
// Error: 2-27 floating placement must be `auto`, `top`, or `bottom`
#place(float: true)[Hello]

--- place-float-right ---
// Error: 2-34 floating placement must be `auto`, `top`, or `bottom`
#place(right, float: true)[Hello]

--- place-float-columns ---
// LARGE
#set page(height: 200pt, width: 300pt)
#show: columns.with(2)

= Introduction
#figure(
  placement: bottom,
  caption: [A glacier],
  image("/assets/images/glacier.jpg", width: 50%),
)
#lorem(45)
#figure(
  placement: top,
  caption: [A rectangle],
  rect[Hello!],
)
#lorem(20)

--- place-float-figure ---
// LARGE
#set page(height: 250pt, width: 150pt)

= Introduction
#lorem(10) #footnote[Lots of Latin]

#figure(
  placement: bottom,
  caption: [A glacier #footnote[Lots of Ice]],
  image("/assets/images/glacier.jpg", width: 80%),
)

#lorem(40)

#figure(
  placement: top,
  caption: [An important],
  image("/assets/images/diagram.svg", width: 80%),
)

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

--- issue-2595-float-overlap ---
#set page(height: 80pt)

Start.

#place(auto, float: true, [
  #block(height: 100%, width: 100%, fill: aqua)
])

#place(auto, float: true, [
  #block(height: 100%, width: 100%, fill: red)
])

#lorem(20)

--- issue-2715-float-order ---
#set page(height: 180pt)
#set figure(placement: auto)

#figure(
  rect(height: 60pt),
  caption: [Rectangle I],
)

#figure(
  rect(height: 50pt),
  caption: [Rectangle II],
)

#figure(
  circle(),
  caption: [Circle],
)

#lorem(20)
