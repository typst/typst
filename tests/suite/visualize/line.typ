// Test lines.

--- line-basic render ---
#set page(height: 60pt)
#box({
  set line(stroke: 0.75pt)
  place(line(end: (0.4em, 0pt)))
  place(line(start: (0pt, 0.4em), end: (0pt, 0pt)))
  line(end: (0.6em, 0.6em))
}) Hello #box(line(length: 1cm))!

#line(end: (70%, 50%))

--- line-positioning render ---
// Test the angle argument and positioning.

#set page(fill: rgb("0B1026"))
#set line(stroke: white)

#let star(size, ..args) = box(width: size, height: size)[
  #set text(spacing: 0%)
  #set line(..args)
  #set align(left)
  #v(30%)
  #place(line(length: +30%, start: (09.0%, 02%)))
  #place(line(length: +30%, start: (38.7%, 02%), angle: -72deg))
  #place(line(length: +30%, start: (57.5%, 02%), angle: 252deg))
  #place(line(length: +30%, start: (57.3%, 02%)))
  #place(line(length: -30%, start: (88.0%, 02%), angle: -36deg))
  #place(line(length: +30%, start: (73.3%, 48%), angle: 252deg))
  #place(line(length: -30%, start: (73.5%, 48%), angle: 36deg))
  #place(line(length: +30%, start: (25.4%, 48%), angle: -36deg))
  #place(line(length: +30%, start: (25.6%, 48%), angle: -72deg))
  #place(line(length: +32%, start: (8.50%, 02%), angle: 34deg))
]

#align(center, grid(
  columns: 3,
  column-gutter: 10pt,
  ..((star(20pt, stroke: 0.5pt),) * 9)
))

--- line-stroke render ---
// Some simple test lines
#line(length: 60pt, stroke: red)
#v(3pt)
#line(length: 60pt, stroke: 2pt)
#v(3pt)
#line(length: 60pt, stroke: blue + 1.5pt)
#v(3pt)
#line(length: 60pt, stroke: (paint: red, thickness: 1pt, dash: "dashed"))
#v(3pt)
#line(length: 60pt, stroke: (paint: red, thickness: 4pt, cap: "round"))

--- line-stroke-set render ---
// Set rules with stroke
#set line(stroke: (paint: red, thickness: 1pt, cap: "butt", dash: "dash-dotted"))
#line(length: 60pt)
#v(3pt)
#line(length: 60pt, stroke: blue)
#v(3pt)
#line(length: 60pt, stroke: (dash: none))

--- line-stroke-dash render ---
// Dashing
#line(length: 60pt, stroke: (paint: red, thickness: 1pt, dash: ("dot", 1pt)))
#v(3pt)
#line(length: 60pt, stroke: (paint: red, thickness: 1pt, dash: ("dot", 1pt, 4pt, 2pt)))
#v(3pt)
#line(length: 60pt, stroke: (paint: red, thickness: 1pt, dash: (array: ("dot", 1pt, 4pt, 2pt), phase: 5pt)))
#v(3pt)
#line(length: 60pt, stroke: (paint: red, thickness: 1pt, dash: ()))
#v(3pt)
#line(length: 60pt, stroke: (paint: red, thickness: 1pt, dash: (1pt, 3pt, 9pt)))

--- line-stroke-field-typo render ---
// Error: 29-56 unexpected key "thicknes", valid keys are "paint", "thickness", "cap", "join", "dash", and "miter-limit"
#line(length: 60pt, stroke: (paint: red, thicknes: 1pt))

--- line-stroke-bad-dash-kind render ---
// Error: 29-55 expected "solid", "dotted", "densely-dotted", "loosely-dotted", "dashed", "densely-dashed", "loosely-dashed", "dash-dotted", "densely-dash-dotted", "loosely-dash-dotted", array, dictionary, none, or auto
#line(length: 60pt, stroke: (paint: red, dash: "dash"))

--- line-bad-point-array render ---
// Test errors.

// Error: 12-19 array must contain exactly two items
// Hint: 12-19 the first item determines the value for the X axis and the second item the value for the Y axis
#line(end: (50pt,))

--- line-bad-point-component-type render ---
// Error: 14-26 expected relative length, found angle
#line(start: (3deg, 10pt), length: 5cm)

--- line-infinite-length render ---
// Error: 2-54 cannot create line with infinite length
#line(start: (0pt, 0pt), end: (float.inf * 1pt, 0pt))
