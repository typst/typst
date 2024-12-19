// Test the `box` and `block` containers.

--- box ---
// Test box in paragraph.
A #box[B \ C] D.

// Test box with height.
Spaced \
#box(height: 0.5cm) \
Apart

--- block-sizing ---
// Test block sizing.
#set page(height: 120pt)
#set block(spacing: 0pt)
#block(width: 90pt, height: 80pt, fill: red)[
  #block(width: 60%, height: 60%, fill: green)
  #block(width: 50%, height: 60%, fill: blue)
]

--- box-fr-width ---
// Test fr box.
Hello #box(width: 1fr, rect(height: 0.7em, width: 100%)) World

--- block-fr-height ---
#set page(height: 100pt)
#rect(height: 10pt, width: 100%)
#align(center, block(height: 1fr, width: 20pt, stroke: 1pt))
#rect(height: 10pt, width: 100%)

--- block-fr-height-auto-width ---
// Test that the fr block can also expand its parent.
#set page(height: 100pt)
#set align(center)
#block(inset: 5pt, stroke: green)[
  #rect(height: 10pt)
  #block(height: 1fr, stroke: 1pt, inset: 5pt)[
    #set align(center + horizon)
    I am the widest
  ]
  #rect(height: 10pt)
]

--- block-fr-height-first-child ---
// Test that block spacing is not trimmed if only an fr block precedes it.
#set page(height: 100pt)
#rect(height: 1fr)
#rect()

--- block-fr-height-multiple ---
#set page(height: 100pt)
#rect(height: 1fr)
#rect()
#block(height: 1fr, line(length: 100%, angle: 90deg))

--- block-negative-height-flow ---
#set page(height: 60pt)
a
#block(height: -25pt)[b]
c

--- block-multiple-pages ---
// Test block over multiple pages.
#set page(height: 60pt)

First!

#block[
  But, soft! what light through yonder window breaks? It is the east, and Juliet
  is the sun.
]

--- block-box-fill ---
#set page(height: 100pt)
#let words = lorem(18).split()
#block(inset: 8pt, width: 100%, fill: aqua, stroke: aqua.darken(30%))[
  #words.slice(0, 13).join(" ")
  #box(fill: teal, outset: 2pt)[tempor]
  #words.slice(13).join(" ")
]

--- block-spacing-basic ---
#set par(spacing: 10pt)
Hello

There

#block(spacing: 20pt)[Further down]

--- block-above-below-context ---
#context test(block.above, auto)
#set block(spacing: 20pt)
#context test(block.above, 20pt)
#context test(block.below, 20pt)

--- block-spacing-context ---
// The values for `above` and `below` might be different, so we cannot retrieve
// `spacing` directly
//
// Error: 16-23 function `block` does not contain field `spacing`
#context block.spacing

--- block-spacing-table ---
// Test that paragraph spacing loses against block spacing.
#set block(spacing: 100pt)
#show table: set block(above: 5pt, below: 5pt)
Hello
#table(columns: 4, fill: (x, y) => if calc.odd(x + y) { silver })[A][B][C][D]

--- block-spacing-maximum ---
// While we're at it, test the larger block spacing wins.
#set block(spacing: 0pt)
#show raw: set block(spacing: 15pt)
#show list: set block(spacing: 2.5pt)

```rust
fn main() {}
```

- List

Paragraph

--- block-spacing-collapse-text-style ---
// Test spacing collapsing with different font sizes.
#grid(columns: 2)[
  #text(size: 12pt, block(below: 1em)[A])
  #text(size: 8pt, block(above: 1em)[B])
][
  #text(size: 12pt, block(below: 1em)[A])
  #text(size: 8pt, block(above: 1.25em)[B])
]

--- block-fixed-height ---
#set page(height: 100pt)
#set align(center)

#lines(3)
#block(width: 80%, height: 60pt, fill: aqua)
#lines(2)
#block(
  breakable: false,
  width: 100%,
  inset: 4pt,
  fill: aqua,
  lines(3) + colbreak(),
)

--- block-consistent-width ---
// Test that block enforces consistent width across regions. Also use some
// introspection to check that measurement is working correctly.
#block(stroke: 1pt, inset: 5pt)[
  #align(right)[Hi]
  #colbreak()
  Hello @netwok
]

#show bibliography: none
#bibliography("/assets/bib/works.bib")

--- block-sticky ---
#set page(height: 100pt)
#lines(3)
#block(sticky: true)[D]
#block(sticky: true)[E]
F

--- block-sticky-alone ---
#set page(height: 50pt)
#block(sticky: true)[A]

--- block-sticky-many ---
#set page(height: 80pt)
#set block(sticky: true)
#block[A]
#block[B]
#block[C]
#block[D]
E
#block[F]
#block[G]

--- block-sticky-colbreak ---
A
#block(sticky: true)[B]
#colbreak()
C

--- block-sticky-breakable ---
// Ensure that sticky blocks are still breakable.
#set page(height: 60pt)
#block(sticky: true, lines(4))
E

--- box-clip-rect ---
// Test box clipping with a rectangle
Hello #box(width: 1em, height: 1em, clip: false)[#rect(width: 3em, height: 3em, fill: red)]
world 1

Space

Hello #box(width: 1em, height: 1em, clip: true)[#rect(width: 3em, height: 3em, fill: red)]
world 2

--- block-clip-text ---
// Test clipping text
#block(width: 5em, height: 2em, clip: false, stroke: 1pt + black)[
  But, soft! what light through
]

#v(2em)

#block(width: 5em, height: 2em, clip: true, stroke: 1pt + black)[
  But, soft! what light through yonder window breaks? It is the east, and Juliet
  is the sun.
]

--- block-clip-svg-glyphs ---
// Test clipping svg glyphs
Emoji: #box(height: 0.5em, stroke: 1pt + black)[🐪, 🌋, 🏞]

Emoji: #box(height: 0.5em, clip: true, stroke: 1pt + black)[🐪, 🌋, 🏞]

--- block-clipping-multiple-pages ---
// Test block clipping over multiple pages.
#set page(height: 60pt)

First!

#block(height: 4em, clip: true, stroke: 1pt + black)[
  But, soft! what light through yonder window breaks? It is the east, and Juliet
  is the sun.
]

--- box-clip-radius ---
// Test clipping with `radius`.
#set page(height: 60pt)

#box(
  radius: 5pt,
  stroke: 2pt + black,
  width: 20pt,
  height: 20pt,
  clip: true,
  image("/assets/images/rhino.png", width: 30pt)
)

--- box-clip-radius-without-stroke ---
// Test clipping with `radius`, but without `stroke`.
#set page(height: 60pt)

#box(
  radius: 5pt,
  width: 20pt,
  height: 20pt,
  clip: true,
  image("/assets/images/rhino.png", width: 30pt)
)

--- box-clip-outset ---
// Test clipping with `outset`.
#set page(height: 60pt)

#box(
  outset: 5pt,
  stroke: 2pt + black,
  width: 20pt,
  height: 20pt,
  clip: true,
  image("/assets/images/rhino.png", width: 30pt)
)

--- container-layoutable-child ---
// Test box/block sizing with directly layoutable child.
//
// Ensure that the output respects the box size.
#let check(f) = f(
  width: 40pt, height: 25pt, fill: aqua,
  grid(rect(width: 5pt, height: 5pt, fill: blue)),
)

#stack(dir: ltr, spacing: 1fr, check(box), check(block))

--- issue-2128-block-width-box ---
// Test box in 100% width block.
#block(width: 100%, fill: red, box("a box"))
#block(width: 100%, fill: red, [#box("a box") #box()])

--- issue-5296-block-sticky-in-block-at-top ---
#set page(height: 3cm)
#v(1.6cm)
#block(height: 2cm, breakable: true)[
  #block(sticky: true)[*A*]

  b
]

--- issue-5296-block-sticky-spaced-from-top-of-page ---
#set page(height: 3cm)
#v(2cm)

#block(sticky: true)[*A*]

b

--- issue-5296-block-sticky-weakly-spaced-from-top-of-page ---
#set page(height: 3cm)
#v(2cm, weak: true)

#block(sticky: true)[*A*]

b

--- issue-5262-block-negative-height ---
#block(height: -1pt)[]

--- issue-5262-block-negative-height-implicit ---
#set page(height: 10pt, margin: (top: 9pt))
#block(height: 100%)[]
