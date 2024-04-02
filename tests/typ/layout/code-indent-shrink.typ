// Spaces in raw blocks should not be shrunk
// as it would mess up the indentation of code
// https://github.com/typst/typst/issues/3191

---
#set par(justify: true)

#show raw.where(block: true): block.with(
  fill: luma(240),
  inset: 10pt,
)

#block(
  width: 60%,
  ```py
  for x in xs:
      print("x=",x)
  ```
)

---
// In normal paragraphs, spaces should still be shrunk.
// The first line here serves as a reference, while the second
// uses non-breaking spaces to create an overflowing line
// (which should shrink).
~~~~No shrinking here

~~~~The~spaces~on~this~line~shrink