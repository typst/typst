// Test footnotes in containers.

---
// Test footnote in caption.
Read the docs #footnote[https://typst.app/docs]!
#figure(
  image("/files/graph.png", width: 70%),
  caption: [
    A graph #footnote[A _graph_ is a structure with nodes and edges.]
  ]
)
More #footnote[just for ...] footnotes #footnote[... testing. :)]

---
// Test duplicate footnotes.
#let lang = footnote[Languages.]
#let nums = footnote[Numbers.]

/ "Hello": A word #lang
/ "123": A number #nums

- "Hello" #lang
- "123" #nums

+ "Hello" #lang
+ "123" #nums

#table(
  columns: 2,
  [Hello], [A word #lang],
  [123], [A number #nums],
)
