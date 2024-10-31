// Test figures.

--- figure-basic ---
#set page(width: 150pt)
#set figure(numbering: "I")

We can clearly see that @fig-cylinder and
@tab-complex are relevant in this context.

#figure(
  table(columns: 2)[a][b],
  caption: [The basic table.],
) <tab-basic>

#figure(
  pad(y: -6pt, image("/assets/images/cylinder.svg", height: 2cm)),
  caption: [The basic shapes.],
  numbering: "I",
) <fig-cylinder>

#figure(
  table(columns: 3)[a][b][c][d][e][f],
  caption: [The complex table.],
) <tab-complex>

--- figure-align ---
#show figure: set align(start)
#figure(
  rect[This is \ left],
  caption: [Start-aligned]
)

--- figure-table ---
// Testing figures with tables.
#figure(
  table(
    columns: 2,
    [Second cylinder],
    image("/assets/images/cylinder.svg"),
  ),
  caption: "A table containing images."
) <fig-image-in-table>

--- figure-placement ---
#set page(height: 160pt, columns: 2)
#set place(clearance: 10pt)

#lines(4)

#figure(
  placement: auto,
  scope: "parent",
  caption: [I],
  rect(height: 15pt, width: 80%),
)

#figure(
  placement: bottom,
  caption: [II],
  rect(height: 15pt, width: 80%),
)

#lines(2)

#figure(
  placement: bottom,
  caption: [III],
  rect(height: 25pt, width: 80%),
)

#figure(
  placement: auto,
  scope: "parent",
  caption: [IV],
  rect(width: 80%),
)

#lines(15)

--- figure-scope-without-placement ---
// Error: 2-27 parent-scoped placement is only available for floating figures
// Hint: 2-27 you can enable floating placement with `figure(placement: auto, ..)`
#figure(scope: "parent")[]

--- figure-theorem ---
// Testing show rules with figures with a simple theorem display
#show figure.where(kind: "theorem"): it => {
  set align(start)
  let name = none
  if not it.caption == none {
    name = [ #emph(it.caption.body)]
  } else {
    name = []
  }

  let title = none
  if not it.numbering == none {
    title = it.supplement
    if not it.numbering == none {
      title += " " +  it.counter.display(it.numbering)
    }
  }
  title = strong(title)
  pad(
    top: 0em, bottom: 0em,
    block(
      fill: green.lighten(90%),
      stroke: 1pt + green,
      inset: 10pt,
      width: 100%,
      radius: 5pt,
      breakable: false,
      [#title#name#h(0.1em):#h(0.2em)#it.body#v(0.5em)]
    )
  )
}

#set page(width: 150pt)
#figure(
  $a^2 + b^2 = c^2$,
  supplement: "Theorem",
  kind: "theorem",
  caption: "Pythagoras' theorem.",
  numbering: "1",
) <fig-formula>

#figure(
  $a^2 + b^2 = c^2$,
  supplement: "Theorem",
  kind: "theorem",
  caption: "Another Pythagoras' theorem.",
  numbering: none,
) <fig-formula>

#figure(
  ```rust
  fn main() {
    println!("Hello!");
  }
  ```,
  caption: [Hello world in _rust_],
)

--- figure-breakable ---
// Test breakable figures
#set page(height: 6em)
#show figure: set block(breakable: true)

#figure(table[a][b][c][d][e], caption: [A table])

--- figure-caption-separator ---
// Test custom separator for figure caption
#set figure.caption(separator: [ --- ])

#figure(
  table(columns: 2)[a][b],
  caption: [The table with custom separator.],
)

--- figure-caption-show ---
// Test figure.caption element
#show figure.caption: emph

#figure(
  [Not italicized],
  caption: [Italicized],
)

--- figure-caption-where-selector ---
// Test figure.caption element for specific figure kinds
#show figure.caption.where(kind: table): underline

#figure(
  [Not a table],
  caption: [Not underlined],
)

#figure(
  table[A table],
  caption: [Underlined],
)

--- figure-and-caption-show ---
// Test creating custom figure and custom caption

#let gap = 0.7em
#show figure.where(kind: "custom"): it => rect(inset: gap, {
  align(center, it.body)
  v(gap, weak: true)
  line(length: 100%)
  v(gap, weak: true)
  align(center, it.caption)
})

#figure(
  [A figure],
  kind: "custom",
  caption: [Hi],
  supplement: [A],
)

#show figure.caption: it => emph[
  #it.body
  (#it.supplement
   #context it.counter.display(it.numbering))
]

#figure(
  [Another figure],
  kind: "custom",
  caption: [Hi],
  supplement: [B],
)

--- figure-caption-position ---
#set figure.caption(position: top)

--- figure-caption-position-bad ---
// Error: 31-38 expected `top` or `bottom`, found horizon
#set figure.caption(position: horizon)

--- figure-localization-fr ---
// Test French
#set text(lang: "fr")
#figure(
  circle(),
  caption: [Un cercle.],
)

--- figure-localization-zh ---
// Test Chinese
#set text(lang: "zh")
#figure(
  rect(),
  caption: [一个矩形],
)

--- figure-localization-ru ---
// Test Russian
#set text(lang: "ru")

#figure(
    polygon.regular(size: 1cm, vertices: 8),
    caption: [Пятиугольник],
)

--- figure-localization-gr ---
// Test Greek
#set text(lang: "gr")
#figure(
  circle(),
  caption: [Ένας κύκλος.],
)

--- issue-2165-figure-caption-panic ---
#figure.caption[]

--- issue-2328-figure-entry-panic ---
// Error: 4-43 footnote entry must have a location
// Hint: 4-43 try using a query or a show rule to customize the footnote instead
HI#footnote.entry(clearance: 2.5em)[There]

--- issue-2530-figure-caption-panic ---
#figure(caption: [test])[].caption

--- issue-3586-figure-caption-separator ---
// Test that figure caption separator is synthesized correctly.
#show figure.caption: c => test(c.separator, [#": "])
#figure(table[], caption: [This is a test caption])

--- issue-4966-figure-float-counter ---
#let c = context counter(figure.where(kind: image)).display()
#set align(center)

#c

#figure(
  square(c),
  placement: bottom,
  caption: [A]
)

#c

#figure(
  circle(c),
  placement: top,
  caption: [B]
)

#c
