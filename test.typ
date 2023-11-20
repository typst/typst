#set page(width: 140pt, height: 140pt)

#rect(
  width: 100pt,
  height: 100pt,
  fill: pattern((5pt, 5pt))[
    #square(size: 5pt, fill: pattern((1pt, 1pt))[
      #square(size: 1pt, fill: gradient.linear(..color.map.viridis, space: rgb))
    ])
  ]
)