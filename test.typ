#set page(
  width: 140pt,
  height: 140pt,
  fill: pattern(
      (30pt, 30pt),
      relative: "parent",
      square(size: 30pt, fill: gradient.conic(..color.map.rainbow))
    )
)

#rotate(45deg, scale(x: 50%, y: 70%, rect(
  width: 100%,
  height: 100%,
  stroke: 1pt,
)[
  #set text(
    fill: pattern(
      (30pt, 30pt),
      relative: "parent",
      square(size: 30pt, fill: gradient.conic(..color.map.rainbow))
    )
  )

  #lorem(10)
]))