// Test lines

---
// Default line.
#line()

---
// Test the `end` argument.
{
    line(end: (10pt, 0pt))
    line(start: (0pt, 10pt), end: (0pt, 0pt))
    line(end: (15pt, 15pt))
}
#v(.5cm)

---
// Test the angle argument and positioning.

#set page(fill: rgb("0B1026"))
#set line(stroke: white)

#let star(width, ..args) = box(width: width, height: width)[
  #set text(spacing: 0%)
  #set line(..args)
  #set align(left)
  #line(length: +30%, start: (09.0%, 02%))
  #line(length: +30%, start: (38.7%, 02%), angle: -72deg)
  #line(length: +30%, start: (57.5%, 02%), angle: 252deg)
  #line(length: +30%, start: (57.3%, 02%))
  #line(length: -30%, start: (88.0%, 02%), angle: -36deg)
  #line(length: +30%, start: (73.3%, 48%), angle: 252deg)
  #line(length: -30%, start: (73.5%, 48%), angle: 36deg)
  #line(length: +30%, start: (25.4%, 48%), angle: -36deg)
  #line(length: +30%, start: (25.6%, 48%), angle: -72deg)
  #line(length: +32%, start: (8.50%, 02%), angle: 34deg)
]

#align(center, grid(
  columns: 3,
  column-gutter: 10pt,
  ..((star(20pt, stroke: 0.5pt),) * 9)
))

---
// Test errors.

// Error: 12-19 point array must contain exactly two entries
#line(end: (50pt,))

---
// Error: 14-26 expected relative length, found angle
#line(start: (3deg, 10pt), length: 5cm)
