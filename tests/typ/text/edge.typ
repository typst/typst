// Test top and bottom text edge.

---
#set page(width: 160pt)
#set text(size: 8pt)

#let try(top, bottom) = rect(inset: 0pt, fill: conifer)[
  #set text(font: "IBM Plex Mono", top-edge: top, bottom-edge: bottom)
  From #top to #bottom
]

#let try-bounds(top, bottom) = rect(inset: 0pt, fill: conifer)[
  #set text(font: "IBM Plex Mono", top-edge: top, bottom-edge: bottom)
  #top to #bottom: "yay, Typst"
]

#try("ascender", "descender")
#try("ascender", "baseline")
#try("cap-height", "baseline")
#try("x-height", "baseline")
#try-bounds("cap-height", "baseline")
#try-bounds("bounds", "baseline")
#try-bounds("bounds", "bounds")
#try-bounds("x-height", "bounds")

#try(4pt, -2pt)
#try(1pt + 0.3em, -0.15em)

---
// Error: 21-23 expected "ascender", "cap-height", "x-height", "baseline", "bounds", or length, found array
#set text(top-edge: ())

---
// Error: 24-26 expected "baseline", "descender", "bounds", or length
#set text(bottom-edge: "")

---
// Error: 24-36 expected "baseline", "descender", "bounds", or length
#set text(bottom-edge: "cap-height")
