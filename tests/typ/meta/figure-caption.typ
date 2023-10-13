// Test figure captions.

---
// Test figure.caption element
#show figure.caption: emph

#figure(
  [Not italicized],
  caption: [Italicized],
)

---
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

---
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
   #it.counter.display(it.numbering))
]

#figure(
  [Another figure],
  kind: "custom",
  caption: [Hi],
  supplement: [B],
)
