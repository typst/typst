--- disable-tags-artifact pdftags ---
= Heading 1
#pdf.artifact[
  #table(
    columns: 2,
    [a], [b],
    [c], [d],
  )
]

= Heading 2

--- disable-tags-tiling pdftags ---
= Rectangle

#let pat = tiling(size: (20pt, 20pt))[
  - a
  - b
    - c
]
#rect(fill: pat)
