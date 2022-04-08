// Test lines

---
// Default line.
#line()

---
// Test the to argument.
{
    line(to: (10pt, 0pt))
    line(origin: (0pt, 10pt), to: (0pt, 0pt))
    line(to: (15pt, 15pt))
}
#v(.5cm)

---
// Test the angle argument and positioning.

#set page(fill: rgb("0B1026"))
#set line(stroke: white)

#let star(width, ..args) = box(width: width, height: width)[
  #set text(spacing: 0%)
  #set line(..args)

  #align(left)[
    #line(length: +30%, origin: (09.0%, 02%))
    #line(length: +30%, origin: (38.7%, 02%), angle: -72deg)
    #line(length: +30%, origin: (57.5%, 02%), angle: 252deg)
    #line(length: +30%, origin: (57.3%, 02%))
    #line(length: -30%, origin: (88.0%, 02%), angle: -36deg)
    #line(length: +30%, origin: (73.3%, 48%), angle: 252deg)
    #line(length: -30%, origin: (73.5%, 48%), angle: 36deg)
    #line(length: +30%, origin: (25.4%, 48%), angle: -36deg)
    #line(length: +30%, origin: (25.6%, 48%), angle: -72deg)
    #line(length: +32%, origin: (8.50%, 02%), angle: 34deg)
  ]
]

#align(center, grid(columns: (1fr,) * 3, ..((star(20pt, stroke: 0.5pt),) * 9)))

---
// Test errors.

// Error: 11-18 point array must contain exactly two entries
#line(to: (50pt,))

---
// Error: 15-27 expected relative length, found angle
#line(origin: (3deg, 10pt), length: 5cm)
