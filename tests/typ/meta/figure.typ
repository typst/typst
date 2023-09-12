// Test figures.

---
#set page(width: 150pt)
#set figure(numbering: "I")

We can clearly see that @fig-cylinder and
@tab-complex are relevant in this context.

#figure(
  table(columns: 2)[a][b],
  caption: [The basic table.],
) <tab-basic>

#figure(
  pad(y: -6pt, image("/files/cylinder.svg", height: 2cm)),
  caption: [The basic shapes.],
  numbering: "I",
) <fig-cylinder>

#figure(
  table(columns: 3)[a][b][c][d][e][f],
  caption: [The complex table.],
) <tab-complex>

---

// Testing figures with tables.
#figure(
  table(
    columns: 2,
    [Second cylinder],
    image("/files/cylinder.svg"),
  ),
  caption: "A table containing images."
) <fig-image-in-table>

---

// Testing show rules with figures with a simple theorem display
#show figure.where(kind: "theorem"): it => {
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

---
// Test breakable figures
#set page(height: 6em)
#show figure: set block(breakable: true)

#figure(table[a][b][c][d][e], caption: [A table])

---
// Test custom separator for figure caption
#set figure.caption(separator: [ --- ])

#figure(
  table(columns: 2)[a][b],
  caption: [The table with custom separator.],
)
