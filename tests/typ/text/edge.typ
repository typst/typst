// Test top and bottom text edge.

---
#set page(width: 160pt)
#set text(size: 8pt)

#let try(top, bottom) = rect(fill: conifer)[
  #set text("IBM Plex Mono", top-edge: top, bottom-edge: bottom)
  From #top to #bottom
]

#try("ascender", "descender")
#try("ascender", "baseline")
#try("cap-height", "baseline")
#try("x-height", "baseline")
#try(4pt, -2pt)
#try(1pt + 0.3em, -0.15em)

---
// Error: 21-23 expected string or length, found array
#set text(top-edge: ())

---
// Error: 24-26 unknown font metric
#set text(bottom-edge: "")
