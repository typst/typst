#set page(width: auto, height: auto)
#set par(leading: 4pt, justify: true)
#set text(font: "New Computer Modern")

#let story = [
  In olden times when wishing still helped one, there lived a king whose
  daughters were all beautiful; and the youngest was so beautiful that the sun
  itself, which has seen so much, was astonished whenever it shone in her face.
  Close by the king’s castle lay a great dark forest, and under an old lime-tree
  in the forest was a well, and when the day was very warm, the king’s child
  went out into the forest and sat down by the side of the cool fountain; and
  when she was bored she took a golden ball, and threw it up on high and caught
  it; and this ball was her favorite plaything.
]

#let column(title, linebreaks, hyphenate) = {
  rect(inset: 0pt, width: 132pt, fill: rgb("eee"))[
    #set par(linebreaks: linebreaks)
    #set text(hyphenate: hyphenate)
    #strong(title) \ #story
  ]
}

#grid(
  columns: 3,
  gutter: 10pt,
  column([Simple without hyphens], "simple", false),
  column([Simple with hyphens], "simple", true),
  column([Optimized with hyphens], "optimized", true),
)
