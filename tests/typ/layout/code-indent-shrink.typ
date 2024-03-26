// Leading spaces in raw blocks should not be shrunken
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
// Leading spaces in normal paragraphs should still
// be shrunken
Reference line

~~Two leading nbsp

~~~~Reference

~~~~More~non~breaking~spaces