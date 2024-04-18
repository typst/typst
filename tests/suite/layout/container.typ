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

--- box-width-fr ---
// Test fr box.
Hello #box(width: 1fr, rect(height: 0.7em, width: 100%)) World

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
#set block(spacing: 10pt)
Hello

There

#block(spacing: 20pt)[Further down]

--- block-spacing-table ---
// Test that paragraph spacing loses against block spacing.
// TODO
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

#lorem(10)
#block(width: 80%, height: 60pt, fill: aqua)
#lorem(6)
#block(
  breakable: false,
  width: 100%,
  inset: 4pt,
  fill: aqua,
  lorem(8) + colbreak(),
)

--- box-clip-rect ---
// Test box clipping with a rectangle
Hello #box(width: 1em, height: 1em, clip: false)[#rect(width: 3em, height: 3em, fill: red)]
world 1

Space

Hello #box(width: 1em, height: 1em, clip: true)[#rect(width: 3em, height: 3em, fill: red)]
world 2

--- block-clip-text ---
// Test cliping text
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
